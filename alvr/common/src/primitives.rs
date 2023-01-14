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
