use alvr_common::{
    glam::UVec2,
    once_cell::sync::{Lazy, OnceCell},
    parking_lot::{Condvar, Mutex},
    prelude::*,
};
use alvr_session::{CodecType, MediacodecDataType};
use jni::{objects::GlobalRef, sys::jobject, JavaVM};
use ndk::{
    hardware_buffer::HardwareBufferUsage,
    media::{
        image_reader::{Image, ImageFormat, ImageListener, ImageReader},
        media_codec::{
            MediaCodec, MediaCodecDirection, MediaCodecInfo, MediaCodecResult, MediaFormat,
        },
    },
    native_window::NativeWindow,
};
use ndk_sys as sys;
use std::{
    collections::HashMap,
    ffi::{c_void, CString},
    ptr::NonNull,
    sync::Arc,
    time::Duration,
};

const MICROPHONE_PERMISSION: &str = "android.permission.RECORD_AUDIO";

pub fn vm() -> JavaVM {
    unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()).unwrap() }
}

pub fn context() -> jobject {
    ndk_context::android_context().context().cast()
}

pub fn try_get_microphone_permission() {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    let mic_perm_jstring = env.new_string(MICROPHONE_PERMISSION).unwrap();

    let permission_status = env
        .call_method(
            context(),
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
            context(),
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[perm_array.into(), 0.into()],
        )
        .unwrap();

        // todo: handle case where permission is rejected
    }
}

pub fn device_name() -> String {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    let jbrand_name = env
        .get_static_field("android/os/Build", "BRAND", "Ljava/lang/String;")
        .unwrap()
        .l()
        .unwrap();
    let brand_name_raw = env.get_string(jbrand_name.into()).unwrap();
    let brand_name = brand_name_raw.to_string_lossy().as_ref().to_owned();
    // Capitalize first letter
    let mut brand_name_it = brand_name.chars();
    let brand_name = brand_name_it
        .next()
        .unwrap()
        .to_uppercase()
        .chain(brand_name_it)
        .collect::<String>();

    let jdevice_name = env
        .get_static_field("android/os/Build", "MODEL", "Ljava/lang/String;")
        .unwrap()
        .l()
        .unwrap();
    let device_name_raw = env.get_string(jdevice_name.into()).unwrap();
    let device_name = device_name_raw.to_string_lossy().as_ref().to_owned();

    format!("{brand_name} {device_name}")
}

