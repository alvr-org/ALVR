use super::VideoDecoderConfig;
use alvr_common::{
    anyhow::{anyhow, bail, Context, Result},
    error, info,
    parking_lot::{Condvar, Mutex},
    warn, RelaxedAtomic, ToAny,
};
use alvr_session::{CodecType, MediacodecPropType};
use ndk::{
    hardware_buffer::HardwareBufferUsage,
    media::{
        image_reader::{AcquireResult, Image, ImageFormat, ImageReader},
        media_codec::{
            DequeuedInputBufferResult, DequeuedOutputBufferInfoResult, MediaCodec,
            MediaCodecDirection, MediaFormat,
        },
    },
};
use std::{
    collections::VecDeque,
    ffi::c_void,
    ops::Deref,
    ptr,
    sync::{Arc, Weak},
    thread::{self, JoinHandle},
    time::Duration,
};

struct FakeThreadSafe<T>(T);
unsafe impl<T> Send for FakeThreadSafe<T> {}
unsafe impl<T> Sync for FakeThreadSafe<T> {}

impl<T> Deref for FakeThreadSafe<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

type SharedMediaCodec = Arc<FakeThreadSafe<MediaCodec>>;

pub struct VideoDecoderSink {
    inner: Arc<Mutex<Option<SharedMediaCodec>>>,
}

unsafe impl Send for VideoDecoderSink {}

impl VideoDecoderSink {
    // Block until the buffer has been written or timeout is reached. Returns false if timeout.
    pub fn push_frame_nal(&mut self, timestamp: Duration, data: &[u8]) -> Result<bool> {
        let Some(decoder) = &*self.inner.lock() else {
            // This might happen only during destruction
            return Ok(false);
        };

        match decoder.dequeue_input_buffer(Duration::ZERO) {
            Ok(DequeuedInputBufferResult::Buffer(mut buffer)) => {
                unsafe {
                    ptr::copy_nonoverlapping(
                        data.as_ptr(),
                        buffer.buffer_mut().as_mut_ptr().cast(),
                        data.len(),
                    )
                };

                // NB: the function expects the timestamp in micros, but nanos is used to have
                // complete precision, so when converted back to Duration it can compare correctly
                // to other Durations
                decoder.queue_input_buffer(buffer, 0, data.len(), timestamp.as_nanos() as _, 0)?;

                Ok(true)
            }
            Ok(DequeuedInputBufferResult::TryAgainLater) => Ok(false),
            Err(e) => bail!("{e}"),
        }
    }
}

struct QueuedImage {
    timestamp: Duration,
    image: Image,
    in_use: bool,
}
unsafe impl Send for QueuedImage {}

// Access the image queue synchronously.
pub struct VideoDecoderSource {
    running: Arc<RelaxedAtomic>,
    dequeue_thread: Option<JoinHandle<()>>,
    image_queue: Arc<Mutex<VecDeque<QueuedImage>>>,
    config: VideoDecoderConfig,
    buffering_running_average: f32,
}

unsafe impl Send for VideoDecoderSource {}

impl VideoDecoderSource {
    // The application MUST finish using the returned buffer before calling this function again
    pub fn dequeue_frame(&mut self) -> Option<(Duration, *mut c_void)> {
        let mut image_queue_lock = self.image_queue.lock();

        if let Some(queued_image) = image_queue_lock.front() {
            if queued_image.in_use {
                // image is released and ready to be reused by the decoder
                image_queue_lock.pop_front();
            }
        }

        // use running average to give more weight to recent samples
        self.buffering_running_average = self.buffering_running_average
            * self.config.buffering_history_weight
            + image_queue_lock.len() as f32 * (1. - self.config.buffering_history_weight);
        if self.buffering_running_average > self.config.max_buffering_frames as f32 {
            image_queue_lock.pop_front();
        }

        if let Some(queued_image) = image_queue_lock.front_mut() {
            queued_image.in_use = true;

            Some((
                queued_image.timestamp,
                queued_image
                    .image
                    .hardware_buffer()
                    .unwrap()
                    .as_ptr()
                    .cast(),
            ))
        } else {
            // TODO: add back when implementing proper phase sync
            //warn!("Video frame queue underflow!");
            None
        }
    }
}

impl Drop for VideoDecoderSource {
    fn drop(&mut self) {
        self.running.set(false);

        // Destruction of decoder, buffered images and ImageReader
        self.dequeue_thread.take().map(|t| t.join());
    }
}

fn mime_for_codec(codec: CodecType) -> &'static str {
    match codec {
        CodecType::H264 => "video/avc",
        CodecType::Hevc => "video/hevc",
        CodecType::AV1 => "video/av01",
    }
}

