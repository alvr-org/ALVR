#![allow(clippy::missing_safety_doc)]

use crate::{connection, SESSION_MANAGER};
use alvr_common::{
    glam::{Quat, Vec3},
    lazy_static, log, Haptics, HEAD_ID, HEAD_PATH, LEFT_HAND_ID, LEFT_HAND_PATH, RIGHT_HAND_ID,
    RIGHT_HAND_PATH,
};
use alvr_session::OpenvrPropValue;
use alvr_sockets::{TimeSyncPacket, VideoFrameHeaderPacket};
use parking_lot::Mutex;
use std::{
    cmp,
    collections::HashMap,
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
    ptr, slice,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

lazy_static! {
    static ref DRIVER_EVENT_RECEIVER: Arc<Mutex<Option<mpsc::Receiver<AlvrEvent>>>> =
        Arc::new(Mutex::new(None));
    pub static ref DRIVER_EVENT_SENDER: Arc<Mutex<Option<mpsc::Sender<AlvrEvent>>>> =
        Arc::new(Mutex::new(None));
    static ref FRAME_TIME: Arc<Mutex<Duration>> =
        Arc::new(Mutex::new(Duration::from_secs_f32(1.0 / 72.0)));
    static ref LAST_VSYNC: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrFov {
    /// Negative, radians
    pub left: f32,
    /// Positive, radians
    pub right: f32,
    /// Positive, radians
    pub top: f32,
    /// Negative, radians
    pub bottom: f32,
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
pub struct AlvrVec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AlvrVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrVideoConfig {
    pub preferred_view_width: u32,
    pub preferred_view_height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrBatteryValue {
    pub top_level_path: u64,
    pub value: f32, // [0, 1]
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
pub enum AlvrOpenvrPropType {
    ALVR_OPENVR_PROP_TYPE_BOOL,
    ALVR_OPENVR_PROP_TYPE_FLOAT,
    ALVR_OPENVR_PROP_TYPE_INT32,
    ALVR_OPENVR_PROP_TYPE_UINT64,
    ALVR_OPENVR_PROP_TYPE_VECTOR3,
    ALVR_OPENVR_PROP_TYPE_DOUBLE,
    ALVR_OPENVR_PROP_TYPE_STRING,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union AlvrOpenvrPropValue {
    pub bool_: bool,
    pub float_: f32,
    pub int32: i32,
    pub uint64: u64,
    pub vector3: [f32; 3],
    pub double_: f64,
    pub string: [c_char; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrOpenvrProp {
    pub name: [c_char; 64],
    pub ty: AlvrOpenvrPropType,
    pub value: AlvrOpenvrPropValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrOpenvrDeviceProp {
    pub top_level_path: u64,
    pub prop: AlvrOpenvrProp,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union AlvrButtonInputValue {
    pub bool_: bool,
    pub float_: f32,
}

// the profile is implied
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrButtonInput {
    pub path: u64,
    pub value: AlvrButtonInputValue,
    pub timestamp_ns: u64, // client reference
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrMotionData {
    pub orientation: AlvrQuat,
    pub position: AlvrVec3,
    pub linear_velocity: AlvrVec3,
    pub angular_velocity: AlvrVec3,
    pub has_velocity: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrDevicePose {
    pub top_level_path: u64,
    pub data: AlvrMotionData,
    pub timestamp_ns: u64, // client reference
}

// for now ALVR expects only two eye views. OpenVR supports only 2 and OpenXR supports more than 2
// only through extensions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrViewsConfig {
    pub ipd_m: f32,
    pub fov: [AlvrFov; 2],
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
pub enum AlvrHandType {
    ALVR_HAND_TYPE_LEFT,
    ALVR_HAND_TYPE_RIGHT,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrHandSkeleton {
    pub hand_type: AlvrHandType,
    pub joints: [AlvrMotionData; 25],
    pub timestamp_ns: u64, // client reference
}

// /user/head
// /user/hand/left
// /user/hand/right
// /user/gamepad
// /user/treadmill
// /user/eyes_ext
// /user/vive_tracker_htcx/role/X
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrDeviceProfile {
    pub top_level_path: u64,
    pub interaction_profile: u64,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub enum AlvrEventType {
    ALVR_EVENT_TYPE_NONE,
    ALVR_EVENT_TYPE_DEVICE_CONNECTED,
    ALVR_EVENT_TYPE_DEVICE_DISCONNECTED,
    ALVR_EVENT_TYPE_OPENVR_PROPERTY_CHANGED,
    ALVR_EVENT_TYPE_VIDEO_CONFIG_UPDATED, // Updated only once per hmd connection
    ALVR_EVENT_TYPE_VIEWS_CONFIG_UPDATED, // Can be updated multiple times but not every frame
    ALVR_EVENT_TYPE_DEVICE_POSE_UPDATED,
    ALVR_EVENT_TYPE_BUTTON_UPDATED,
    ALVR_EVENT_TYPE_HAND_SKELETON_UPDATED,
    ALVR_EVENT_TYPE_BATTERY_UPDATED,
    ALVR_EVENT_TYPE_BOUNDS_UPDATED,
    ALVR_EVENT_TYPE_RESTART_REQUESTED,
    ALVR_EVENT_TYPE_SHUTDOWN_REQUESTED,
}

#[repr(C)]
pub union AlvrEventData {
    pub none: (),
    pub device_profile: AlvrDeviceProfile,
    pub top_level_path: u64,
    pub openvr_prop: AlvrOpenvrDeviceProp,
    pub video_config: AlvrVideoConfig,
    pub views_config: AlvrViewsConfig,
    pub device_pose: AlvrDevicePose,
    pub button: AlvrButtonInput,
    pub hand_skeleton: AlvrHandSkeleton, // this field is way oversized. todo: workaround
    pub battery: AlvrBatteryValue,
    pub bounds_rect: AlvrVec2,
}

#[repr(C)]
pub struct AlvrEvent {
    pub ty: AlvrEventType,
    pub data: AlvrEventData,
}

#[repr(C)]
pub struct AlvrLayerView {
    pub texture_id: u64,
    pub orientation: AlvrQuat,
    pub fov: AlvrFov,
    pub rect_offset: AlvrVec2,
    pub rect_size: AlvrVec2,
}

#[repr(C)]
pub struct AlvrLayer {
    pub views: [AlvrLayerView; 2],
}

#[repr(C)]
pub struct AlvrGraphicsContext {
    pub vk_get_device_proc_addr: *mut c_void,
    pub vk_instance: u64,
    pub vk_physical_device: u64,
    pub vk_device: u64,
    pub vk_queue_family_index: u64,
    pub vk_queue_index: u64,
}

/// Initialize ALVR runtime and create the graphics context
/// For OpenVR/Windows use vk_get_device_proc_addr == null
/// Returns true if success
#[no_mangle]
pub unsafe extern "C" fn alvr_initialize(
    graphics_handles: AlvrGraphicsContext,
    rendering_statistics_callback: Option<unsafe extern "C" fn(*mut f32, *mut f32, *mut f32)>, // render_ms, idle_ms, wait_ms
) -> bool {
    // graphics_handles is ignored for now. todo: create GraphicsContext

    unsafe extern "C" fn log_error(string_ptr: *const c_char) {
        alvr_common::show_e(CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe fn log(level: log::Level, string_ptr: *const c_char) {
        log::log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe extern "C" fn log_warn(string_ptr: *const c_char) {
        log(log::Level::Warn, string_ptr);
    }

    unsafe extern "C" fn log_info(string_ptr: *const c_char) {
        log(log::Level::Info, string_ptr);
    }

    unsafe extern "C" fn log_debug(string_ptr: *const c_char) {
        log(log::Level::Debug, string_ptr);
    }

    extern "C" fn video_send(header: crate::VideoFrame, buffer_ptr: *mut u8, len: i32) {
        if let Some(sender) = &*crate::VIDEO_SENDER.lock() {
            let header = VideoFrameHeaderPacket {
                packet_counter: header.packetCounter,
                tracking_frame_index: header.trackingFrameIndex,
                video_frame_index: header.videoFrameIndex,
                sent_time: header.sentTime,
                frame_byte_size: header.frameByteSize,
                fec_index: header.fecIndex,
                fec_percentage: header.fecPercentage,
            };

            let mut vec_buffer = vec![0; len as _];

            // use copy_nonoverlapping (aka memcpy) to avoid freeing memory allocated by C++
            unsafe {
                ptr::copy_nonoverlapping(buffer_ptr, vec_buffer.as_mut_ptr(), len as _);
            }

            sender.send((header, vec_buffer)).ok();
        }
    }

    extern "C" fn haptics_send(haptics: crate::HapticsFeedback) {}

    extern "C" fn time_sync_send(data: crate::TimeSync) {
        if let Some(sender) = &*crate::TIME_SYNC_SENDER.lock() {
            let time_sync = TimeSyncPacket {
                mode: data.mode,
                server_time: data.serverTime,
                client_time: data.clientTime,
                packets_lost_total: data.packetsLostTotal,
                packets_lost_in_second: data.packetsLostInSecond,
                average_send_latency: data.averageSendLatency,
                average_transport_latency: data.averageTransportLatency,
                average_decode_latency: data.averageDecodeLatency,
                idle_time: data.idleTime,
                fec_failure: data.fecFailure,
                fec_failure_in_second: data.fecFailureInSecond,
                fec_failure_total: data.fecFailureTotal,
                fps: data.fps,
                server_total_latency: data.serverTotalLatency,
                tracking_recv_frame_index: data.trackingRecvFrameIndex,
            };

            sender.send(time_sync).ok();
        }
    }

    pub extern "C" fn driver_ready_idle(set_default_chap: bool) {}

    extern "C" fn _shutdown_runtime() {}

    crate::FRAME_RENDER_VS_CSO_PTR = crate::FRAME_RENDER_VS_CSO.as_ptr();
    crate::FRAME_RENDER_VS_CSO_LEN = crate::FRAME_RENDER_VS_CSO.len() as _;
    crate::FRAME_RENDER_PS_CSO_PTR = crate::FRAME_RENDER_PS_CSO.as_ptr();
    crate::FRAME_RENDER_PS_CSO_LEN = crate::FRAME_RENDER_PS_CSO.len() as _;
    crate::QUAD_SHADER_CSO_PTR = crate::QUAD_SHADER_CSO.as_ptr();
    crate::QUAD_SHADER_CSO_LEN = crate::QUAD_SHADER_CSO.len() as _;
    crate::COMPRESS_AXIS_ALIGNED_CSO_PTR = crate::COMPRESS_AXIS_ALIGNED_CSO.as_ptr();
    crate::COMPRESS_AXIS_ALIGNED_CSO_LEN = crate::COMPRESS_AXIS_ALIGNED_CSO.len() as _;
    crate::COLOR_CORRECTION_CSO_PTR = crate::COLOR_CORRECTION_CSO.as_ptr();
    crate::COLOR_CORRECTION_CSO_LEN = crate::COLOR_CORRECTION_CSO.len() as _;

    crate::LogError = Some(log_error);
    crate::LogWarn = Some(log_warn);
    crate::LogInfo = Some(log_info);
    crate::LogDebug = Some(log_debug);
    crate::DriverReadyIdle = Some(driver_ready_idle);
    crate::VideoSend = Some(video_send);
    crate::HapticsSend = Some(haptics_send);
    crate::TimeSyncSend = Some(time_sync_send);
    crate::ShutdownRuntime = Some(_shutdown_runtime);
    crate::RenderingStatistics = rendering_statistics_callback;

    crate::init();

    crate::CppInit();

    let (sender, receiver) = mpsc::channel();

    *DRIVER_EVENT_SENDER.lock() = Some(sender);
    *DRIVER_EVENT_RECEIVER.lock() = Some(receiver);

    alvr_common::show_err(alvr_commands::apply_driver_paths_backup(
        crate::FILESYSTEM_LAYOUT.openvr_driver_root_dir.clone(),
    ));

    if let Some(runtime) = &mut *crate::RUNTIME.lock() {
        runtime.spawn(async move {
            tokio::select! {
                _ = connection::connection_lifecycle_loop() => (),
                _ = crate::SHUTDOWN_NOTIFIER.notified() => (),
            }
        });
    }

    true
}

/// Destroy ALVR runtime
#[no_mangle]
pub extern "C" fn alvr_shutdown() {
    crate::shutdown_runtime();
}

/// Purpose: make interface more efficient by using integers instead of strings for IDs
/// Note: inverse function not provided. match with a map if necessary
#[no_mangle]
pub unsafe extern "C" fn alvr_path_string_to_hash(path: *const c_char) -> u64 {
    alvr_common::hash_string(CStr::from_ptr(path).to_str().unwrap())
}

#[no_mangle]
pub extern "C" fn alvr_read_event(timeout_ms: u64) -> AlvrEvent {
    DRIVER_EVENT_RECEIVER
        .lock()
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(timeout_ms))
        .unwrap_or(AlvrEvent {
            ty: AlvrEventType::ALVR_EVENT_TYPE_NONE,
            data: AlvrEventData { none: () },
        })
}

#[no_mangle]
pub unsafe extern "C" fn alvr_get_serial_number(
    top_level_path: u64,
    serial_number: *mut c_char,
    max_length: u64,
) {
    let session_manager = &*SESSION_MANAGER.lock();
    let settings = &session_manager.get().session_settings;
    let value = if top_level_path == *HEAD_ID {
        settings.headset.serial_number.clone()
    } else if top_level_path == *LEFT_HAND_ID {
        format!(
            "{}_Left",
            settings.headset.controllers.content.serial_number
        )
    } else if top_level_path == *RIGHT_HAND_ID {
        format!(
            "{}_Right",
            settings.headset.controllers.content.serial_number
        )
    } else {
        unreachable!()
    };

    let c_string = CString::new(value).unwrap();
    let length = cmp::min(c_string.as_bytes_with_nul().len(), max_length as usize);
    ptr::copy_nonoverlapping(c_string.as_ptr(), serial_number, length);
}

/// Use properties == null to get the number of properties
#[no_mangle]
pub unsafe extern "C" fn alvr_get_static_openvr_properties(
    top_level_path: u64,
    properties: *mut AlvrOpenvrProp,
) -> u64 {
    // todo: move to dashboard
    let session_settings = SESSION_MANAGER.lock().get().session_settings.clone();
    let mut props = HashMap::new();
    props.insert(
        HEAD_PATH.to_owned(),
        vec![
            (
                "Prop_TrackingSystemName_String".to_owned(),
                OpenvrPropValue::String(session_settings.headset.tracking_system_name),
            ),
            (
                "Prop_ModelNumber_String".into(),
                OpenvrPropValue::String(session_settings.headset.model_number),
            ),
            (
                "Prop_ManufacturerName_String".into(),
                OpenvrPropValue::String(session_settings.headset.manufacturer_name),
            ),
            (
                "Prop_RenderModelName_String".into(),
                OpenvrPropValue::String(session_settings.headset.render_model_name),
            ),
            (
                "Prop_RegisteredDeviceType_String".into(),
                OpenvrPropValue::String(session_settings.headset.registered_device_type),
            ),
            (
                "Prop_DriverVersion_String".into(),
                OpenvrPropValue::String(session_settings.headset.driver_version),
            ),
            (
                "Prop_SecondsFromVsyncToPhotons_Float".into(),
                OpenvrPropValue::Float(0.0),
            ),
            (
                "Prop_CurrentUniverseId_Uint64".into(),
                OpenvrPropValue::Uint64(session_settings.headset.universe_id),
            ),
            #[cfg(windows)]
            ("Prop_IsOnDesktop_Bool".into(), OpenvrPropValue::Bool(true)),
            #[cfg(windows)]
            (
                "Prop_DriverDirectModeSendsVsyncEvents_Bool".into(),
                OpenvrPropValue::Bool(false),
            ),
            (
                "Prop_DeviceProvidesBatteryStatus_Bool".into(),
                OpenvrPropValue::Bool(true),
            ),
            (
                "Prop_NamedIconPathDeviceOff_String".into(),
                OpenvrPropValue::String("{oculus}/icons/quest_headset_off.png".into()),
            ),
            (
                "Prop_NamedIconPathDeviceSearching_String".into(),
                OpenvrPropValue::String("{oculus}/icons/quest_headset_searching.gif".into()),
            ),
            (
                "Prop_NamedIconPathDeviceSearchingAlert_String".into(),
                OpenvrPropValue::String("{oculus}/icons/quest_headset_alert_searching.gif".into()),
            ),
            (
                "Prop_NamedIconPathDeviceReady_String".into(),
                OpenvrPropValue::String("{oculus}/icons/quest_headset_ready.png".into()),
            ),
            (
                "Prop_NamedIconPathDeviceReadyAlert_String".into(),
                OpenvrPropValue::String("{oculus}/icons/quest_headset_ready_alert.png".into()),
            ),
            (
                "Prop_NamedIconPathDeviceStandby_String".into(),
                OpenvrPropValue::String("{oculus}/icons/quest_headset_standby.png".into()),
            ),
        ],
    );
    props.insert(
        LEFT_HAND_PATH.into(),
        vec![
            (
                "Prop_TrackingSystemName_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .tracking_system_name
                        .clone(),
                ),
            ),
            (
                "Prop_ModelNumber_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .model_number
                        .clone(),
                ),
            ),
            (
                "Prop_ManufacturerName_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .manufacturer_name
                        .clone(),
                ),
            ),
            (
                "Prop_RenderModelName_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .render_model_name_left,
                ),
            ),
            (
                "Prop_RegisteredDeviceType_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .registered_device_type
                        .clone(),
                ),
            ),
            // Prop_SerialNumber_String
            // Prop_AttachedDeviceId_String
            (
                "Prop_DeviceProvidesBatteryStatus_Bool".into(),
                OpenvrPropValue::Bool(true),
            ),
            (
                "Prop_ControllerType_String".into(),
                OpenvrPropValue::String(
                    session_settings.headset.controllers.content.ctrl_type_left,
                ),
            ),
            (
                "Prop_ControllerType_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .input_profile_path
                        .clone(),
                ),
            ),
        ],
    );
    props.insert(
        RIGHT_HAND_PATH.into(),
        vec![
            (
                "Prop_TrackingSystemName_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .tracking_system_name,
                ),
            ),
            (
                "Prop_ModelNumber_String".into(),
                OpenvrPropValue::String(session_settings.headset.controllers.content.model_number),
            ),
            (
                "Prop_ManufacturerName_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .manufacturer_name,
                ),
            ),
            (
                "Prop_RenderModelName_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .render_model_name_right,
                ),
            ),
            (
                "Prop_RegisteredDeviceType_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .registered_device_type,
                ),
            ),
            // Prop_SerialNumber_String
            // Prop_AttachedDeviceId_String
            (
                "Prop_DeviceProvidesBatteryStatus_Bool".into(),
                OpenvrPropValue::Bool(true),
            ),
            (
                "Prop_ControllerType_String".into(),
                OpenvrPropValue::String(
                    session_settings.headset.controllers.content.ctrl_type_right,
                ),
            ),
            (
                "Prop_ControllerType_String".into(),
                OpenvrPropValue::String(
                    session_settings
                        .headset
                        .controllers
                        .content
                        .input_profile_path,
                ),
            ),
        ],
    );

    let props = if top_level_path == *HEAD_ID {
        props[HEAD_PATH].clone()
    } else if top_level_path == *LEFT_HAND_ID {
        props[LEFT_HAND_PATH].clone()
    } else if top_level_path == *RIGHT_HAND_ID {
        props[RIGHT_HAND_PATH].clone()
    } else {
        log::warn!("unsupported device");
        vec![]
    };

    let prop_count = props.len();

    if !properties.is_null() {
        let properties = slice::from_raw_parts_mut(properties, prop_count);
        for idx in 0..prop_count {
            let (name, value) = props[idx].clone();

            let c_string = CString::new(name).unwrap();
            let mut name = [0; 64];

            ptr::copy_nonoverlapping(
                c_string.as_ptr(),
                name.as_mut_ptr(),
                c_string.as_bytes_with_nul().len(),
            );

            let ty = match &value {
                OpenvrPropValue::Bool(_) => AlvrOpenvrPropType::ALVR_OPENVR_PROP_TYPE_BOOL,
                OpenvrPropValue::Float(_) => AlvrOpenvrPropType::ALVR_OPENVR_PROP_TYPE_FLOAT,
                OpenvrPropValue::Int32(_) => AlvrOpenvrPropType::ALVR_OPENVR_PROP_TYPE_INT32,
                OpenvrPropValue::Uint64(_) => AlvrOpenvrPropType::ALVR_OPENVR_PROP_TYPE_UINT64,
                OpenvrPropValue::Vector3(_) => AlvrOpenvrPropType::ALVR_OPENVR_PROP_TYPE_VECTOR3,
                OpenvrPropValue::Double(_) => AlvrOpenvrPropType::ALVR_OPENVR_PROP_TYPE_DOUBLE,
                OpenvrPropValue::String(_) => AlvrOpenvrPropType::ALVR_OPENVR_PROP_TYPE_STRING,
            };

            let value = match value {
                OpenvrPropValue::Bool(bool_) => AlvrOpenvrPropValue { bool_ },
                OpenvrPropValue::Float(float_) => AlvrOpenvrPropValue { float_ },
                OpenvrPropValue::Int32(int32) => AlvrOpenvrPropValue { int32 },
                OpenvrPropValue::Uint64(uint64) => AlvrOpenvrPropValue { uint64 },
                OpenvrPropValue::Vector3(vector3) => AlvrOpenvrPropValue { vector3 },
                OpenvrPropValue::Double(double_) => AlvrOpenvrPropValue { double_ },
                OpenvrPropValue::String(value) => {
                    let c_string = CString::new(value).unwrap();
                    let mut string = [0; 64];

                    ptr::copy_nonoverlapping(
                        c_string.as_ptr(),
                        string.as_mut_ptr(),
                        c_string.as_bytes_with_nul().len(),
                    );

                    AlvrOpenvrPropValue { string }
                }
            };

            properties[idx] = AlvrOpenvrProp { name, ty, value };
        }
    }

    prop_count as _
}

/// Returns the id of the texture. image handle obtained from `texture`. `texture` can be already
/// initialized (from the Vulkan layer)
#[no_mangle]
pub unsafe extern "C" fn alvr_create_texture(
    width: u32,
    height: u32,
    format: u32,
    sample_count: u32,
    dxgi_handle: bool,    // create HANDLEs to DXGI resource, ignored on Linux.
    texture: *mut c_void, // array of size images_count
) -> u64 {
    crate::CreateTexture(width, height, format, sample_count, texture)
}

#[no_mangle]
pub unsafe extern "C" fn alvr_destroy_texture(id: u64) {
    crate::DestroyTexture(id);
}

/// This function is used both to set the framerate and apply phase sync
#[no_mangle]
pub extern "C" fn alvr_wait_for_vsync(timeout_ms: u64) {
    // naive implementation. todo: phase sync

    let last_vsync_ref = &mut *LAST_VSYNC.lock();

    let frame_time = *FRAME_TIME.lock();

    let now = Instant::now();

    thread::sleep(Duration::min(
        (*last_vsync_ref + frame_time).saturating_duration_since(now),
        Duration::from_millis(timeout_ms),
    ));

    *last_vsync_ref += frame_time;
}

/// syncTexture should be ignored on linux
#[no_mangle]
pub unsafe extern "C" fn alvr_present_layers(
    sync_texture: *mut c_void,
    layers: *const AlvrLayer,
    layers_count: u64,
    target_timestamp_ns: u64,
) {
    let layers = slice::from_raw_parts(layers, layers_count as _)
        .iter()
        .map(|layer| {
            let left_view = crate::LayerView {
                texture_id: layer.views[0].texture_id,
                orientation: crate::TrackingQuat {
                    w: layer.views[0].orientation.w,
                    x: layer.views[0].orientation.x,
                    y: layer.views[0].orientation.y,
                    z: layer.views[0].orientation.z,
                },
                rect_offset: crate::TrackingVector2 {
                    x: layer.views[0].rect_offset.x,
                    y: layer.views[0].rect_offset.y,
                },
                rect_size: crate::TrackingVector2 {
                    x: layer.views[0].rect_size.x,
                    y: layer.views[0].rect_size.y,
                },
            };
            let right_view = crate::LayerView {
                texture_id: layer.views[1].texture_id,
                orientation: crate::TrackingQuat {
                    w: layer.views[1].orientation.w,
                    x: layer.views[1].orientation.x,
                    y: layer.views[1].orientation.y,
                    z: layer.views[1].orientation.z,
                },
                rect_offset: crate::TrackingVector2 {
                    x: layer.views[1].rect_offset.x,
                    y: layer.views[1].rect_offset.y,
                },
                rect_size: crate::TrackingVector2 {
                    x: layer.views[1].rect_size.x,
                    y: layer.views[1].rect_size.y,
                },
            };

            crate::Layer {
                views: [left_view, right_view],
            }
        })
        .collect::<Vec<_>>();

    crate::PresentLayers(sync_texture, layers.as_ptr(), layers_count);
}

#[no_mangle]
pub extern "C" fn alvr_send_haptics(path: u64, duration_s: f32, frequency: f32, amplitude: f32) {
    if let Some(sender) = &*crate::HAPTICS_SENDER.lock() {
        let haptics = Haptics {
            path,
            duration: Duration::from_secs_f32(duration_s),
            frequency,
            amplitude,
        };

        sender.send(haptics).ok();
    }
}

/// Note: this is highly discouraged. Should be used only with OpenVR to set poseTimeOffset for pose
/// submission
#[no_mangle]
pub extern "C" fn alvr_get_best_effort_client_time_ns(top_level_path: u64) -> u64 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn alvr_popup_error(message: *const c_char) {
    alvr_common::show_e(CStr::from_ptr(message).to_string_lossy());
}

#[no_mangle]
pub unsafe extern "C" fn alvr_error(message: *const c_char) {
    log(log::Level::Error, message);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_warning(message: *const c_char) {
    log(log::Level::Warn, message);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_info(message: *const c_char) {
    log(log::Level::Info, message);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_debug(message: *const c_char) {
    log(log::Level::Debug, message);
}

///////////////////////////////////////////////////////////////////////////////

unsafe fn log(level: log::Level, string_ptr: *const c_char) {
    log::log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
}

pub fn to_capi_quat(quat: Quat) -> AlvrQuat {
    AlvrQuat {
        x: quat.x,
        y: quat.y,
        z: quat.z,
        w: quat.w,
    }
}

pub fn to_capi_vec3(vec: Vec3) -> AlvrVec3 {
    AlvrVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}
