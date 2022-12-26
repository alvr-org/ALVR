use crate::RenderViewInput;
use alvr_common::{
    glam::{Quat, UVec2, Vec3},
    Fov,
};
use alvr_session::FoveatedRenderingDesc;
use std::{
    ffi::{c_char, c_void, CStr},
    ptr,
};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AlvrQuat {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlvrFov {
    left: f32,
    right: f32,
    up: f32,
    down: f32,
}

#[repr(C)]
pub struct AlvrViewInput {
    orientation: AlvrQuat,
    position: [f32; 3],
    fov: AlvrFov,
    swapchain_index: u32,
}

#[repr(C)]
pub struct AlvrStreamConfig {
    pub view_resolution_width: u32,
    pub view_resolution_height: u32,
    pub swapchain_textures: *mut *const u32,
    pub swapchain_length: u32,
    pub enable_foveation: bool,
    pub foveation_center_size_x: f32,
    pub foveation_center_size_y: f32,
    pub foveation_center_shift_x: f32,
    pub foveation_center_shift_y: f32,
    pub foveation_edge_ratio_x: f32,
    pub foveation_edge_ratio_y: f32,
}

#[no_mangle]
pub extern "C" fn alvr_initialize_opengl() {
    crate::initialize();
}

#[no_mangle]
pub extern "C" fn alvr_destroy_opengl() {
    crate::destroy();
}

unsafe fn convert_swapchain_array(
    swapchain_textures: *mut *const u32,
    swapchain_length: u32,
) -> [Vec<u32>; 2] {
    let swapchain_length = swapchain_length as usize;
    let mut left_swapchain = vec![0; swapchain_length];
    ptr::copy_nonoverlapping(
        *swapchain_textures,
        left_swapchain.as_mut_ptr(),
        swapchain_length,
    );
    let mut right_swapchain = vec![0; swapchain_length];
    ptr::copy_nonoverlapping(
        *swapchain_textures.offset(1),
        right_swapchain.as_mut_ptr(),
        swapchain_length,
    );

    [left_swapchain, right_swapchain]
}

#[no_mangle]
pub unsafe extern "C" fn alvr_resume_opengl(
    preferred_view_width: u32,
    preferred_view_height: u32,
    swapchain_textures: *mut *const u32,
    swapchain_length: u32,
) {
    crate::resume(
        UVec2::new(preferred_view_width, preferred_view_height),
        convert_swapchain_array(swapchain_textures, swapchain_length),
    );
}

#[no_mangle]
pub extern "C" fn alvr_pause_opengl() {
    crate::pause();
}

#[no_mangle]
pub unsafe extern "C" fn alvr_update_hud_message(message: *const c_char) {
    crate::update_hud_message(CStr::from_ptr(message).to_str().unwrap());
}

#[no_mangle]
pub unsafe extern "C" fn alvr_start_stream_opengl(config: AlvrStreamConfig) {
    let view_resolution = UVec2::new(config.view_resolution_width, config.view_resolution_height);
    let swapchain_textures =
        convert_swapchain_array(config.swapchain_textures, config.swapchain_length);
    let foveated_rendering = config.enable_foveation.then_some(FoveatedRenderingDesc {
        center_size_x: config.foveation_center_size_x,
        center_size_y: config.foveation_center_size_y,
        center_shift_x: config.foveation_center_shift_x,
        center_shift_y: config.foveation_center_shift_y,
        edge_ratio_x: config.foveation_edge_ratio_x,
        edge_ratio_y: config.foveation_edge_ratio_y,
    });

    crate::start_stream(view_resolution, swapchain_textures, foveated_rendering);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_lobby_opengl(view_inputs: *const AlvrViewInput) {
    let view_inputs = [
        {
            let o = (*view_inputs).orientation;
            let f = (*view_inputs).fov;
            RenderViewInput {
                orientation: Quat::from_xyzw(o.x, o.y, o.z, o.w),
                position: Vec3::from_array((*view_inputs).position),
                fov: Fov {
                    left: f.left,
                    right: f.right,
                    up: f.up,
                    down: f.down,
                },
                swapchain_index: (*view_inputs).swapchain_index,
            }
        },
        {
            let o = (*view_inputs.offset(1)).orientation;
            let f = (*view_inputs.offset(1)).fov;
            RenderViewInput {
                orientation: Quat::from_xyzw(o.x, o.y, o.z, o.w),
                position: Vec3::from_array((*view_inputs).position),
                fov: Fov {
                    left: f.left,
                    right: f.right,
                    up: f.up,
                    down: f.down,
                },
                swapchain_index: (*view_inputs.offset(1)).swapchain_index,
            }
        },
    ];

    crate::render_lobby(view_inputs);
}

#[no_mangle]
pub unsafe extern "C" fn alvr_render_stream_opengl(
    hardware_buffer: *mut c_void,
    swapchain_indices: *const u32,
) {
    crate::render_stream(
        hardware_buffer,
        [*swapchain_indices, *swapchain_indices.offset(1)],
    );
}