// Attempts to create a MediaCodec, and then configure and start it.
fn decoder_attempt_setup(
    codec_type: CodecType,
    is_software: bool,
    format: &MediaFormat,
    image_reader: &ImageReader,
) -> Result<MediaCodec> {
    let decoder = if is_software {
        let sw_codec_name = match codec_type {
            CodecType::H264 => "OMX.google.h264.decoder",
            CodecType::Hevc => "OMX.google.hevc.decoder",
            CodecType::AV1 => bail!("AV1 is not supported for software decoding"),
        };
        MediaCodec::from_codec_name(&sw_codec_name)
            .ok_or(anyhow!("no such codec: {}", &sw_codec_name))?
    } else {
        let mime = mime_for_codec(codec_type);
        MediaCodec::from_decoder_type(&mime)
            .ok_or(anyhow!("unable to find decoder for mime type: {}", &mime))?
    };
    decoder
        .configure(
            &format,
            Some(&image_reader.window()?),
            MediaCodecDirection::Decoder,
        )
        .with_context(|| format!("failed to configure decoder"))?;

    decoder
        .start()
        .with_context(|| format!("failed to start decoder"))?;

    Ok(decoder)
}

// Since we leak the ImageReader, and we pass frame_result_callback to it which contains a reference
// to ClientCoreContext, to avoid circular references we need to use a Weak reference.
fn decoder_lifecycle(
    config: VideoDecoderConfig,
    csd_0: Vec<u8>,
    frame_result_callback: Weak<impl Fn(Result<Duration>) + Send + Sync + 'static>,
    running: Arc<RelaxedAtomic>,
    decoder_sink: Arc<Mutex<Option<SharedMediaCodec>>>,
    decoder_ready_notifier: Arc<Condvar>,
    image_queue: Arc<Mutex<VecDeque<QueuedImage>>>,
    image_reader: &mut ImageReader,
) -> Result<()> {
    // 2x: keep the target buffering in the middle of the max amount of queuable frames
    let available_buffering_frames = (2. * config.max_buffering_frames).ceil() as usize;

    image_reader.set_image_listener(Box::new({
        let image_queue = Arc::clone(&image_queue);
        move |image_reader| {
            let mut image_queue_lock = image_queue.lock();

            if image_queue_lock.len() > available_buffering_frames {
                warn!("Video frame queue overflow!");
                image_queue_lock.pop_front();
            }

            match image_reader.acquire_next_image() {
                Ok(AcquireResult::Image(image)) => {
                    let timestamp = Duration::from_nanos(image.timestamp().unwrap() as u64);

                    if let Some(callback) = frame_result_callback.upgrade() {
                        callback(Ok(timestamp));
                    }

                    image_queue_lock.push_back(QueuedImage {
                        timestamp,
                        image,
                        in_use: false,
                    });
                }
                Ok(e) => {
                    error!("ImageReader error: {e:?}");

                    image_queue_lock.pop_front();
                }
                Err(e) => {
                    error!("ImageReader error: {e}");

                    image_queue_lock.clear();
                }
            }
        }
    }))?;

    // Documentation says that this call is necessary to properly dispose acquired buffers.
    // todo: find out how to use it and avoid leaking the ImageReader
    image_reader.set_buffer_removed_listener(Box::new(|_, _| ()))?;

    let mime = mime_for_codec(config.codec);

    let mut format = MediaFormat::new();
    format.set_str("mime", mime);
    // Given https://github.com/alvr-org/ALVR/pull/1933#discussion_r1431902906 - change at own risk.
    // It might be harmless, it might not be, but it's definitely a risk.
    format.set_i32("width", 512);
    format.set_i32("height", 1024);
    format.set_buffer("csd-0", &csd_0);

    for (key, prop) in &config.options {
        let maybe_error = match prop.ty {
            MediacodecPropType::Float => prop
                .value
                .parse()
                .map(|value| format.set_f32(key, value))
                .to_any(),
            MediacodecPropType::Int32 => prop
                .value
                .parse()
                .map(|value| format.set_i32(key, value))
                .to_any(),
            MediacodecPropType::Int64 => prop
                .value
                .parse()
                .map(|value| format.set_i64(key, value))
                .to_any(),
            MediacodecPropType::String => Ok(format.set_str(key, &prop.value)),
        };

        if let Err(e) = maybe_error {
            error!("Failed to set property {key} to {}: {e}", prop.value);
        }
    }

    info!("Using AMediaCodec format:{} ", format);

    let decoder = if config.force_software_decoder {
        decoder_attempt_setup(config.codec, true, &format, &image_reader)?
    } else {
        // Hardware decoders sometimes fail at the CSD-0.
        // May as well fall back if this occurs.
        match decoder_attempt_setup(config.codec, false, &format, &image_reader) {
            Ok(d) => d,
            Err(e) => {
                // would be "warn!" but this is a severe caveat and a pretty major error.
                error!("Attempting software fallback due to error in default decoder: {e:#}");

                decoder_attempt_setup(config.codec, true, &format, &image_reader)?
            }
        }
    };

    let decoder = Arc::new(FakeThreadSafe(decoder));

    {
        let mut decoder_lock = decoder_sink.lock();

        *decoder_lock = Some(Arc::clone(&decoder));

        decoder_ready_notifier.notify_one();
    }

    let mut error_counter = 0;
    while running.value() {
        match decoder.dequeue_output_buffer(Duration::from_millis(1)) {
            Ok(DequeuedOutputBufferInfoResult::Buffer(buffer)) => {
                // The buffer timestamp is actually nanoseconds
                let presentation_time_ns = buffer.info().presentation_time_us();

                if let Err(e) = decoder.release_output_buffer_at_time(buffer, presentation_time_ns)
                {
                    error!("Decoder dequeue error: {e}");
                }
            }
            Ok(DequeuedOutputBufferInfoResult::TryAgainLater) => continue,
            Ok(i) => info!("Decoder dequeue event: {i:?}"),
            Err(e) => {
                error!("Decoder dequeue error: {e}");

                error_counter += 1;
                if error_counter > 10 {
                    bail!("Too many decoder errors: {e}");
                }

                // lessen logcat flood (just in case)
                thread::sleep(Duration::from_millis(50));

                continue;
            }
        }

        error_counter = 0;
    }

    // Destroy all resources
    decoder_sink.lock().take(); // Make sure the shared ref is deleted first
    decoder.stop()?;
    drop(decoder);

    Ok(())
}

