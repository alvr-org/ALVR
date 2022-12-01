use crate::decoder::DecoderInitConfig;
use alvr_common::{
    once_cell::sync::Lazy,
    parking_lot::{Condvar, Mutex},
    prelude::*,
    RelaxedAtomic,
};
use alvr_session::{CodecType, MediacodecDataType};
use jni::{
    objects::{GlobalRef, JObject, JString},
    sys::jobject,
    JNIEnv, JavaVM,
};
use ndk::{
    hardware_buffer::HardwareBufferUsage,
    media::{
        image_reader::{Image, ImageFormat, ImageReader},
        media_codec::{
            MediaCodec, MediaCodecDirection, MediaCodecInfo, MediaCodecResult, MediaFormat,
        },
    },
};
use std::{
    collections::VecDeque,
    ffi::{c_void, CStr},
    net::{IpAddr, Ipv4Addr},
    ops::{Deref, DerefMut},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

const IMAGE_READER_DEADLOCK_TIMEOUT: Duration = Duration::from_millis(100);
static WIFI_LOCK: Lazy<Mutex<Option<GlobalRef>>> = Lazy::new(|| Mutex::new(None));

pub fn vm() -> JavaVM {
    unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap() }
}

pub fn context() -> jobject {
    ndk_context::android_context().context().cast()
}

fn get_api_level() -> i32 {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
        .unwrap()
        .i()
        .unwrap()
}

pub fn try_get_microphone_permission() {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    let mic_perm_jstring = env.new_string("android.permission.RECORD_AUDIO").unwrap();

    let permission_status = env
        .call_method(
            unsafe { JObject::from_raw(context()) },
            "checkSelfPermission",
            "(Ljava/lang/String;)I",
            &[mic_perm_jstring.into()],
        )
        .unwrap()
        .i()
        .unwrap();

    if permission_status != 0 {
        let string_class = env.find_class("java/lang/String").unwrap();
        let perm_array = env
            .new_object_array(1, string_class, mic_perm_jstring)
            .unwrap();

        env.call_method(
            unsafe { JObject::from_raw(context()) },
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[unsafe { JObject::from_raw(perm_array) }.into(), 0.into()],
        )
        .unwrap();

        // todo: handle case where permission is rejected
    }
}

pub fn device_model() -> String {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    let jdevice_name = env
        .get_static_field("android/os/Build", "MODEL", "Ljava/lang/String;")
        .unwrap()
        .l()
        .unwrap();
    let device_name_raw = env.get_string(jdevice_name.into()).unwrap();

    device_name_raw.to_string_lossy().as_ref().to_owned()
}

struct FakeThreadSafe<T>(T);
unsafe impl<T> Send for FakeThreadSafe<T> {}
unsafe impl<T> Sync for FakeThreadSafe<T> {}

impl<T> Deref for FakeThreadSafe<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for FakeThreadSafe<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

fn get_wifi_manager<'a>(env: &JNIEnv<'a>) -> JObject<'a> {
    let wifi_service_str = env.new_string("wifi").unwrap();

    env.call_method(
        unsafe { JObject::from_raw(context()) },
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[wifi_service_str.into()],
    )
    .unwrap()
    .l()
    .unwrap()
}

