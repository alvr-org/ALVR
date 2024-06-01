use alvr_common::{Fov, Pose};

pub mod opengl;

pub struct RenderViewInput {
    pub pose: Pose,
    pub fov: Fov,
    pub swapchain_index: u32,
}
