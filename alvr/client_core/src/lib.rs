#![allow(non_upper_case_globals, non_snake_case, clippy::missing_safety_doc)]

mod connection;
mod connection_utils;
mod logging_backend;
mod platform;
mod statistics;
mod storage;

#[cfg(target_os = "android")]
mod audio;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use crate::{
    platform::DecoderDequeuedData,
    storage::{LOBBY_ROOM_BIN, LOBBY_ROOM_GLTF},
};
use alvr_audio::{AudioDevice, AudioDeviceType};
use alvr_common::{
    glam::{Quat, UVec2, Vec2, Vec3},
    once_cell::sync::{Lazy, OnceCell},
    parking_lot::Mutex,
    prelude::*,
    RelaxedAtomic, ALVR_VERSION,
};
use alvr_events::ButtonValue;
use alvr_session::{AudioDeviceId, CodecType, MediacodecDataType};
use alvr_sockets::{
    BatteryPacket, ClientControlPacket, ClientStatistics, DeviceMotion, Fov, HeadsetInfoPacket,
    Tracking, ViewsConfig,
};
use jni::{
    objects::{GlobalRef, ReleaseMode},
    sys::jobject,
};
use platform::{VideoDecoderDequeuer, VideoDecoderEnqueuer};
use statistics::StatisticsManager;
use std::{
    collections::VecDeque,
    ffi::{c_void, CStr},
    os::raw::c_char,
    ptr, slice,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};
use storage::Config;
use tokio::{
    runtime::{self, Runtime},
    sync::mpsc,
    sync::Notify,
};

static STATISTICS_MANAGER: Lazy<Mutex<Option<StatisticsManager>>> = Lazy::new(|| Mutex::new(None));

static RUNTIME: Lazy<Mutex<Option<Runtime>>> = Lazy::new(|| Mutex::new(None));
static TRACKING_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<Tracking>>>> =
    Lazy::new(|| Mutex::new(None));
static STATISTICS_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ClientStatistics>>>> =
    Lazy::new(|| Mutex::new(None));
static CONTROL_CHANNEL_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ClientControlPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static DISCONNECT_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);
static ON_DESTROY_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);

static PREFERRED_RESOLUTION: Lazy<Mutex<UVec2>> = Lazy::new(|| Mutex::new(UVec2::ZERO));

static EVENT_BUFFER: Lazy<Mutex<VecDeque<AlvrEvent>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

static LAST_ENQUEUED_TIMESTAMPS: Lazy<Mutex<VecDeque<Duration>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

pub struct DecoderInitConfig {
    codec: CodecType,
    options: Vec<(String, MediacodecDataType)>,
}

pub static DECODER_INIT_CONFIG: Lazy<Mutex<DecoderInitConfig>> = Lazy::new(|| {
    Mutex::new(DecoderInitConfig {
        codec: CodecType::H264,
        options: vec![],
    })
});
static DECODER_ENQUEUER: Lazy<Mutex<Option<VideoDecoderEnqueuer>>> = Lazy::new(|| Mutex::new(None));
static DECODER_DEQUEUER: Lazy<Mutex<Option<VideoDecoderDequeuer>>> = Lazy::new(|| Mutex::new(None));

static IS_RESUMED: Lazy<RelaxedAtomic> = Lazy::new(|| RelaxedAtomic::new(false));

