use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

pub use glam;

// Field of view in radians
#[derive(Serialize, Deserialize, PartialEq, Default, Clone, Copy)]
pub struct Fov {
    pub left: f32,
    pub right: f32,
    pub up: f32,
    pub down: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug)]
pub struct Pose {
    pub orientation: Quat,
    pub position: Vec3,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug)]
pub struct DeviceMotion {
    pub pose: Pose,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}
