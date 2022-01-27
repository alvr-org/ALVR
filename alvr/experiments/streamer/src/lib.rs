use std::{ffi::c_void, os::raw::c_char};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrFov {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrQuat {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
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
    pub bool: bool,
    pub float: f32,
    pub int32: i32,
    pub uint64: u64,
    pub vector3: AlvrVec3,
    pub double: f64,
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
    pub bool: bool,
    pub float: f32,
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
pub struct AlvrViewsInfo {
    pub ipd_m: f32,
    pub fov: AlvrFov,
    pub timestamp_ns: u64, // client reference
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
pub struct AlvrDeviceProfile {
    pub top_level_path: u64,
    pub interaction_profile: u64,
}

#[repr(u8)]
pub enum AlvrEventType {
    None,
    DeviceConnected,
    DeviceDisconnected,
    VideoConfigUpdated,
    BatteryUpdated,
    OpenvrPropertyChanged,
    ButtonUpdated,
    DevicePoseUpdated,
    ViewInputUpdated,
    HandsSkeletonUpdated,
    RestartRequested,
    ShutdownRequested,
}

#[repr(C)]
pub union AlvrEventData {
    pub none: (),
    pub top_level_path: u64,
    pub video_config: AlvrVideoConfig,
    pub battery: AlvrBatteryValue,
    pub openvr_prop: AlvrOpenvrProp,
    pub button: AlvrButtonInput,
    pub device_pose: AlvrDevicePose,
    pub views_info: AlvrViewsInfo,
    pub hand_skeleton: AlvrHandSkeleton, // this field is way oversized. todo: workaround
}

#[repr(C)]
pub struct AlvrEvent {
    pub ty: AlvrEventType,
    pub data: AlvrEventData,
}

#[repr(C)]
pub struct AlvrLayer {
    pub orientation: AlvrQuat,
    pub fov: AlvrFov,
    pub swapchain_id: u64,
    pub rect_offset: AlvrVec2,
    pub rect_size: AlvrVec2,
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

// Initialize ALVR runtime and create the graphics context
// for OpenVR/Windows use vk_get_device_proc_addr == null
#[no_mangle]
pub extern "C" fn alvr_initialize(graphics_handles: AlvrGraphicsContext) {}

// Purpose: make interface more efficient by using integers instead of strings for IDs
// note: inverse function not provided. match with a map
#[no_mangle]
pub extern "C" fn alvr_path_string_to_hash(path: *const c_char) -> u64 {
    0
}

#[no_mangle]
pub extern "C" fn alvr_read_event(timeout_ns: u64) -> AlvrEvent {
    AlvrEvent {
        ty: AlvrEventType::None,
        data: AlvrEventData { none: () },
    }
}

// returns false if (virtual) HMD not connected
#[no_mangle]
pub extern "C" fn alvr_get_display_config(config: *mut AlvrVideoConfig) -> bool {
    false
}

// use props == null to get the number of properties
#[no_mangle]
pub extern "C" fn alvr_get_static_openvr_properties(
    top_level_path: u64,
    props: *mut AlvrOpenvrProp,
) -> usize {
    0
}

// returns the id of the swapchain. image handles obtained from `textures`. `textures` can be
// already initialized (from the Vulkan layer)
#[no_mangle]
pub extern "C" fn alvr_create_swapchain(
    images_count: usize,
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

// this function is used both to set the framerate and apply phase sync
#[no_mangle]
pub extern "C" fn alvr_wait_for_vsync(timeout_ns: u64) {}

#[no_mangle]
pub extern "C" fn alvr_present_layers(layers: *mut [AlvrLayer; 2], layers_count: usize) {}

#[no_mangle]
pub extern "C" fn alvr_send_haptics(path: u64, duration_ns: u64, frequency: f32, amplitude: f32) {}