// Note: tried and failed to use libc
pub fn local_ip() -> IpAddr {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    let wifi_manager = get_wifi_manager(&env);
    let wifi_info = env
        .call_method(
            wifi_manager,
            "getConnectionInfo",
            "()Landroid/net/wifi/WifiInfo;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let ip_i32 = env
        .call_method(wifi_info, "getIpAddress", "()I", &[])
        .unwrap()
        .i()
        .unwrap();

    let ip_arr = ip_i32.to_le_bytes();

    IpAddr::V4(Ipv4Addr::new(ip_arr[0], ip_arr[1], ip_arr[2], ip_arr[3]))
}

// This is needed to avoid wifi scans that disrupt streaming.
pub fn acquire_wifi_lock() {
    let mut maybe_wifi_lock = WIFI_LOCK.lock();

    if maybe_wifi_lock.is_none() {
        let vm = vm();
        let env = vm.attach_current_thread().unwrap();

        let wifi_mode = if get_api_level() >= 29 {
            // Recommended for virtual reality since it disables WIFI scans
            4 // WIFI_MODE_FULL_LOW_LATENCY
        } else {
            3 // WIFI_MODE_FULL_HIGH_PERF
        };

        let wifi_manager = get_wifi_manager(&env);
        let wifi_lock_jstring = env.new_string("alvr_wifi_lock").unwrap();
        let wifi_lock = env
            .call_method(
                wifi_manager,
                "createWifiLock",
                "(ILjava/lang/String;)Landroid/net/wifi/WifiManager$WifiLock;",
                &[wifi_mode.into(), wifi_lock_jstring.into()],
            )
            .unwrap()
            .l()
            .unwrap();
        env.call_method(wifi_lock, "acquire", "()V", &[]).unwrap();

        *maybe_wifi_lock = Some(env.new_global_ref(wifi_lock).unwrap());
    }
}

pub fn release_wifi_lock() {
    if let Some(wifi_lock) = WIFI_LOCK.lock().take() {
        let vm = vm();
        let env = vm.attach_current_thread().unwrap();

        env.call_method(wifi_lock.as_obj(), "release", "()V", &[])
            .unwrap();

        // wifi_lock is dropped here
    }
}

pub struct VideoDecoderEnqueuer {
    inner: Arc<FakeThreadSafe<MediaCodec>>,
}

unsafe impl Send for VideoDecoderEnqueuer {}

impl VideoDecoderEnqueuer {
    // Block until the buffer has been written or timeout is reached. Returns false if timeout.
    pub fn push_frame_nal(
        &self,
        timestamp: Duration,
        data: &[u8],
        timeout: Duration,
    ) -> StrResult<bool> {
        match self.inner.dequeue_input_buffer(timeout) {
            MediaCodecResult::Ok(mut buffer) => {
                buffer.buffer_mut()[..data.len()].copy_from_slice(data);

                // NB: the function expects the timestamp in micros, but nanos is used to have complete
                // precision, so when converted back to Duration it can compare correctly to other
                // Durations
                self.inner
                    .queue_input_buffer(buffer, 0, data.len(), timestamp.as_nanos() as _, 0)
                    .map_err(err!())?;

                Ok(true)
            }
            MediaCodecResult::Info(_) => {
                // Should be TryAgainLater
                Ok(false)
            }
            MediaCodecResult::Err(e) => fmt_e!("{e}"),
        }
    }
}

pub struct DequeuedFrame {
    pub timestamp: Duration,
    pub buffer_ptr: *mut c_void,
}

struct QueuedImage {
    timestamp: Duration,
    image: Image,
    in_use: bool,
}
unsafe impl Send for QueuedImage {}

// Access the image queue synchronously.
pub struct VideoDecoderDequeuer {
    running: Arc<RelaxedAtomic>,
    dequeue_thread: Option<JoinHandle<()>>,
    image_queue: Arc<Mutex<VecDeque<QueuedImage>>>,
    target_buffering_frames: f32,
    buffering_history_weight: f32,
    buffering_running_average: f32,
}

unsafe impl Send for VideoDecoderDequeuer {}

impl VideoDecoderDequeuer {
    // The application MUST finish using the returned buffer before calling this function again
    pub fn dequeue_frame(&mut self) -> Option<DequeuedFrame> {
        let mut image_queue_lock = self.image_queue.lock();

        if let Some(queued_image) = image_queue_lock.front() {
            if queued_image.in_use {
                // image is released and ready to be reused by the decoder
                image_queue_lock.pop_front();
            }
        }

        // use running average to give more weight to recent samples
        self.buffering_running_average = self.buffering_running_average
            * self.buffering_history_weight
            + image_queue_lock.len() as f32 * (1. - self.buffering_history_weight);
        if self.buffering_running_average > self.target_buffering_frames as f32 {
            image_queue_lock.pop_front();
        }

        if let Some(queued_image) = image_queue_lock.front_mut() {
            queued_image.in_use = true;

            Some(DequeuedFrame {
                timestamp: queued_image.timestamp,
                buffer_ptr: queued_image
                    .image
                    .get_hardware_buffer()
                    .unwrap()
                    .as_ptr()
                    .cast(),
            })
        } else {
            warn!("Video frame queue underflow!");

            None
        }
    }
}

impl Drop for VideoDecoderDequeuer {
    fn drop(&mut self) {
        self.running.set(false);

        // Destruction of decoder, buffered images and ImageReader
        self.dequeue_thread.take().map(|t| t.join());
    }
}

pub fn video_decoder_split(
    config: DecoderInitConfig,
    csd_0: &[u8],
    dequeued_frame_callback: impl Fn(Duration) + Send + 'static,
) -> StrResult<(VideoDecoderEnqueuer, VideoDecoderDequeuer)> {
    // 2x: keep the target buffering in the middle of the max amount of queuable frames
    let available_buffering_frames = (2. * config.max_buffering_frames).ceil() as usize;

    let image_reader = ImageReader::new_with_usage(
        1,
        1,
        ImageFormat::PRIVATE,
        HardwareBufferUsage::GPU_SAMPLED_IMAGE,
        available_buffering_frames as i32 + 1 + 1,
        // + 1 for decoder (internal) + 1 for rendering (in_use == true)
    )
    .unwrap();

    let mime = match config.codec {
        CodecType::H264 => "video/avc",
        CodecType::HEVC => "video/hevc",
    };

    let format = MediaFormat::new();
    format.set_str("mime", mime);
    format.set_i32("width", 512);
    format.set_i32("height", 1024);
    format.set_buffer("csd-0", csd_0);

    for (key, value) in config.options {
        match value {
            MediacodecDataType::Float(value) => format.set_f32(&key, value),
            MediacodecDataType::Int32(value) => format.set_i32(&key, value),
            MediacodecDataType::Int64(value) => format.set_i64(&key, value),
            MediacodecDataType::String(value) => format.set_str(&key, &value),
        }
    }

    let decoder = Arc::new(FakeThreadSafe(
        MediaCodec::from_decoder_type(mime).ok_or_else(enone!())?,
    ));
    decoder
        .configure(
            &format,
            Some(&image_reader.get_window().unwrap()),
            MediaCodecDirection::Decoder,
        )
        .map_err(err!())?;
    decoder.start().map_err(err!())?;

    let mut image_reader = FakeThreadSafe(image_reader);
    let running = Arc::new(RelaxedAtomic::new(true));
    let image_queue = Arc::new(Mutex::new(VecDeque::<QueuedImage>::new()));

    let dequeue_thread = thread::spawn({
        let running = Arc::clone(&running);
        let decoder = Arc::clone(&decoder);
        let image_queue = Arc::clone(&image_queue);
        move || {
            let acquired_image = Arc::new(Mutex::new(Ok(None)));
            let image_acquired_notifier = Arc::new(Condvar::new());

            image_reader
                .set_image_listener(Box::new({
                    let acquired_image = Arc::clone(&acquired_image);
                    let image_acquired_notifier = Arc::clone(&image_acquired_notifier);
                    move |image_reader| {
                        let mut acquired_image_lock = acquired_image.lock();
                        *acquired_image_lock = image_reader.acquire_next_image().map_err(err!());
                        image_acquired_notifier.notify_one();
                    }
                }))
                .unwrap();

            // Documentation says that this call is necessary to properly dispose acquired buffers.
            // todo: find out how to use it and avoid leaking the ImageReader
            image_reader
                .set_buffer_removed_listener(Box::new(|_, _| ()))
                .unwrap();

            let mut overflow_logged = false;
            while running.value() {
                // Check if there is any image ready to be used by the decoder, ie the queue is not
                // full. in this case use a simple loop, no need for anything fancier since this is
                // an exceptional situation.
                if image_queue.lock().len() > available_buffering_frames {
                    // use a flag to avoid flooding the logcat
                    if !overflow_logged {
                        warn!("Video frame queue overflow!");
                        overflow_logged = true;
                    }

                    thread::sleep(Duration::from_millis(1));

                    continue;
                } else {
                    overflow_logged = false;
                }

                let mut acquired_image_ref = acquired_image.lock();

                match decoder.dequeue_output_buffer(Duration::from_millis(1)) {
                    MediaCodecResult::Ok(buffer) => {
                        // The buffer timestamp is actually nanoseconds
                        let timestamp = Duration::from_nanos(buffer.presentation_time_us() as _);

                        if let Err(e) = decoder.release_output_buffer(buffer, true) {
                            error!("Decoder dequeue error: {e}");

                            break;
                        }

                        dequeued_frame_callback(timestamp);

                        // Note: parking_lot::Condvar has no spurious wakeups
                        if image_acquired_notifier
                            .wait_for(&mut acquired_image_ref, IMAGE_READER_DEADLOCK_TIMEOUT)
                            .timed_out()
                        {
                            error!("ImageReader stalled");

                            break;
                        }

                        match &mut *acquired_image_ref {
                            Ok(image @ Some(_)) => {
                                image_queue.lock().push_back(QueuedImage {
                                    timestamp,
                                    image: image.take().unwrap(),
                                    in_use: false,
                                });
                            }
                            Ok(None) => {
                                error!("ImageReader error: No buffer available");
                                break;
                            }
                            Err(e) => {
                                error!("ImageReader error: {e}");
                                break;
                            }
                        }
                    }
                    MediaCodecResult::Info(MediaCodecInfo::TryAgainLater) => (),
                    MediaCodecResult::Info(i) => info!("Decoder dequeue event: {i:?}"),
                    MediaCodecResult::Err(e) => {
                        error!("Decoder dequeue error: {e}");

                        // lessen logcat flood (just in case)
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }

            // Make sure the ImageReader surface is not used anymore. Destroy the decoder
            // Supposes that VideoDecoderEnqueuer has already been destroyed.
            drop(decoder);

            // Make sure there is no lingering image from the ImageReader
            image_queue.lock().clear();

            // Finally destroy the ImageReader
            // FIXME: it still crashes!
            // drop(image_reader);

            // Since I cannot destroy the ImageReader, leak its memory
            // THIS IS VERY WRONG. todo: find solution ASAP
            error!("Leaking ImageReader. FIXME");
            Box::leak(Box::new(image_reader));
        }
    });

    Ok((
        VideoDecoderEnqueuer { inner: decoder },
        VideoDecoderDequeuer {
            running,
            dequeue_thread: Some(dequeue_thread),
            image_queue,
            target_buffering_frames: config.max_buffering_frames,
            buffering_history_weight: config.buffering_history_weight,
            buffering_running_average: 0.0,
        },
    ))
}
