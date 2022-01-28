use std::{ffi::c_void, os::raw::c_char};

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

#[repr(C)]
pub struct AlvrVec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
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

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum AlvrOpenvrPropType {
    Bool,
    Float,
    Int32,
    Uint64,
    Vector3,
    Double,
    String,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union AlvrOpenvrPropValue {
    pub bool_: bool,
    pub float_: f32,
    pub int32: i32,
    pub uint64: u64,
    pub vector3: AlvrVec3,
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

#[repr(C)]
#[derive(Clone, Copy)]
pub enum AlvrHandType {
    Left,
    Right,
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
    pub serial_number: [c_char; 64],
}

#[repr(u8)]
pub enum AlvrEventType {
    None,
    DeviceConnected,
    DeviceDisconnected,
    OpenvrPropertyChanged,
    VideoConfigUpdated, // Updated only once per hmd connection
    ViewsConfigUpdated, // Can be updated multiple times but not every frame
    DevicePoseUpdated,
    ButtonUpdated,
    HandSkeletonUpdated,
    BatteryUpdated,
    RestartRequested,
    ShutdownRequested,
}

#[repr(C)]
pub union AlvrEventData {
    pub none: (),
    pub device_profile: AlvrDeviceProfile,
    pub top_level_path: u64,
    pub openvr_prop: AlvrOpenvrProp,
    pub video_config: AlvrVideoConfig,
    pub views_config: AlvrViewsConfig,
    pub device_pose: AlvrDevicePose,
    pub button: AlvrButtonInput,
    pub hand_skeleton: AlvrHandSkeleton, // this field is way oversized. todo: workaround
    pub battery: AlvrBatteryValue,
}

#[repr(C)]
pub struct AlvrEvent {
    pub ty: AlvrEventType,
    pub data: AlvrEventData,
}

#[repr(C)]
pub struct AlvrLayerView {
    pub orientation: AlvrQuat,
    pub fov: AlvrFov,
    pub swapchain_id: u64,
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
/// Returns true is success
#[no_mangle]
pub extern "C" fn alvr_initialize(graphics_handles: AlvrGraphicsContext) -> bool {
    false
}

/// Destroy ALVR runtime
#[no_mangle]
pub extern "C" fn alvr_shutdown() {}

/// Purpose: make interface more efficient by using integers instead of strings for IDs
/// Note: inverse function not provided. match with a map
#[no_mangle]
pub extern "C" fn alvr_path_string_to_hash(path: *const c_char) -> u64 {
    0
}

#[no_mangle]
pub extern "C" fn alvr_read_event(timeout_ms: u64) -> AlvrEvent {
    AlvrEvent {
        ty: AlvrEventType::None,
        data: AlvrEventData { none: () },
    }
}

/// Use props == null to get the number of properties
#[no_mangle]
pub extern "C" fn alvr_get_static_openvr_properties(
    top_level_path: u64,
    props: *mut AlvrOpenvrProp,
) -> u64 {
    0
}

/// Returns the id of the swapchain. image handles obtained from `textures`. `textures` can be
/// Already initialized (from the Vulkan layer)
#[no_mangle]
pub extern "C" fn alvr_create_swapchain(
    images_count: u64,
    width: u32,
    height: u32,
    format: u32,
    sample_count: u32,
    dxgi_handle: bool,     // create HANDLEs to DXGI resource, ignored on Linux.
    textures: *mut c_void, // array of size images_count
) -> u64 {
    0
}

#[no_mangle]
pub extern "C" fn alvr_destroy_swapchain(id: u64) {}

#[no_mangle]
pub extern "C" fn alvr_swapchain_get_next_index(swapchain_id: u64) -> u32 {
    0
}

/// This function is used both to set the framerate and apply phase sync
#[no_mangle]
pub extern "C" fn alvr_wait_for_vsync(timeout_ms: u64) {}

#[no_mangle]
pub extern "C" fn alvr_present_layers(layers: *mut AlvrLayer, layers_count: u64) {}

#[no_mangle]
pub extern "C" fn alvr_send_haptics(path: u64, duration_ns: u64, frequency: f32, amplitude: f32) {}

/// Note: this is highly discouraged. Should be used only with OpenVR to set poseTimeOffset for pose
/// submission
#[no_mangle]
pub extern "C" fn alvr_get_best_effort_client_time_ns(top_level_path: u64) -> u64 {
    0
}

#[no_mangle]
pub extern "C" fn show_error(message: *const c_char) {}

#[no_mangle]
pub extern "C" fn log_error(message: *const c_char) {}

#[no_mangle]
pub extern "C" fn log_warning(message: *const c_char) {}

#[no_mangle]
pub extern "C" fn log_info(message: *const c_char) {}

#[no_mangle]
pub extern "C" fn log_debug(message: *const c_char) {}