#[repr(u8)]
pub enum AlvrEvent {
    Haptics {
        device_id: u64,
        duration_s: f32,
        frequency: f32,
        amplitude: f32,
    },
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EyeFov {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AlvrQuat {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct AlvrDeviceMotion {
    device_id: u64,
    orientation: AlvrQuat,
    position: [f32; 3],
    linear_velocity: [f32; 3],
    angular_velocity: [f32; 3],
}

#[repr(C)]
pub struct AlvrEyeInput {
    orientation: AlvrQuat,
    position: [f32; 3],
    fov: EyeFov,
}

#[repr(C)]
pub struct OculusHand {
    enabled: bool,
    bone_rotations: [AlvrQuat; 19],
}

#[repr(C)]
pub enum AlvrButtonValue {
    Binary(bool),
    Scalar(f32),
}

#[repr(C)]
pub enum AlvrLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[no_mangle]
pub unsafe extern "C" fn alvr_path_string_to_hash(path: *const c_char) -> u64 {
    alvr_common::hash_string(CStr::from_ptr(path).to_str().unwrap())
}

#[no_mangle]
pub extern "C" fn alvr_log(level: AlvrLogLevel, message: *const c_char) {
    let message = unsafe { CStr::from_ptr(message) }.to_str().unwrap();
    match level {
        AlvrLogLevel::Error => error!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Warn => warn!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Info => info!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Debug => debug!("[ALVR NATIVE] {message}"),
    }
}

#[no_mangle]
pub extern "C" fn alvr_log_time(tag: *const c_char) {
    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap();
    error!("[ALVR NATIVE] {tag}: {:?}", Instant::now());
}

// NB: context must be thread safe.
#[no_mangle]
pub extern "C" fn alvr_initialize(
    java_vm: *mut c_void,
    context: *mut c_void,
    recommended_view_width: u32,
    recommended_view_height: u32,
    refresh_rates: *const f32,
    refresh_rates_count: i32,
) {
    unsafe { ndk_context::initialize_android_context(java_vm, context) };
    logging_backend::init_logging();

    extern "C" fn video_error_report_send() {
        if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
            sender.send(ClientControlPacket::VideoErrorReport).ok();
        }
    }

    extern "C" fn create_decoder(buffer: *const c_char, length: i32) {
        if DECODER_ENQUEUER.lock().is_none() {
            let mut csd_0 = vec![0; length as _];
            unsafe { ptr::copy_nonoverlapping(buffer, csd_0.as_mut_ptr(), length as _) };

            let config = DECODER_INIT_CONFIG.lock();

            let (enqueuer, dequeuer) =
                platform::video_decoder_split(config.codec, &csd_0, &config.options).unwrap();

            *DECODER_ENQUEUER.lock() = Some(enqueuer);
            *DECODER_DEQUEUER.lock() = Some(dequeuer);

            if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
                sender.send(ClientControlPacket::RequestIdr).ok();
            }
        }
    }

    extern "C" fn push_nal(buffer: *const c_char, length: i32, frame_index: u64) {
        if let Some(decoder) = &*DECODER_ENQUEUER.lock() {
            let timestamp = Duration::from_nanos(frame_index);

            {
                let mut timestamps_lock = LAST_ENQUEUED_TIMESTAMPS.lock();

                timestamps_lock.push_back(timestamp);
                if timestamps_lock.len() > 20 {
                    timestamps_lock.pop_front();
                }
            }

            // todo: check is copy can be avoided
            let mut nal_buffer = vec![0; length as _];
            unsafe { ptr::copy_nonoverlapping(buffer, nal_buffer.as_mut_ptr(), length as _) }

            show_err(decoder.push_frame_nal(timestamp, &nal_buffer, Duration::from_millis(500)));
        } else if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
            sender.send(ClientControlPacket::RequestIdr).ok();
        }
    }

    unsafe {
        pathStringToHash = Some(alvr_path_string_to_hash);
        videoErrorReportSend = Some(video_error_report_send);
        createDecoder = Some(create_decoder);
        pushNal = Some(push_nal);

        LOBBY_ROOM_GLTF_PTR = LOBBY_ROOM_GLTF.as_ptr();
        LOBBY_ROOM_GLTF_LEN = LOBBY_ROOM_GLTF.len() as _;
        LOBBY_ROOM_BIN_PTR = LOBBY_ROOM_BIN.as_ptr();
        LOBBY_ROOM_BIN_LEN = LOBBY_ROOM_BIN.len() as _;
    }

    // Make sure to reset config in case of version compat mismatch.
    if Config::load().protocol_id != alvr_common::protocol_id() {
        // NB: Config::default() sets the current protocol ID
        Config::default().store();
    }

    platform::try_get_microphone_permission();

    unsafe { initNative() };

    *PREFERRED_RESOLUTION.lock() = UVec2::new(recommended_view_width, recommended_view_height);

    let available_refresh_rates =
        unsafe { slice::from_raw_parts(refresh_rates, refresh_rates_count as _).to_vec() };
    let preferred_refresh_rate = available_refresh_rates.last().cloned().unwrap_or(60_f32);

    let microphone_sample_rate =
        AudioDevice::new(None, &AudioDeviceId::Default, AudioDeviceType::Input)
            .unwrap()
            .input_sample_rate()
            .unwrap();

    let headset_info = HeadsetInfoPacket {
        recommended_eye_width: recommended_view_width as _,
        recommended_eye_height: recommended_view_height as _,
        available_refresh_rates,
        preferred_refresh_rate,
        microphone_sample_rate,
        reserved: format!("{}", *ALVR_VERSION),
    };

    let runtime = Runtime::new().unwrap();

