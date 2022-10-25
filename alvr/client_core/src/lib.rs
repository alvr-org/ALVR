#![allow(
    non_upper_case_globals,
    non_snake_case,
    clippy::missing_safety_doc,
    clippy::unseparated_literal_suffix
)]

mod connection;
mod decoder;
mod logging_backend;
mod platform;
mod sockets;
mod statistics;
mod storage;

#[cfg(target_os = "android")]
mod audio;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{
    glam::{Quat, UVec2, Vec2, Vec3},
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    prelude::*,
    RelaxedAtomic,
};
use alvr_events::ButtonValue;
use alvr_sockets::{
    BatteryPacket, ClientControlPacket, ClientStatistics, DeviceMotion, Fov, Tracking, ViewsConfig,
};
use decoder::EXTERNAL_DECODER;
use statistics::StatisticsManager;
use std::{
    collections::VecDeque,
    ffi::{c_char, c_void, CStr},
    ptr, slice,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use storage::Config;
use tokio::{sync::mpsc, sync::Notify};

static STATISTICS_MANAGER: Lazy<Mutex<Option<StatisticsManager>>> = Lazy::new(|| Mutex::new(None));

static TRACKING_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<Tracking>>>> =
    Lazy::new(|| Mutex::new(None));
static STATISTICS_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ClientStatistics>>>> =
    Lazy::new(|| Mutex::new(None));
static CONTROL_CHANNEL_SENDER: Lazy<Mutex<Option<mpsc::UnboundedSender<ClientControlPacket>>>> =
    Lazy::new(|| Mutex::new(None));
static DISCONNECT_NOTIFIER: Lazy<Notify> = Lazy::new(Notify::new);

static EVENT_QUEUE: Lazy<Mutex<VecDeque<AlvrEvent>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

static IS_ALIVE: RelaxedAtomic = RelaxedAtomic::new(true);
static IS_RESUMED: RelaxedAtomic = RelaxedAtomic::new(false);
static IS_STREAMING: RelaxedAtomic = RelaxedAtomic::new(false);

static CONNECTION_THREAD: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

#[repr(u8)]
pub enum AlvrCodec {
    H264,
    H265,
}

#[repr(u8)]
pub enum AlvrEvent {
    StreamingStarted {
        view_width: u32,
        view_height: u32,
        fps: f32,
        oculus_foveation_level: i32,
        dynamic_oculus_foveation: bool,
        extra_latency: bool,
        controller_prediction_multiplier: f32,
    },
    StreamingStopped,
    Haptics {
        device_id: u64,
        duration_s: f32,
        frequency: f32,
        amplitude: f32,
    },
    CreateDecoder {
        codec: AlvrCodec,
    },
    NalReady,
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
pub unsafe extern "C" fn alvr_log(level: AlvrLogLevel, message: *const c_char) {
    let message = CStr::from_ptr(message).to_str().unwrap();
    match level {
        AlvrLogLevel::Error => error!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Warn => warn!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Info => info!("[ALVR NATIVE] {message}"),
        AlvrLogLevel::Debug => debug!("[ALVR NATIVE] {message}"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_log_time(tag: *const c_char) {
    let tag = CStr::from_ptr(tag).to_str().unwrap();
    error!("[ALVR NATIVE] {tag}: {:?}", Instant::now());
}

/// On non-Android platforms, java_vm and constext should be null.
/// NB: context must be thread safe.
#[allow(unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn alvr_initialize(
    java_vm: *mut c_void,
    context: *mut c_void,
    recommended_view_width: u32,
    recommended_view_height: u32,
    refresh_rates: *const f32,
    refresh_rates_count: i32,
    external_decoder: bool,
) {
    #[cfg(target_os = "android")]
    ndk_context::initialize_android_context(java_vm, context);

    logging_backend::init_logging();

    createDecoder = Some(decoder::create_decoder);
    pushNal = Some(decoder::push_nal);

    // Make sure to reset config in case of version compat mismatch.
    if Config::load().protocol_id != alvr_common::protocol_id() {
        // NB: Config::default() sets the current protocol ID
        Config::default().store();
    }

    #[cfg(target_os = "android")]
    platform::try_get_microphone_permission();

    EXTERNAL_DECODER.set(external_decoder);

    let recommended_view_resolution = UVec2::new(recommended_view_width, recommended_view_height);

    let supported_refresh_rates =
        slice::from_raw_parts(refresh_rates, refresh_rates_count as _).to_vec();

    *CONNECTION_THREAD.lock() = Some(thread::spawn(move || {
        connection::connection_lifecycle_loop(recommended_view_resolution, supported_refresh_rates)
            .ok();
    }));
}

#[no_mangle]
pub extern "C" fn alvr_destroy() {
    IS_ALIVE.set(false);

    if let Some(thread) = CONNECTION_THREAD.lock().take() {
        thread.join().ok();
    }
}

#[no_mangle]
pub extern "C" fn alvr_resume() {
    IS_RESUMED.set(true);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_pause() {
    IS_RESUMED.set(false);
}

/// Returns true if there was a new event
#[no_mangle]
pub unsafe extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent) -> bool {
    if let Some(event) = EVENT_QUEUE.lock().pop_front() {
        *out_event = event;

        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_send_views_config(fov: *const EyeFov, ipd_m: f32) {
    let fov = slice::from_raw_parts(fov, 2);
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
pub unsafe extern "C" fn alvr_send_tracking(
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
        ptr::copy_nonoverlapping(
            device_motions,
            raw_motions.as_mut_ptr(),
            device_motions_count as _,
        );

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

/// Call only with external decoder
#[no_mangle]
pub extern "C" fn alvr_request_idr() {
    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
        sender.send(ClientControlPacket::RequestIdr).ok();
    }
}

/// Call only with external decoder
#[no_mangle]
pub extern "C" fn alvr_report_frame_decoded(timestamp_ns: u64) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        stats.report_frame_decoded(Duration::from_nanos(timestamp_ns as _));
    }
}

/// Call only with external decoder
#[no_mangle]
pub extern "C" fn alvr_report_compositor_start(timestamp_ns: u64) {
    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
        stats.report_compositor_start(Duration::from_nanos(timestamp_ns as _));
    }
}

/// Can be called before or after `alvr_initialize()`
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_initialize_opengl() {
    use crate::storage::{LOBBY_ROOM_BIN, LOBBY_ROOM_GLTF};

    LOBBY_ROOM_GLTF_PTR = LOBBY_ROOM_GLTF.as_ptr();
    LOBBY_ROOM_GLTF_LEN = LOBBY_ROOM_GLTF.len() as _;
    LOBBY_ROOM_BIN_PTR = LOBBY_ROOM_BIN.as_ptr();
    LOBBY_ROOM_BIN_LEN = LOBBY_ROOM_BIN.len() as _;

    initGraphicsNative();
}

/// Must be called after `alvr_destroy()`. Can be skipped if the GL context is destroyed before
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_destroy_opengl() {
    destroyGraphicsNative();
}

/// Must be called before `alvr_resume()`
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_resume_opengl(
    preferred_view_width: u32,
    preferred_view_height: u32,
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) {
    prepareLobbyRoom(
        preferred_view_width as _,
        preferred_view_height as _,
        swapchain_textures,
        swapchain_length,
    );
}

/// Must be called after `alvr_pause()`
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_pause_opengl() {
    destroyRenderers();
}

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_start_stream_opengl(
    swapchain_textures: *mut *const i32,
    swapchain_length: i32,
) {
    streamStartNative(swapchain_textures, swapchain_length);
}

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_render_lobby_opengl(
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

    renderLobbyNative(eye_inputs.as_ptr(), swapchain_indices);
}

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn alvr_render_stream_opengl(
    hardware_buffer: *mut c_void,
    swapchain_indices: *const i32,
) {
    renderStreamNative(hardware_buffer, swapchain_indices);
}
