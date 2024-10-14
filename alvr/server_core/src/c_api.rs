#![allow(dead_code, unused_variables)]
#![allow(clippy::missing_safety_doc)]

use crate::{
    logging_backend, tracking::HandType, ServerCoreContext, ServerCoreEvent, SESSION_MANAGER,
};
use alvr_common::{
    log,
    once_cell::sync::Lazy,
    parking_lot::{Mutex, RwLock},
    Fov, Pose,
};
use alvr_packets::{ButtonEntry, ButtonValue, Haptics};
use alvr_session::CodecType;
use std::{
    collections::{HashMap, VecDeque},
    ffi::{c_char, CStr, CString},
    path::PathBuf,
    ptr,
    str::FromStr,
    sync::mpsc,
    time::{Duration, Instant},
};

static SERVER_CORE_CONTEXT: Lazy<RwLock<Option<ServerCoreContext>>> =
    Lazy::new(|| RwLock::new(None));
static EVENTS_RECEIVER: Lazy<Mutex<Option<mpsc::Receiver<ServerCoreEvent>>>> =
    Lazy::new(|| Mutex::new(None));
static BUTTONS_QUEUE: Lazy<Mutex<VecDeque<Vec<ButtonEntry>>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

#[repr(C)]
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

#[repr(u8)]
pub enum AlvrCodecType {
    H264 = 0,
    Hevc = 1,
    AV1 = 2,
}

#[repr(C)]
pub struct AlvrPose {
    orientation: AlvrQuat,
    position: [f32; 3],
}

#[repr(C)]
pub struct AlvrDeviceMotion {
    pub pose: AlvrPose,
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
}

#[repr(u8)]
pub enum AlvrHandType {
    Left = 0,
    Right = 1,
}

#[repr(C)]
pub union AlvrButtonValue {
    pub scalar: bool,
    pub float: f32,
}

// the profile is implied
#[repr(C)]
pub struct AlvrButtonEntry {
    pub id: u64,
    pub value: AlvrButtonValue,
}

#[repr(C)]
pub struct AlvrBatteryInfo {
    pub device_id: u64,
    /// range [0, 1]
    pub gauge_value: f32,
    pub is_plugged: bool,
}

#[repr(u8)]
pub enum AlvrEvent {
    ClientConnected,
    ClientDisconnected,
    Battery(AlvrBatteryInfo),
    PlayspaceSync([f32; 2]),
    ViewsConfig {
        local_view_transform: [AlvrPose; 2],
        fov: [AlvrFov; 2],
    },
    TrackingUpdated {
        sample_timestamp_ns: u64,
    },
    ButtonsUpdated,
    RequestIDR,
    CaptureFrame,
    RestartPending,
    ShutdownPending,
}

#[repr(C)]
pub struct AlvrTargetConfig {
    game_render_width: u32,
    game_render_height: u32,
    stream_width: u32,
    stream_height: u32,
}

#[repr(C)]
pub struct AlvrDeviceConfig {
    device_id: u64,
    interaction_profile_id: u64,
}

