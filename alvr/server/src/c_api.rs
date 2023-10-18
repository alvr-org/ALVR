#![allow(dead_code, unused_variables)]

use std::{
    ffi::{c_char, CStr},
    time::Instant,
};

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

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum AlvrOutput {
    Haptics {
        frequency: f32,
        amplitude: f32,
        duration_ns: u64,
    },
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrBatteryValue {
    pub device_id: u64,
    /// range [0, 1]
    pub value: f32,
}

#[repr(C)]
pub enum AlvrEvent {
    Battery(AlvrBatteryValue),
    Bounds([f32; 2]),
    Restart,
    Shutdown,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrTargetConfig {
    target_width: u32,
    target_height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrDeviceConfig {
    device_id: u64,
    interaction_profile_id: u64,
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
pub unsafe extern "C" fn alvr_initialize(out_target_config: *mut AlvrTargetConfig) {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_poll_event(out_event: *mut AlvrEvent) -> bool {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_shutdown() {
    todo!()
}

// Device API:

// Use the two-call pattern to first get the array length then the array data.
#[no_mangle]
pub unsafe extern "C" fn alvr_get_devices(out_device_configs: *mut AlvrDeviceConfig) -> u64 {
    todo!()
}

// After this call, previous button and tracking data is discarded
#[no_mangle]
pub unsafe extern "C" fn alvr_update_inputs(device_id: u64) {
    todo!()
}

// Use the two-call pattern to first get the array length then the array data.
// Data is updated after a call to alvr_update_inputs.
#[no_mangle]
pub unsafe extern "C" fn alvr_get_inputs(
    device_id: u64,
    out_inputs_arr: *mut AlvrInput,
    out_timestamp_ns: u64,
) -> u64 {
    todo!()
}

// pose_id is something like /user/hand/left/input/grip/pose
#[no_mangle]
pub unsafe extern "C" fn alvr_get_tracked_pose(
    pose_id: u64,
    timestamp_ns: u64,
    out_relation: *mut AlvrSpaceRelation,
) {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_get_hand_tracking(
    device_id: u64,
    timestamp_ns: u64,
    out_joint_set: *mut AlvrJointSet,
) {
    todo!()
}

// Currently only haptics is supported
#[no_mangle]
pub unsafe extern "C" fn alvr_set_output(output_id: u64, value: *const AlvrOutput) {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_view_poses(
    out_head_relation: *mut AlvrSpaceRelation,
    out_fov_arr: *mut AlvrFov,            // 2 elements
    out_relative_pose_arr: *mut AlvrPose, // 2 elements
) {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_destroy_device(device_id: u64) {
    todo!()
}

// Compositor target API:

// This should reflect the client current framerate
#[no_mangle]
pub unsafe extern "C" fn alvr_get_framerate() -> f32 {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_pre_vulkan() {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_post_vulkan() {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_create_vk_target_swapchain(
    width: u32,
    height: u32,
    vk_color_format: i32,
    vk_color_space: i32,
    vk_image_usage: u32,
    vk_present_mode: i32,
    image_count: u64,
) {
    todo!()
}

// returns vkResult
#[no_mangle]
pub unsafe extern "C" fn alvr_acquire_image(out_swapchain_index: u64) -> i32 {
    todo!()
}

// returns vkResult
#[no_mangle]
pub unsafe extern "C" fn alvr_present(
    vk_queue: u64,
    swapchain_index: u64,
    timeline_semaphore_value: u64,
    timestamp_ns: u64,
) -> i32 {
    todo!()
}

#[no_mangle]
pub unsafe extern "C" fn alvr_destroy_vk_target_swapchain() {
    todo!()
}
