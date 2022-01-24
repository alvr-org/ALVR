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
    top_level_path: [c_char; 32],
    value: f32, // [0, 1]
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
    bool: bool,
    float: f32,
    int32: i32,
    uint64: u64,
    vector3: AlvrVec3,
    double: f64,
    string: [c_char; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrOpenvrProp {
    name: [c_char; 64],
    ty: AlvrOpenvrPropType,
    value: AlvrOpenvrPropValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union AlvrInputButtonValue {
    bool: bool,
    float: f32,
}

// the profile is implied
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrInputButton {
    path: [c_char; 64],
    value: AlvrInputButtonValue,
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
    pub top_level_path: [c_char; 32],
    pub data: AlvrMotionData,
    pub timestamp_ns: u64, // client reference
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrViewInput {
    pub view_index: u32,
    pub orientation: AlvrQuat,
    pub position: AlvrVec3,
    pub fov: AlvrFov,
    pub timestamp_ns: u64, // client reference
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrHandSkeleton {
    pub top_level_path: [c_char; 32],
    pub joints: [AlvrMotionData; 25],
    pub timestamp_ns: u64, // client reference
}

#[repr(u8)]
pub enum AlvrEventType {
    None,
    ClientConnected,
    ClientDisconnected,
    VideoConfigUpdated,
    BatteryUpdated,
    OpenvrPropertyChanged,
    ButtonUpdated,
    DevicePoseUpdated, // HMD pose will never be reported. Use ViewInputUpdated
    ViewInputUpdated,
    HandsSkeletonUpdated,
    RestartRequested,
    ShutdownRequested,
}

#[repr(C)]
pub union AlvrEventData {
    none: (),
    video_config: AlvrVideoConfig,
    battery: AlvrBatteryValue,
    openvr_prop: AlvrOpenvrProp,
    button: AlvrInputButton,
    device_pose: AlvrDevicePose,
    view_input: AlvrViewInput,
    hand_skeleton: AlvrHandSkeleton, // this field is way oversized. todo: workaround
}

#[repr(C)]
pub struct AlvrEvent {
    ty: AlvrEventType,
    data: AlvrEventData,
}

#[repr(C)]
pub struct AlvrDisplayConfig {
    pub presentation: bool,
    pub config: AlvrVideoConfig,
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
pub struct AlvrDeviceProfile {
    pub top_level_path: [c_char; 32],
    pub interaction_profile: [c_char; 64],
    pub serial_number: [c_char; 64],
}

#[no_mangle]
pub extern "C" fn alvr_read_event(timeout_ns: u64) -> AlvrEvent {
    AlvrEvent {
        ty: AlvrEventType::None,
        data: AlvrEventData { none: () },
    }
}

// Use config == null to get the number of devices
#[no_mangle]
pub extern "C" fn alvr_get_available_devices_profiles(
    device_profiles: *mut *const AlvrDeviceProfile,
) -> usize {
    // /user/head
    // /user/hand/left
    // /user/hand/right
    // /user/gamepad
    // /user/treadmill
    // /user/eyes_ext
    // /user/vive_tracker_htcx/role/X
    todo!()
}

#[no_mangle]
pub extern "C" fn alvr_get_display_config() -> AlvrDisplayConfig {
    AlvrDisplayConfig {
        presentation: true, // false for tracker only
        config: todo!(),
    }
}

// use props == null to get the number of properties
#[no_mangle]
pub extern "C" fn alvr_get_static_openvr_properties(
    top_level_path: *const c_char,
    props: *mut AlvrOpenvrProp,
) -> usize {
    0
}

// returns the id of the swapchain
#[no_mangle]
pub extern "C" fn alvr_create_swapchain(
    images_count: u64,
    width: u32,
    height: u32,
    format: u32,
    sample_count: u32,
    handle: bool,          // create handle to DXGI resource, ignored on Linux
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
pub extern "C" fn alvr_wait_for_vsync() {}

#[no_mangle]
pub extern "C" fn alvr_present_layers(layers: *mut [AlvrLayer; 2], layers_count: usize) {}

#[no_mangle]
pub extern "C" fn alvr_send_haptics(
    top_level_path: *const c_char,
    duration_ns: u64,
    frequency: f32,
    amplitude: f32,
) {
}
