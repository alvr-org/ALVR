#![allow(dead_code, unused_variables)]
#![allow(clippy::missing_safety_doc)]

use crate::{
    SESSION_MANAGER, ServerCoreContext, ServerCoreEvent, logging_backend, tracking::HandType,
};
use alvr_common::{
    AlvrCodecType, AlvrPose, AlvrViewParams, log,
    parking_lot::{Mutex, RwLock},
};
use alvr_packets::{ButtonEntry, ButtonValue, Haptics};
use alvr_session::CodecType;
use std::{
    collections::{HashMap, VecDeque},
    ffi::{CStr, CString, c_char},
    path::PathBuf,
    ptr,
    str::FromStr,
    sync::{LazyLock, mpsc},
    time::{Duration, Instant},
};

static SERVER_CORE_CONTEXT: RwLock<Option<ServerCoreContext>> = RwLock::new(None);
static EVENTS_RECEIVER: Mutex<Option<mpsc::Receiver<ServerCoreEvent>>> = Mutex::new(None);
static BUTTONS_QUEUE: Mutex<VecDeque<Vec<ButtonEntry>>> = Mutex::new(VecDeque::new());

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
    LocalViewParams([AlvrViewParams; 2]), // In relation to head
    TrackingUpdated { sample_timestamp_ns: u64 },
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
#[unsafe(no_mangle)]
pub extern "C" fn alvr_get_time_ns() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

