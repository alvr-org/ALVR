use crate::decoder::DecoderInitConfig;
use alvr_common::{
    once_cell::sync::Lazy,
    parking_lot::{Condvar, Mutex},
    prelude::*,
    RelaxedAtomic,
};
use alvr_session::{CodecType, MediacodecDataType};
use jni::{
    objects::{GlobalRef, JObject},
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
    ffi::c_void,
    net::{IpAddr, Ipv4Addr},
    ops::Deref,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

static WIFI_LOCK: Lazy<Mutex<Option<GlobalRef>>> = Lazy::new(|| Mutex::new(None));

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

    let jname = env
        .get_static_field("android/os/Build", "MODEL", "Ljava/lang/String;")
        .unwrap()
        .l()
        .unwrap();
    let name_raw = env.get_string(jname.into()).unwrap();

    name_raw.to_string_lossy().as_ref().to_owned()
}

pub fn manufacturer_name() -> String {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    let jname = env
        .get_static_field("android/os/Build", "MANUFACTURER", "Ljava/lang/String;")
        .unwrap()
        .l()
        .unwrap();
    let name_raw = env.get_string(jname.into()).unwrap();

    name_raw.to_string_lossy().as_ref().to_owned()
}

fn get_system_service<'a>(env: &JNIEnv<'a>, service_name: &str) -> JObject<'a> {
    let service_str = env.new_string(service_name).unwrap();

    env.call_method(
        unsafe { JObject::from_raw(context()) },
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[service_str.into()],
    )
    .unwrap()
    .l()
    .unwrap()
}