    runtime.spawn(async move {
        let connection_loop = connection::connection_lifecycle_loop(headset_info);

        tokio::select! {
            _ = connection_loop => (),
            _ = ON_DESTROY_NOTIFIER.notified() => ()
        };
    });

    *RUNTIME.lock() = Some(runtime);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_destroy() {
    ON_DESTROY_NOTIFIER.notify_waiters();

    // shutdown and wait for tasks to finish
    drop(RUNTIME.lock().take());

    destroyNative();
}

#[no_mangle]
pub unsafe extern "C" fn alvr_resume(swapchain_textures: *mut *const i32, swapchain_length: i32) {
    let config = Config::load();

    let resolution = *PREFERRED_RESOLUTION.lock();

    prepareLoadingRoom(
        resolution.x as _,
        resolution.y as _,
        config.dark_mode,
        swapchain_textures,
        swapchain_length,
    );

    IS_RESUMED.set(true);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_pause() {
    IS_RESUMED.set(false);

    destroyRenderers();
}

#[no_mangle]
pub unsafe extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent) -> bool {
    if let Some(event) = EVENT_BUFFER.lock().pop_front() {
        *out_event = event;

        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_start_stream(
    codec: i32,
    real_time: bool,
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) {
    streamStartNative(swapchain_textures, swapchain_length);
}

#[no_mangle]
pub extern "C" fn alvr_send_views_config(fov: *const EyeFov, ipd_m: f32) {
    let fov = unsafe { slice::from_raw_parts(fov, 2) };
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender
            .send(ClientControlPacket::ViewsConfig(ViewsConfig {
                fov: [
                    Fov {
                        left: fov[0].left,
                        right: fov[0].right,
                        top: fov[0].top,
                        bottom: fov[0].bottom,
                    },
                    Fov {
                        left: fov[1].left,
                        right: fov[1].right,
                        top: fov[1].top,
                        bottom: fov[1].bottom,
                    },
                ],
                ipd_m,
            }))
            .ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_battery(device_id: u64, gauge_value: f32, is_plugged: bool) {
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender
            .send(ClientControlPacket::Battery(BatteryPacket {
                device_id,
                gauge_value,
                is_plugged,
            }))
            .ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_playspace(width: f32, height: f32) {
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender
            .send(ClientControlPacket::PlayspaceSync(Vec2::new(width, height)))
            .ok();
    }
}

/// Returns frame timestamp in nanoseconds
#[no_mangle]
pub unsafe extern "C" fn alvr_wait_for_frame(buffer: *mut *mut c_void) -> i64 {
    let timestamp = if let Some(decoder) = &*DECODER_DEQUEUER.lock() {
        // Note on frame pacing: sometines there could be late frames stored inside the decoder,
        // which are gradually drained by polling two frames per frame. But sometimes a frame could
        // be received earlier than usual because of network jitter. In this case, if we polled the
        // second frame immediately, the next frame would probably be late. To mitigate this
        // scenario, a 5ms delay measurement is used to decide if to poll the second frame or not.
        // todo: remove the 5ms "magic number" and implement proper phase sync measuring network
        // jitter variance.
        let start_instant = Instant::now();
        match decoder.dequeue_frame(Duration::from_millis(50), Duration::from_millis(100)) {
            Ok(DecoderDequeuedData::Frame {
                buffer_ptr,
                timestamp,
            }) => {
                if Instant::now() - start_instant < Duration::from_millis(5) {
                    info!("Try draining extra decoder frame");
                    match decoder
                        .dequeue_frame(Duration::from_micros(1), Duration::from_millis(100))
                    {
                        Ok(DecoderDequeuedData::Frame {
                            buffer_ptr,
                            timestamp,
                        }) => {
                            *buffer = buffer_ptr;
                            Some(timestamp)
                        }
                        Ok(_) => {
                            // Note: data from first dequeue!
                            *buffer = buffer_ptr;
                            Some(timestamp)
                        }
                        Err(e) => {
                            error!("Error while decoder dequeue (2nd time): {e}");
                            DISCONNECT_NOTIFIER.notify_waiters();

                            None
                        }
                    }
                } else {
                    *buffer = buffer_ptr;
                    Some(timestamp)
                }
            }
            Ok(data) => {
                info!("Decoder: no frame dequeued. {data:?}");

                None
            }
            Err(e) => {
                error!("Error while decoder dequeue: {e}");
                DISCONNECT_NOTIFIER.notify_waiters();

                None
            }
        }
    } else {
        thread::sleep(Duration::from_millis(5));
        None
    };

    if let Some(timestamp) = timestamp {
        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            stats.report_frame_decoded(timestamp);
        }

        if !LAST_ENQUEUED_TIMESTAMPS.lock().contains(&timestamp) {
            error!("Detected late decoder, disconnecting...");
            DISCONNECT_NOTIFIER.notify_waiters();
        }

        timestamp.as_nanos() as i64
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_lobby(
    eye_inputs: *const AlvrEyeInput,
    swapchain_indices: *const i32,
) {
    let eye_inputs = [
        {
            let o = (*eye_inputs).orientation;
            let f = (*eye_inputs).fov;
            EyeInput {
                orientation: [o.x, o.y, o.z, o.w],
                position: (*eye_inputs).position,
                fovLeft: f.left,
                fovRight: f.right,
                fovTop: f.top,
                fovBottom: f.bottom,
            }
        },
        {
            let o = (*eye_inputs.offset(1)).orientation;
            let f = (*eye_inputs.offset(1)).fov;
            EyeInput {
                orientation: [o.x, o.y, o.z, o.w],
                position: (*eye_inputs.offset(1)).position,
                fovLeft: f.left,
                fovRight: f.right,
                fovTop: f.top,
                fovBottom: f.bottom,
            }
        },
    ];

    renderLoadingNative(eye_inputs.as_ptr(), swapchain_indices);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_stream(
    swapchain_indices: *const i32,
    hardware_buffer: *mut c_void,
) {
    renderNative(swapchain_indices, hardware_buffer);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_is_streaming() -> bool {
    isConnectedNative()
}

#[no_mangle]
pub extern "C" fn alvr_send_button(path_id: u64, value: AlvrButtonValue) {
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender
            .send(ClientControlPacket::Button {
                path_id,
                value: match value {
                    AlvrButtonValue::Binary(value) => ButtonValue::Binary(value),
                    AlvrButtonValue::Scalar(value) => ButtonValue::Scalar(value),
                },
            })
            .ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_tracking(
    target_timestamp_ns: u64,
    device_motions: *const AlvrDeviceMotion,
    device_motions_count: u64,
    left_oculus_hand: OculusHand,
    right_oculus_hand: OculusHand,
) {
    fn from_tracking_quat(quat: AlvrQuat) -> Quat {
        Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w)
    }

    fn from_oculus_hand(hand: OculusHand) -> Option<[Quat; 19]> {
        hand.enabled.then(|| {
            let vec = hand
                .bone_rotations
                .iter()
                .cloned()
                .map(from_tracking_quat)
                .collect::<Vec<_>>();

            let mut array = [Quat::IDENTITY; 19];
            array.copy_from_slice(&vec);

            array
        })
    }

    if let Some(sender) = &*TRACKING_SENDER.lock() {
        let mut raw_motions = vec![AlvrDeviceMotion::default(); device_motions_count as _];
        unsafe {
            ptr::copy_nonoverlapping(
                device_motions,
                raw_motions.as_mut_ptr(),
                device_motions_count as _,
            )
        };

        let device_motions = raw_motions
            .into_iter()
            .map(|motion| {
                (
                    motion.device_id,
                    DeviceMotion {
                        orientation: from_tracking_quat(motion.orientation),
                        position: Vec3::from_slice(&motion.position),
                        linear_velocity: Vec3::from_slice(&motion.linear_velocity),
                        angular_velocity: Vec3::from_slice(&motion.angular_velocity),
                    },
                )
            })
            .collect::<Vec<_>>();

        let input = Tracking {
            target_timestamp: Duration::from_nanos(target_timestamp_ns),
            device_motions,
            left_hand_skeleton: from_oculus_hand(left_oculus_hand),
            right_hand_skeleton: from_oculus_hand(right_oculus_hand),
        };

        sender.send(input).ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_get_prediction_offset_ns() -> u64 {
    if let Some(stats) = &*STATISTICS_MANAGER.lock() {
        stats.average_total_pipeline_latency().as_nanos() as _
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn alvr_report_submit(target_timestamp_ns: u64, vsync_queue_ns: u64) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        let timestamp = Duration::from_nanos(target_timestamp_ns);
        stats.report_submit(timestamp, Duration::from_nanos(vsync_queue_ns));

        if let Some(sender) = &*STATISTICS_SENDER.lock() {
            if let Some(stats) = stats.summary(timestamp) {
                sender.send(stats).ok();
            } else {
                error!("Statistics summary not ready!");
            }
        }
    }
}