pub struct VideoDecoderEnqueuer {
    inner: Arc<MediaCodec>,
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

#[derive(Debug)]
pub enum DecoderDequeuedData {
    Frame {
        buffer_ptr: *mut c_void,
        timestamp: Duration,
    },
    TryAgainLater,
    OtherNonFatal,
}

pub struct VideoDecoderDequeuer {
    inner: Option<Arc<MediaCodec>>,
    image_reader: Option<ImageReader>,
    acquired_image: Arc<Mutex<StrResult<Option<Image>>>>,
    image_acquired_notifier: Arc<Condvar>,
}

unsafe impl Send for VideoDecoderDequeuer {}

impl VideoDecoderDequeuer {
    // dequeue_timeout: Should be small.
    // deadlock_timeout: Should be not too small, not too large. When this timeout is reached an
    // error is returned and the client should be disconnected
    pub fn dequeue_frame(
        &self,
        dequeue_timeout: Duration,  // should be small
        deadlock_timeout: Duration, // should be not too small not too large
    ) -> StrResult<DecoderDequeuedData> {
        let mut acquired_image_ref = self.acquired_image.lock();

        match self
            .inner
            .as_ref()
            .unwrap()
            .dequeue_output_buffer(dequeue_timeout)
        {
            MediaCodecResult::Ok(buffer) => {
                // The buffer timestamp is actually nanoseconds
                let timestamp = Duration::from_nanos(buffer.presentation_time_us() as _);

                self.inner
                    .as_ref()
                    .unwrap()
                    .release_output_buffer(buffer, true)
                    .map_err(err!())?;

                // Note: parking_lot::Condvar has no spurious wakeups
                if self
                    .image_acquired_notifier
                    .wait_for(&mut acquired_image_ref, deadlock_timeout)
                    .timed_out()
                {
                    return fmt_e!("ImageReader stalled");
                }

                match &*acquired_image_ref {
                    Ok(Some(image)) => Ok(DecoderDequeuedData::Frame {
                        buffer_ptr: image.get_hardware_buffer().map_err(err!())?.as_ptr().cast(),
                        timestamp,
                    }),
                    Ok(None) => fmt_e!("No buffer available"),
                    Err(e) => fmt_e!("{e}"),
                }
            }
            MediaCodecResult::Info(MediaCodecInfo::TryAgainLater) => {
                Ok(DecoderDequeuedData::TryAgainLater)
            }
            MediaCodecResult::Info(_) => Ok(DecoderDequeuedData::OtherNonFatal),
            MediaCodecResult::Err(e) => fmt_e!("{e}"),
        }
    }
}

impl Drop for VideoDecoderDequeuer {
    fn drop(&mut self) {
        // Make sure the ImageReader surface is not used anymore. Destroy the decoder
        drop(self.inner.take());

        // Make sure there is no lingering image form the ImageReader
        *self.acquired_image.lock() = Ok(None);

        // Finally destroy the ImageReader
        // FIXME: it still crashes!
        // drop(self.image_reader.take());

        // Since I cannot destroy the ImageReader, leak its memory
        // THIS IS VERY WRONG. todo: find solution ASAP
        error!("Leaking ImageReader. FIXME");
        Box::leak(Box::new(self.image_reader.take()));
    }
}

pub fn video_decoder_split(
    codec_type: CodecType,
    csd_0: &[u8],
    extra_options: &[(String, MediacodecDataType)],
) -> StrResult<(VideoDecoderEnqueuer, VideoDecoderDequeuer)> {
    let mut image_reader = ImageReader::new_with_usage(
        1,
        1,
        ImageFormat::PRIVATE,
        HardwareBufferUsage::GPU_SAMPLED_IMAGE,
        5,
    )
    .unwrap();

    let mime = match codec_type {
        CodecType::H264 => "video/avc",
        CodecType::HEVC => "video/hevc",
    };

    let format = MediaFormat::new();
    format.set_str("mime", mime);
    format.set_i32("width", 512);
    format.set_i32("height", 1024);
    format.set_buffer("csd-0", csd_0);

    for (key, value) in extra_options {
        match value {
            MediacodecDataType::Float(value) => format.set_f32(key, *value),
            MediacodecDataType::Int32(value) => format.set_i32(key, *value),
            MediacodecDataType::Int64(value) => format.set_i64(key, *value),
            MediacodecDataType::String(value) => format.set_str(key, value),
        }
    }

    let decoder = Arc::new(MediaCodec::from_decoder_type(mime).ok_or_else(enone!())?);
    decoder
        .configure(
            &format,
            Some(&image_reader.get_window().unwrap()),
            MediaCodecDirection::Decoder,
        )
        .map_err(err!())?;
    decoder.start().map_err(err!())?;

    let acquired_image = Arc::new(Mutex::new(Ok(None)));
    let image_acquired_notifier = Arc::new(Condvar::new());

    image_reader
        .set_image_listener(Box::new({
            let acquired_image = Arc::clone(&acquired_image);
            let image_acquired_notifier = Arc::clone(&image_acquired_notifier);
            move |image_reader| {
                let mut acquired_image_lock = acquired_image.lock();
                *acquired_image_lock = image_reader.acquire_latest_image().map_err(err!());
                image_acquired_notifier.notify_one();
            }
        }))
        .map_err(err!())?;

    // Documentation says that this call is necessary to properly dispose acquired buffers.
    // todo: find out how to use it and avoid leaking the ImageReader
    image_reader
        .set_buffer_removed_listener(Box::new(|_, _| ()))
        .map_err(err!())?;

    Ok((
        VideoDecoderEnqueuer {
            inner: Arc::clone(&decoder),
        },
        VideoDecoderDequeuer {
            inner: Some(Arc::clone(&decoder)),
            image_reader: Some(image_reader),
            acquired_image,
            image_acquired_notifier,
        },
    ))
}