#[repr(C)]
pub struct AlvrDynamicEncoderParams {
    bitrate_bps: f32,
    framerate: f32,
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
pub unsafe extern "C" fn alvr_error(string_ptr: *const c_char) {
    alvr_common::show_e(CStr::from_ptr(string_ptr).to_string_lossy());
}

pub unsafe fn log(level: log::Level, string_ptr: *const c_char) {
    log::log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
}

#[no_mangle]
pub unsafe extern "C" fn alvr_warn(string_ptr: *const c_char) {
    log(log::Level::Warn, string_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_info(string_ptr: *const c_char) {
    log(log::Level::Info, string_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_dbg_server_impl(string_ptr: *const c_char) {
    alvr_common::dbg_server_impl!("{}", CStr::from_ptr(string_ptr).to_string_lossy());
}

#[no_mangle]
pub unsafe extern "C" fn alvr_dbg_encoder(string_ptr: *const c_char) {
    alvr_common::dbg_encoder!("{}", CStr::from_ptr(string_ptr).to_string_lossy());
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
    string_to_c_str(buffer, &serde_json::to_string(&crate::settings()).unwrap())
}

/// This must be called before alvr_initialize()
#[no_mangle]
pub unsafe extern "C" fn alvr_initialize_environment(
    config_dir: *const c_char,
    log_dir: *const c_char,
) {
    let config_dir = PathBuf::from_str(CStr::from_ptr(config_dir).to_str().unwrap()).unwrap();
    let log_dir = PathBuf::from_str(CStr::from_ptr(log_dir).to_str().unwrap()).unwrap();

    crate::initialize_environment(alvr_filesystem::Layout {
        config_dir,
        log_dir,
        ..Default::default()
    });
}

/// Either session_log_path or crash_log_path can be null, in which case log is outputted to
/// stdout/stderr on Windows.
#[no_mangle]
pub unsafe extern "C" fn alvr_initialize_logging(
    session_log_path: *const c_char,
    crash_log_path: *const c_char,
) {
    let session_log_path = (!session_log_path.is_null())
        .then(|| PathBuf::from_str(CStr::from_ptr(session_log_path).to_str().unwrap()).unwrap());
    let crash_log_path = (!crash_log_path.is_null())
        .then(|| PathBuf::from_str(CStr::from_ptr(crash_log_path).to_str().unwrap()).unwrap());

    logging_backend::init_logging(session_log_path, crash_log_path);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_initialize() -> AlvrTargetConfig {
    let (context, receiver) = ServerCoreContext::new();
    *SERVER_CORE_CONTEXT.write() = Some(context);
    *EVENTS_RECEIVER.lock() = Some(receiver);

    let session_manager_lock = SESSION_MANAGER.read();
    let restart_settings = &session_manager_lock.session().openvr_config;

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
pub unsafe extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent, timeout_ns: u64) -> bool {
    if let Some(receiver) = &*EVENTS_RECEIVER.lock() {
        if let Ok(event) = receiver.recv_timeout(Duration::from_nanos(timeout_ns)) {
            match event {
                ServerCoreEvent::SetOpenvrProperty { .. } => {} // implementation not needed
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
                ServerCoreEvent::Tracking { sample_timestamp } => {
                    *out_event = AlvrEvent::TrackingUpdated {
                        sample_timestamp_ns: sample_timestamp.as_nanos() as u64,
                    };
                }
                ServerCoreEvent::Buttons(entries) => {
                    BUTTONS_QUEUE.lock().push_back(entries);
                    *out_event = AlvrEvent::ButtonsUpdated;
                }
                ServerCoreEvent::RequestIDR => *out_event = AlvrEvent::RequestIDR,
                ServerCoreEvent::CaptureFrame => *out_event = AlvrEvent::CaptureFrame,
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

/// Returns false if there is no tracking sample for the requested sample timestamp
#[no_mangle]
pub unsafe extern "C" fn alvr_get_device_motion(
    device_id: u64,
    sample_timestamp_ns: u64,
    out_motion: *mut AlvrDeviceMotion,
) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        if let Some(motion) =
            context.get_device_motion(device_id, Duration::from_nanos(sample_timestamp_ns))
        {
            *out_motion = AlvrDeviceMotion {
                pose: pose_to_capi(&motion.pose),
                linear_velocity: motion.linear_velocity.to_array(),
                angular_velocity: motion.angular_velocity.to_array(),
            };

            true
        } else {
            false
        }
    } else {
        false
    }
}

/// out_skeleton must be an array of length 26
/// Returns false if there is no tracking sample for the requested sample timestamp
#[no_mangle]
pub unsafe extern "C" fn alvr_get_hand_skeleton(
    hand_type: AlvrHandType,
    sample_timestamp_ns: u64,
    out_skeleton: *mut AlvrPose,
) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        if let Some(skeleton) = context.get_hand_skeleton(
            match hand_type {
                AlvrHandType::Left => HandType::Left,
                AlvrHandType::Right => HandType::Right,
            },
            Duration::from_nanos(sample_timestamp_ns),
        ) {
            for (i, joint_pose) in skeleton.iter().enumerate() {
                (*out_skeleton.add(i)) = pose_to_capi(joint_pose);
            }

            true
        } else {
            false
        }
    } else {
        false
    }
}

/// Call with null out_entries to get the buffer length
/// call with non-null out_entries to get the buttons and advanced the internal queue
#[no_mangle]
pub unsafe extern "C" fn alvr_get_buttons(out_entries: *mut AlvrButtonEntry) -> u64 {
    let entries_count = BUTTONS_QUEUE.lock().front().map(|e| e.len()).unwrap_or(0) as u64;

    if out_entries.is_null() {
        return entries_count;
    }

    if let Some(button_entries) = BUTTONS_QUEUE.lock().pop_front() {
        for (i, entry) in button_entries.into_iter().enumerate() {
            let out_entry = &mut (*out_entries.add(i));
            out_entry.id = entry.path_id;
            match entry.value {
                ButtonValue::Binary(value) => out_entry.value.scalar = value,
                ButtonValue::Scalar(value) => out_entry.value.float = value,
            }
        }

        entries_count
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn alvr_send_haptics(
    device_id: u64,
    duration_s: f32,
    frequency: f32,
    amplitude: f32,
) {
    if let Ok(duration) = Duration::try_from_secs_f32(duration_s) {
        if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
            context.send_haptics(Haptics {
                device_id,
                duration,
                frequency,
                amplitude,
            });
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_set_video_config_nals(
    codec: AlvrCodecType,
    buffer_ptr: *const u8,
    len: i32,
) {
    let codec = match codec {
        AlvrCodecType::H264 => CodecType::H264,
        AlvrCodecType::Hevc => CodecType::Hevc,
        AlvrCodecType::AV1 => CodecType::AV1,
    };

    let mut config_buffer = vec![0; len as usize];

    ptr::copy_nonoverlapping(buffer_ptr, config_buffer.as_mut_ptr(), len as usize);

    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.set_video_config_nals(config_buffer, codec);
    }
}

#[no_mangle]
pub unsafe extern "C" fn alvr_send_video_nal(
    timestamp_ns: u64,
    buffer_ptr: *mut u8,
    len: i32,
    is_idr: bool,
) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        let buffer = std::slice::from_raw_parts(buffer_ptr, len as usize);
        context.send_video_nal(Duration::from_nanos(timestamp_ns), buffer.to_vec(), is_idr);
    }
}

/// Returns true if updated
#[no_mangle]
pub unsafe extern "C" fn alvr_get_dynamic_encoder_params(
    out_params: *mut AlvrDynamicEncoderParams,
) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        if let Some(params) = context.get_dynamic_encoder_params() {
            (*out_params).bitrate_bps = params.bitrate_bps;
            (*out_params).framerate = params.framerate;

            true
        } else {
            false
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn alvr_report_composed(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_composed(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

#[no_mangle]
pub extern "C" fn alvr_report_present(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_present(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

/// Retrun true if a valid value is provided
#[no_mangle]
pub unsafe extern "C" fn alvr_duration_until_next_vsync(out_ns: *mut u64) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        if let Some(duration) = context.duration_until_next_vsync() {
            *out_ns = duration.as_nanos() as u64;
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