// Create a sink/source pair
pub fn video_decoder_split(
    config: VideoDecoderConfig,
    csd_0: Vec<u8>,
    frame_result_callback: impl Fn(Result<Duration>) + Send + Sync + 'static,
) -> Result<(VideoDecoderSink, VideoDecoderSource)> {
    let running = Arc::new(RelaxedAtomic::new(true));
    let decoder_sink = Arc::new(Mutex::new(None::<SharedMediaCodec>));
    let decoder_ready_notifier = Arc::new(Condvar::new());
    let image_queue = Arc::new(Mutex::new(VecDeque::<QueuedImage>::new()));

    let dequeue_thread = thread::spawn({
        let config = config.clone();
        let running = Arc::clone(&running);
        let decoder_sink = Arc::clone(&decoder_sink);
        let decoder_ready_notifier = Arc::clone(&decoder_ready_notifier);
        let image_queue = Arc::clone(&image_queue);
        move || {
            const MAX_BUFFERING_FRAMES: usize = 10;
            let mut image_reader = match ImageReader::new_with_usage(
                1,
                1,
                ImageFormat::PRIVATE,
                HardwareBufferUsage::GPU_SAMPLED_IMAGE,
                MAX_BUFFERING_FRAMES as i32,
            ) {
                Ok(reader) => reader,
                Err(e) => {
                    frame_result_callback(Err(anyhow!("{e}")));
                    return;
                }
            };

            let frame_result_callback = Arc::new(frame_result_callback);

            if let Err(e) = decoder_lifecycle(
                config,
                csd_0,
                Arc::downgrade(&frame_result_callback),
                running,
                decoder_sink,
                decoder_ready_notifier,
                Arc::clone(&image_queue),
                &mut image_reader,
            ) {
                frame_result_callback(Err(e));
            }

            image_queue.lock().clear();
            error!("FIXME: Leaking Imagereader!");
            Box::leak(Box::new(image_reader));
        }
    });

    // Make sure the decoder is ready: we don't want to try to enqueue frame and lose them, to avoid
    // image corruption.
    {
        let mut decoder_lock = decoder_sink.lock();

        if decoder_lock.is_none() {
            // No spurious wakeups
            decoder_ready_notifier.wait(&mut decoder_lock);
        }
    }

    let sink = VideoDecoderSink {
        inner: decoder_sink,
    };
    let source = VideoDecoderSource {
        running,
        dequeue_thread: Some(dequeue_thread),
        image_queue,
        config,
        buffering_running_average: 0.0,
    };

    Ok((sink, source))
}
