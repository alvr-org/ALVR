#![allow(dead_code, unused_variables)]

use crate::{logging_backend, ServerCoreContext, ServerCoreEvent, SERVER_DATA_MANAGER};
use alvr_common::{
    log,
    once_cell::sync::Lazy,
    parking_lot::{Mutex, RwLock},
    Fov, Pose,
};
use alvr_packets::Haptics;
use std::{
    collections::HashMap,
    ffi::{c_char, CStr, CString},
    ptr,
    time::{Duration, Instant},
};

static SERVER_CORE_CONTEXT: Lazy<RwLock<Option<ServerCoreContext>>> =
    Lazy::new(|| RwLock::new(None));

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrFov {
    /// Negative, radians
    pub left: f32,
    /// Positive, radians
    pub right: f32,
    /// Positive, radians
    pub up: f32,
    /// Negative, radians
    pub down: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
impl Default for AlvrQuat {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AlvrPose {
    orientation: AlvrQuat,
    position: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrSpaceRelation {
    pub pose: AlvrPose,
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub has_velocity: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrJoint {
    relation: AlvrSpaceRelation,
    radius: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrJointSet {
    values: [AlvrJoint; 26],
    global_hand_relation: AlvrSpaceRelation,
    is_active: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union AlvrInputValue {
    pub bool_: bool,
    pub float_: f32,
}

// the profile is implied
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrInput {
    pub id: u64,
    pub value: AlvrInputValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrBatteryInfo {
    pub device_id: u64,
    /// range [0, 1]
    pub gauge_value: f32,
    pub is_plugged: bool,
}

#[repr(C)]
pub enum AlvrEvent {
    ClientConnected,
    ClientDisconnected,
    Battery(AlvrBatteryInfo),
    PlayspaceSync([f32; 2]),
    ViewsConfig {
        local_view_transform: [AlvrPose; 2],
        fov: [AlvrFov; 2],
    },
    RequestIDR,
    RestartPending,
    ShutdownPending,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrTargetConfig {
    game_render_width: u32,
    game_render_height: u32,
    stream_width: u32,
    stream_height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrDeviceConfig {
    device_id: u64,
    interaction_profile_id: u64,
}

fn pose_to_capi(pose: &Pose) -> AlvrPose {
    AlvrPose {
        orientation: AlvrQuat {
            x: pose.orientation.x,
            y: pose.orientation.y,
            z: pose.orientation.z,
            w: pose.orientation.w,
        },
        position: pose.position.to_array(),
    }
}

fn fov_to_capi(fov: &Fov) -> AlvrFov {
    AlvrFov {
        left: fov.left,
        right: fov.right,
        up: fov.up,
        down: fov.down,
    }
}

fn string_to_c_str(buffer: *mut c_char, value: &str) -> u64 {
    let cstring = CString::new(value).unwrap();
    if !buffer.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(cstring.as_ptr(), buffer, cstring.as_bytes_with_nul().len());
        }
    }

    cstring.as_bytes_with_nul().len() as u64
}

// Get ALVR server time. The libalvr user should provide timestamps in the provided time frame of
// reference in the following functions
#[no_mangle]
pub unsafe extern "C" fn alvr_get_time_ns() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

// The libalvr user is responsible of interpreting values and calling functions using
// device/input/output identifiers obtained using this function
#[no_mangle]
pub unsafe extern "C" fn alvr_path_to_id(path_string: *const c_char) -> u64 {
    alvr_common::hash_string(CStr::from_ptr(path_string).to_str().unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn alvr_log_error(string_ptr: *const c_char) {
    alvr_common::show_e(CStr::from_ptr(string_ptr).to_string_lossy());
}

pub fn log(level: log::Level, string_ptr: *const c_char) {
    unsafe { log::log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy()) };
}

#[no_mangle]
pub extern "C" fn alvr_log_warn(string_ptr: *const c_char) {
    log(log::Level::Warn, string_ptr);
}

#[no_mangle]
pub extern "C" fn alvr_log_info(string_ptr: *const c_char) {
    log(log::Level::Info, string_ptr);
}

#[no_mangle]
pub extern "C" fn alvr_log_debug(string_ptr: *const c_char) {
    log(log::Level::Debug, string_ptr);
}

// Should not be used in production
#[no_mangle]
pub unsafe extern "C" fn alvr_log_periodically(tag_ptr: *const c_char, message_ptr: *const c_char) {
    const INTERVAL: Duration = Duration::from_secs(1);
    static LASTEST_TAG_TIMESTAMPS: Lazy<Mutex<HashMap<String, Instant>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));

    let tag = CStr::from_ptr(tag_ptr).to_string_lossy();
    let message = CStr::from_ptr(message_ptr).to_string_lossy();

    let mut timestamps_ref = LASTEST_TAG_TIMESTAMPS.lock();
    let old_timestamp = timestamps_ref
        .entry(tag.to_string())
        .or_insert_with(Instant::now);
    if *old_timestamp + INTERVAL < Instant::now() {
        *old_timestamp += INTERVAL;

        log::warn!("{}: {}", tag, message);
    }
}

#[no_mangle]
pub extern "C" fn alvr_get_settings_json(buffer: *mut c_char) -> u64 {
    string_to_c_str(
        buffer,
        &serde_json::to_string(&SERVER_DATA_MANAGER.read().settings()).unwrap(),
    )
}

#[no_mangle]
pub extern "C" fn alvr_initialize_logging() {
    logging_backend::init_logging();
}

#[no_mangle]
pub unsafe extern "C" fn alvr_initialize() -> AlvrTargetConfig {
    *SERVER_CORE_CONTEXT.write() = Some(ServerCoreContext::new());

    let data_manager_lock = SERVER_DATA_MANAGER.read();
    let restart_settings = &data_manager_lock.session().openvr_config;

    AlvrTargetConfig {
        game_render_width: restart_settings.target_eye_resolution_width,
        game_render_height: restart_settings.target_eye_resolution_height,
        stream_width: restart_settings.eye_resolution_width,
        stream_height: restart_settings.eye_resolution_height,
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_start_connection() {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.start_connection();
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        if let Some(event) = context.poll_event() {
            match event {
                ServerCoreEvent::ClientConnected => {
                    *out_event = AlvrEvent::ClientConnected;
                }
                ServerCoreEvent::ClientDisconnected => {
                    *out_event = AlvrEvent::ClientDisconnected;
                }
                ServerCoreEvent::Battery(battery) => {
                    *out_event = AlvrEvent::Battery(AlvrBatteryInfo {
                        device_id: battery.device_id,
                        gauge_value: battery.gauge_value,
                        is_plugged: battery.is_plugged,
                    });
                }
                ServerCoreEvent::PlayspaceSync(bounds) => {
                    *out_event = AlvrEvent::PlayspaceSync(bounds.to_array())
                }
                ServerCoreEvent::ViewsConfig(config) => {
                    *out_event = AlvrEvent::ViewsConfig {
                        local_view_transform: [
                            pose_to_capi(&config.local_view_transforms[0]),
                            pose_to_capi(&config.local_view_transforms[1]),
                        ],
                        fov: [fov_to_capi(&config.fov[0]), fov_to_capi(&config.fov[1])],
                    }
                }
                ServerCoreEvent::RequestIDR => *out_event = AlvrEvent::RequestIDR,
                ServerCoreEvent::GameRenderLatencyFeedback(_) => {} // implementation not needed
                ServerCoreEvent::RestartPending => {
                    *out_event = AlvrEvent::RestartPending;
                }
                ServerCoreEvent::ShutdownPending => {
                    *out_event = AlvrEvent::ShutdownPending;
                }
            }

            true
        } else {
            false
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_haptics(
    device_id: u64,
    duration_s: f32,
    frequency: f32,
    amplitude: f32,
) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.send_haptics(Haptics {
            device_id,
            duration: Duration::from_secs_f32(f32::max(duration_s, 0.0)),
            frequency,
            amplitude,
        });
    }
}

extern "C" fn report_composed(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_composed(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

extern "C" fn report_present(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_present(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

/// Retrun true if a valid value is provided
#[no_mangle]
pub extern "C" fn alvr_duration_until_next_vsync(out_ns: *mut u64) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        if let Some(duration) = context.duration_until_next_vsync() {
            unsafe { *out_ns = duration.as_nanos() as u64 };
            true
        } else {
            false
        }
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_restart() {
    if let Some(context) = SERVER_CORE_CONTEXT.write().take() {
        context.restart();
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_shutdown() {
    SERVER_CORE_CONTEXT.write().take();
}

// // Device API:

// // Use the two-call pattern to first get the array length then the array data.
// #[no_mangle]
// pub unsafe extern "C" fn alvr_get_devices(out_device_configs: *mut AlvrDeviceConfig) -> u64 {
//     todo!()
// }

// // After this call, previous button and tracking data is discarded
// #[no_mangle]
// pub unsafe extern "C" fn alvr_update_inputs(device_id: u64) {
//     todo!()
// }

// // Use the two-call pattern to first get the array length then the array data.
// // Data is updated after a call to alvr_update_inputs.
// #[no_mangle]
// pub unsafe extern "C" fn alvr_get_inputs(
//     device_id: u64,
//     out_inputs_arr: *mut AlvrInput,
//     out_timestamp_ns: u64,
// ) -> u64 {
//     todo!()
// }

// // pose_id is something like /user/hand/left/input/grip/pose
// #[no_mangle]
// pub unsafe extern "C" fn alvr_get_tracked_pose(
//     pose_id: u64,
//     timestamp_ns: u64,
//     out_relation: *mut AlvrSpaceRelation,
// ) {
//     todo!()
// }

// #[no_mangle]
// pub unsafe extern "C" fn alvr_get_hand_tracking(
//     device_id: u64,
//     timestamp_ns: u64,
//     out_joint_set: *mut AlvrJointSet,
// ) {
//     todo!()
// }

// #[no_mangle]
// pub unsafe extern "C" fn alvr_view_poses(
//     out_head_relation: *mut AlvrSpaceRelation,
//     out_fov_arr: *mut AlvrFov,            // 2 elements
//     out_relative_pose_arr: *mut AlvrPose, // 2 elements
// ) {
//     todo!()
// }

// #[no_mangle]
// pub unsafe extern "C" fn alvr_destroy_device(device_id: u64) {
//     todo!()
// }

// // Compositor target API:

// This should reflect the client current framerate
// #[no_mangle]
// pub unsafe extern "C" fn alvr_get_framerate() -> f32 {
//     todo!()
// }

// #[no_mangle]
// pub unsafe extern "C" fn alvr_pre_vulkan() {
//     todo!()
// }

// #[no_mangle]
// pub unsafe extern "C" fn alvr_post_vulkan() {
//     todo!()
// }

// #[no_mangle]
// pub unsafe extern "C" fn alvr_create_vk_target_swapchain(
//     width: u32,
//     height: u32,
//     vk_color_format: i32,
//     vk_color_space: i32,
//     vk_image_usage: u32,
//     vk_present_mode: i32,
//     image_count: u64,
// ) {
//     todo!()
// }

// // returns vkResult
// #[no_mangle]
// pub unsafe extern "C" fn alvr_acquire_image(out_swapchain_index: u64) -> i32 {
//     todo!()
// }

// // returns vkResult
// #[no_mangle]
// pub unsafe extern "C" fn alvr_present(
//     vk_queue: u64,
//     swapchain_index: u64,
//     timeline_semaphore_value: u64,
//     timestamp_ns: u64,
// ) -> i32 {
//     todo!()
// }

// #[no_mangle]
// pub unsafe extern "C" fn alvr_destroy_vk_target_swapchain() {
//     todo!()
// }
