use std::os::raw::c_char;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AFov {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AQuat {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
pub struct AVec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVideoConfig {
    pub preferred_view_width: u32,
    pub preferred_view_height: u32,
    pub suggegested_fov: [AFov; 2],
    pub initial_ipd_m: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ABatteryValue {
    device_index: u32,
    value: f32, // [0, 1]
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum AOpenvrPropType {
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
pub union AOpenvrPropValue {
    bool: bool,
    float: f32,
    int32: i32,
    uint64: u64,
    vector3: AVec3,
    double: f64,
    string: [c_char; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AOpenvrProp {
    name: [c_char; 64],
    ty: AOpenvrPropType,
    value: AOpenvrPropValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union AInputButtonValue {
    bool: bool,
    float: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AInputButton {
    path: [c_char; 64],
    value: AInputButtonValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AMotionData {
    pub orientation: AQuat,
    pub position: AVec3,
    pub linear_velocity: AVec3,
    pub angular_velocity: AVec3,
    pub has_velocity: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ATrackingInput {
    device_index: u32,
    data: AMotionData,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HandSkeleton {
    device_index: u32,
    joints: [AMotionData; 25],
}

#[repr(u8)]
pub enum AEventType {
    None,
    ClientConnected,
    ClientDisconnected,
    VideoConfigUpdated,
    BatteryUpdated,
    OpenvrPropertyChanged,
    ButtonUpdated,
    TrackingUpdated,
    HandsSkeletonUpdated,
    RestartRequested,
    ShutdownRequested,
}

#[repr(C)]
pub union AEventData {
    none: (),
    video_config: AVideoConfig,
    battery: ABatteryValue,
    openvr_prop: AOpenvrProp,
    button: AInputButton,
    tracking: ATrackingInput,
    hand_skeleton: HandSkeleton, // this field is way oversized. todo: workaround
}

#[repr(C)]
pub struct AEvent {
    ty: AEventType,
    data: AEventData,
}

#[repr(u32)]
pub enum ADeviceType {
    None,
    Hmd,
    LeftHand,
    RightHand,
    GenericTracker,
}

#[repr(C)]
pub struct ADeviceConfig {
    pub ty: ADeviceType,
    pub serial_number: [c_char; 64],
}

#[repr(C)]
pub struct ADisplayConfig {
    pub presentation: bool,
    pub config: AVideoConfig,
}

#[repr(C)]
pub enum AInputButtonType {
    Binary,
    NormalizedOneSided,
    NormalizedTwoSided,
}

#[repr(C)]
pub struct AInputButtonDef {
    path: [c_char; 64],
    ty: AInputButtonType,
}

#[repr(C)]
pub struct ALayer {
    pub orientation: AQuat,
    pub fov: AFov,
    pub swapchain_id: u64,
    pub rect_offset: AVec2,
    pub rect_size: AVec2,
}

pub extern "C" fn alvr_read_event(timeout_ns: u64) -> AEvent {
    AEvent {
        ty: AEventType::None,
        data: AEventData { none: () },
    }
}

// These are virtual devices. The number and type of deviced will not change.
pub extern "C" fn alvr_get_device_config(device_index: u32) -> ADeviceConfig {
    ADeviceConfig {
        ty: ADeviceType::None,
        serial_number: [0; 64],
    }
}

pub extern "C" fn alvr_get_display_config() -> ADisplayConfig {
    ADisplayConfig {
        presentation: true, // false for tracker only
        config: todo!(),
    }
}

// use props == null to get props_count
pub extern "C" fn alvr_get_static_openvr_properties(
    device_index: u32,
    props: *mut AOpenvrProp,
    props_count: *mut usize,
) {
}

// use props == null to get props_count
pub extern "C" fn alvr_get_button_layout(
    device_index: u32,
    layout: *mut AInputButtonDef,
    input_count: *mut usize,
) {
}

// returns the id of the swapchain
pub extern "C" fn alvr_create_swapchain(
    images_count: u64,
    width: u32,
    height: u32,
    format: u32,
    sample_count: u32,
    handle: bool,       // create handle to DXGI resource, ignored on Linux
    textures: *mut u64, // array of size images_count
) -> u64 {
    0
}

pub extern "C" fn alvr_destroy_swapchain(id: u64) {}

pub extern "C" fn alvr_swapchain_get_next_index(id: u64) -> u32 {
    0
}

// this function is used both to set the framerate and apply phase sync
pub extern "C" fn alvr_wait_for_vsync() {}

pub extern "C" fn alvr_present_layers(layers: *mut [ALayer; 2], layers_count: usize) {}

pub extern "C" fn alvr_send_haptics(
    decide_index: u32,
    duration_ns: u64,
    frequency: f32,
    amplitude: f32,
) {
}