// Note: tried and failed to use libc
pub fn local_ip() -> IpAddr {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    let wifi_manager = get_system_service(&env, "wifi");
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

        let wifi_manager = get_system_service(&env, "wifi");
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

pub fn battery_status() -> (f32, bool) {
    let vm = vm();
    let env = vm.attach_current_thread().unwrap();

    const BATTERY_PROPERTY_CAPACITY: i32 = 4;

    let battery_manager = get_system_service(&env, "batterymanager");

    let percentage = env
        .call_method(
            battery_manager,
            "getIntProperty",
            "(I)I",
            &[BATTERY_PROPERTY_CAPACITY.into()],
        )
        .unwrap()
        .i()
        .unwrap();

    let is_charging = env
        .call_method(battery_manager, "isCharging", "()Z", &[])
        .unwrap()
        .z()
        .unwrap();

    (percentage as f32 / 100.0, is_charging)
}

pub struct VideoDecoderEnqueuer {
    inner: Arc<Mutex<Option<SharedMediaCodec>>>,
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
        let Some(decoder) = &*self.inner.lock() else {
            // This might happen only during destruction
            return Ok(false);
        };

        match decoder.dequeue_input_buffer(timeout) {
            MediaCodecResult::Ok(mut buffer) => {
                buffer.buffer_mut()[..data.len()].copy_from_slice(data);

                // NB: the function expects the timestamp in micros, but nanos is used to have
                // complete precision, so when converted back to Duration it can compare correctly
                // to other Durations
                decoder
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
    config: DecoderInitConfig,
    buffering_running_average: f32,
}

unsafe impl Send for VideoDecoderDequeuer {}

impl VideoDecoderDequeuer {
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
                    .get_hardware_buffer()
                    .unwrap()
                    .as_ptr()
                    .cast(),
            ))
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

// Create a enqueuer/dequeuer pair. To preserve the state of internal variables, use
// `enqueuer.recreate_decoder()` instead of dropping the pair and calling this function again.
pub fn video_decoder_split(
    config: DecoderInitConfig,
    csd_0: Vec<u8>,
    dequeued_frame_callback: impl Fn(Duration) + Send + 'static,
) -> StrResult<(VideoDecoderEnqueuer, VideoDecoderDequeuer)> {
    let running = Arc::new(RelaxedAtomic::new(true));
    let decoder_enqueuer = Arc::new(Mutex::new(None::<SharedMediaCodec>));
    let decoder_ready_notifier = Arc::new(Condvar::new());
    let image_queue = Arc::new(Mutex::new(VecDeque::<QueuedImage>::new()));

    let dequeue_thread = thread::spawn({
        let config = config.clone();
        let running = Arc::clone(&running);
        let decoder_enqueuer = Arc::clone(&decoder_enqueuer);
        let decoder_ready_notifier = Arc::clone(&decoder_ready_notifier);
        let image_queue = Arc::clone(&image_queue);
        move || {
            const MAX_BUFFERING_FRAMES: usize = 10;

            // 2x: keep the target buffering in the middle of the max amount of queuable frames
            let available_buffering_frames = (2. * config.max_buffering_frames).ceil() as usize;

            let mime = match config.codec {
                CodecType::H264 => "video/avc",
                CodecType::HEVC => "video/hevc",
            };

            let format = MediaFormat::new();
            format.set_str("mime", mime);
            format.set_i32("width", 512);
            format.set_i32("height", 1024);
            format.set_buffer("csd-0", &csd_0);

            for (key, value) in &config.options {
                match value {
                    MediacodecDataType::Float(value) => format.set_f32(key, *value),
                    MediacodecDataType::Int32(value) => format.set_i32(key, *value),
                    MediacodecDataType::Int64(value) => format.set_i64(key, *value),
                    MediacodecDataType::String(value) => format.set_str(key, value),
                }
            }

            let mut image_reader = ImageReader::new_with_usage(
                1,
                1,
                ImageFormat::PRIVATE,
                HardwareBufferUsage::GPU_SAMPLED_IMAGE,
                MAX_BUFFERING_FRAMES as i32,
            )
            .unwrap();

            image_reader
                .set_image_listener(Box::new({
                    let image_queue = Arc::clone(&image_queue);
                    move |image_reader| {
                        let mut image_queue_lock = image_queue.lock();

                        if image_queue_lock.len() > available_buffering_frames {
                            warn!("Video frame queue overflow!");
                            image_queue_lock.pop_front();
                        }

                        match &mut image_reader.acquire_next_image() {
                            Ok(image @ Some(_)) => {
                                let image = image.take().unwrap();
                                let timestamp =
                                    Duration::from_nanos(image.get_timestamp().unwrap() as u64);

                                dequeued_frame_callback(timestamp);

                                image_queue_lock.push_back(QueuedImage {
                                    timestamp,
                                    image,
                                    in_use: false,
                                });
                            }
                            Ok(None) => {
                                error!("ImageReader error: No buffer available");

                                image_queue_lock.clear();
                            }
                            Err(e) => {
                                error!("ImageReader error: {e}");

                                image_queue_lock.clear();
                            }
                        }
                    }
                }))
                .unwrap();

            // Documentation says that this call is necessary to properly dispose acquired buffers.
            // todo: find out how to use it and avoid leaking the ImageReader
            image_reader
                .set_buffer_removed_listener(Box::new(|_, _| ()))
                .unwrap();

            let decoder = Arc::new(FakeThreadSafe(
                MediaCodec::from_decoder_type(&mime).unwrap(),
            ));
            decoder
                .configure(
                    &format,
                    Some(&image_reader.get_window().unwrap()),
                    MediaCodecDirection::Decoder,
                )
                .unwrap();
            decoder.start().unwrap();

            {
                let mut decoder_lock = decoder_enqueuer.lock();

                *decoder_lock = Some(Arc::clone(&decoder));

                decoder_ready_notifier.notify_one();
            }

            while running.value() {
                match decoder.dequeue_output_buffer(Duration::from_millis(1)) {
                    MediaCodecResult::Ok(buffer) => {
                        // The buffer timestamp is actually nanoseconds
                        let presentation_time_ns = buffer.presentation_time_us();

                        if let Err(e) =
                            decoder.release_output_buffer_at_time(buffer, presentation_time_ns)
                        {
                            error!("Decoder dequeue error: {e}");
                        }
                    }
                    MediaCodecResult::Info(MediaCodecInfo::TryAgainLater) => {
                        thread::sleep(Duration::from_micros(500))
                    }
                    MediaCodecResult::Info(i) => info!("Decoder dequeue event: {i:?}"),
                    MediaCodecResult::Err(e) => {
                        error!("Decoder dequeue error: {e}");

                        // lessen logcat flood (just in case)
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }

            // Destroy all resources
            decoder_enqueuer.lock().take(); // Make sure the shared ref is deleted first
            decoder.stop().unwrap();
            drop(decoder);

            image_queue.lock().clear();
            error!("FIXME: Leaking Imagereader!");
            Box::leak(Box::new(image_reader));
        }
    });

    // Make sure the decoder is ready: we don't want to try to enqueue frame and lose them, to avoid
    // image corruption.
    {
        let mut decoder_lock = decoder_enqueuer.lock();

        if decoder_lock.is_none() {
            // No spurious wakeups
            decoder_ready_notifier.wait(&mut decoder_lock);
        }
    }

    let enqueuer = VideoDecoderEnqueuer {
        inner: decoder_enqueuer,
    };
    let dequeuer = VideoDecoderDequeuer {
        running,
        dequeue_thread: Some(dequeue_thread),
        image_queue,
        config,
        buffering_running_average: 0.0,
    };

    Ok((enqueuer, dequeuer))
}