// The libalvr user is responsible of interpreting values and calling functions using
// device/input/output identifiers obtained using this function
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_path_to_id(path_string: *const c_char) -> u64 {
    alvr_common::hash_string(unsafe { CStr::from_ptr(path_string) }.to_str().unwrap())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_error(string_ptr: *const c_char) {
    alvr_common::show_e(unsafe { CStr::from_ptr(string_ptr) }.to_string_lossy());
}

pub unsafe fn log(level: log::Level, string_ptr: *const c_char) {
    log::log!(
        level,
        "{}",
        unsafe { CStr::from_ptr(string_ptr) }.to_string_lossy()
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_warn(string_ptr: *const c_char) {
    unsafe { log(log::Level::Warn, string_ptr) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_info(string_ptr: *const c_char) {
    unsafe { log(log::Level::Info, string_ptr) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_dbg_server_impl(string_ptr: *const c_char) {
    alvr_common::dbg_server_impl!(
        "{}",
        unsafe { CStr::from_ptr(string_ptr) }.to_string_lossy()
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_dbg_encoder(string_ptr: *const c_char) {
    alvr_common::dbg_encoder!(
        "{}",
        unsafe { CStr::from_ptr(string_ptr) }.to_string_lossy()
    );
}

// Should not be used in production
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_log_periodically(tag_ptr: *const c_char, message_ptr: *const c_char) {
    const INTERVAL: Duration = Duration::from_secs(1);
    static LASTEST_TAG_TIMESTAMPS: LazyLock<Mutex<HashMap<String, Instant>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let tag = unsafe { CStr::from_ptr(tag_ptr) }.to_string_lossy();
    let message = unsafe { CStr::from_ptr(message_ptr) }.to_string_lossy();

    let mut timestamps_ref = LASTEST_TAG_TIMESTAMPS.lock();
    let old_timestamp = timestamps_ref
        .entry(tag.to_string())
        .or_insert_with(Instant::now);
    if *old_timestamp + INTERVAL < Instant::now() {
        *old_timestamp += INTERVAL;

        log::warn!("{}: {}", tag, message);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_get_settings_json(buffer: *mut c_char) -> u64 {
    string_to_c_str(buffer, &serde_json::to_string(&crate::settings()).unwrap())
}

/// This must be called before alvr_initialize()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_initialize_environment(
    config_dir: *const c_char,
    log_dir: *const c_char,
) {
    let config_dir =
        PathBuf::from_str(unsafe { CStr::from_ptr(config_dir) }.to_str().unwrap()).unwrap();
    let log_dir = PathBuf::from_str(unsafe { CStr::from_ptr(log_dir) }.to_str().unwrap()).unwrap();

    crate::initialize_environment(alvr_filesystem::Layout {
        config_dir,
        log_dir,
        ..Default::default()
    });
}

/// Either session_log_path or crash_log_path can be null, in which case log is outputted to
/// stdout/stderr on Windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_initialize_logging(
    session_log_path: *const c_char,
    crash_log_path: *const c_char,
) {
    let session_log_path = (!session_log_path.is_null()).then(|| {
        PathBuf::from_str(
            unsafe { CStr::from_ptr(session_log_path) }
                .to_str()
                .unwrap(),
        )
        .unwrap()
    });
    let crash_log_path = (!crash_log_path.is_null()).then(|| {
        PathBuf::from_str(unsafe { CStr::from_ptr(crash_log_path) }.to_str().unwrap()).unwrap()
    });

    logging_backend::init_logging(session_log_path, crash_log_path);
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_initialize() -> AlvrTargetConfig {
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

#[unsafe(no_mangle)]
pub extern "C" fn alvr_start_connection() {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.start_connection();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent, timeout_ns: u64) -> bool {
    if let Some(receiver) = &*EVENTS_RECEIVER.lock()
        && let Ok(event) = receiver.recv_timeout(Duration::from_nanos(timeout_ns))
    {
        match event {
            ServerCoreEvent::ClientConnected => unsafe {
                *out_event = AlvrEvent::ClientConnected;
            },
            ServerCoreEvent::ClientDisconnected => unsafe {
                *out_event = AlvrEvent::ClientDisconnected;
            },
            ServerCoreEvent::Battery(battery) => unsafe {
                *out_event = AlvrEvent::Battery(AlvrBatteryInfo {
                    device_id: battery.device_id,
                    gauge_value: battery.gauge_value,
                    is_plugged: battery.is_plugged,
                });
            },
            ServerCoreEvent::PlayspaceSync(bounds) => unsafe {
                *out_event = AlvrEvent::PlayspaceSync(bounds.to_array())
            },
            ServerCoreEvent::LocalViewParams(config) => unsafe {
                *out_event = AlvrEvent::LocalViewParams([
                    alvr_common::to_capi_view_params(&config[0]),
                    alvr_common::to_capi_view_params(&config[1]),
                ])
            },
            ServerCoreEvent::Tracking { poll_timestamp } => unsafe {
                *out_event = AlvrEvent::TrackingUpdated {
                    sample_timestamp_ns: poll_timestamp.as_nanos() as u64,
                };
            },
            ServerCoreEvent::Buttons(entries) => {
                BUTTONS_QUEUE.lock().push_back(entries);
                unsafe { *out_event = AlvrEvent::ButtonsUpdated };
            }
            ServerCoreEvent::RequestIDR => unsafe { *out_event = AlvrEvent::RequestIDR },
            ServerCoreEvent::CaptureFrame => unsafe { *out_event = AlvrEvent::CaptureFrame },
            ServerCoreEvent::RestartPending => unsafe {
                *out_event = AlvrEvent::RestartPending;
            },
            ServerCoreEvent::ShutdownPending => unsafe {
                *out_event = AlvrEvent::ShutdownPending;
            },
            ServerCoreEvent::GameRenderLatencyFeedback(_)
            | ServerCoreEvent::SetOpenvrProperty { .. } => {} // implementation not needed
        }

        true
    } else {
        false
    }
}

/// Returns false if there is no tracking sample for the requested sample timestamp
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_get_device_motion(
    device_id: u64,
    sample_timestamp_ns: u64,
    out_motion: *mut AlvrDeviceMotion,
) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read()
        && let Some(motion) =
            context.get_device_motion(device_id, Duration::from_nanos(sample_timestamp_ns))
    {
        unsafe {
            *out_motion = AlvrDeviceMotion {
                pose: alvr_common::to_capi_pose(&motion.pose),
                linear_velocity: motion.linear_velocity.to_array(),
                angular_velocity: motion.angular_velocity.to_array(),
            };
        }

        true
    } else {
        false
    }
}

/// out_skeleton must be an array of length 26
/// Returns false if there is no tracking sample for the requested sample timestamp
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_get_hand_skeleton(
    hand_type: AlvrHandType,
    sample_timestamp_ns: u64,
    out_skeleton: *mut AlvrPose,
) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read()
        && let Some(skeleton) = context.get_hand_skeleton(
            match hand_type {
                AlvrHandType::Left => HandType::Left,
                AlvrHandType::Right => HandType::Right,
            },
            Duration::from_nanos(sample_timestamp_ns),
        )
    {
        for (i, joint_pose) in skeleton.iter().enumerate() {
            unsafe { *out_skeleton.add(i) = alvr_common::to_capi_pose(joint_pose) };
        }

        true
    } else {
        false
    }
}

/// Call with null out_entries to get the buffer length
/// call with non-null out_entries to get the buttons and advanced the internal queue
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_get_buttons(out_entries: *mut AlvrButtonEntry) -> u64 {
    let entries_count = BUTTONS_QUEUE.lock().front().map_or(0, |e| e.len()) as u64;

    if out_entries.is_null() {
        return entries_count;
    }

    if let Some(button_entries) = BUTTONS_QUEUE.lock().pop_front() {
        for (i, entry) in button_entries.into_iter().enumerate() {
            let out_entry = unsafe { &mut *out_entries.add(i) };
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

#[unsafe(no_mangle)]
pub extern "C" fn alvr_send_haptics(
    device_id: u64,
    duration_s: f32,
    frequency: f32,
    amplitude: f32,
) {
    if let Ok(duration) = Duration::try_from_secs_f32(duration_s)
        && let Some(context) = &*SERVER_CORE_CONTEXT.read()
    {
        context.send_haptics(Haptics {
            device_id,
            duration,
            frequency,
            amplitude,
        });
    }
}

#[unsafe(no_mangle)]
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

    unsafe { ptr::copy_nonoverlapping(buffer_ptr, config_buffer.as_mut_ptr(), len as usize) };

    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.set_video_config_nals(config_buffer, codec);
    }
}

/// global_view_params must be an array of length 2
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_send_video_nal(
    timestamp_ns: u64,
    global_view_params: *const AlvrViewParams,
    is_idr: bool,
    buffer_ptr: *mut u8,
    len: i32,
) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        let buffer = unsafe { std::slice::from_raw_parts(buffer_ptr, len as usize) };

        let global_view_params = unsafe {
            [
                alvr_common::from_capi_view_params(&(*global_view_params)),
                alvr_common::from_capi_view_params(&(*global_view_params.add(1))),
            ]
        };

        context.send_video_nal(
            Duration::from_nanos(timestamp_ns),
            global_view_params,
            is_idr,
            buffer.to_vec(),
        );
    }
}

/// Returns true if updated
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_get_dynamic_encoder_params(
    out_params: *mut AlvrDynamicEncoderParams,
) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read()
        && let Some(params) = context.get_dynamic_encoder_params()
    {
        unsafe {
            (*out_params).bitrate_bps = params.bitrate_bps;
            (*out_params).framerate = params.framerate;
        }

        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_report_composed(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_composed(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_report_present(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_present(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

/// Retr  un true if a valid value is provided
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alvr_duration_until_next_vsync(out_ns: *mut u64) -> bool {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read()
        && let Some(duration) = context.duration_until_next_vsync()
    {
        unsafe { *out_ns = duration.as_nanos() as u64 };

        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_restart() {
    if let Some(context) = SERVER_CORE_CONTEXT.write().take() {
        context.restart();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alvr_shutdown() {
    SERVER_CORE_CONTEXT.write().take();
}
