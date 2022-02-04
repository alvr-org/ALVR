use std::time::Duration;

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use settings_schema::{EntryData, SettingsSchema};

// Field of view in radians
#[derive(SettingsSchema, Serialize, Deserialize, PartialEq, Default, Clone, Copy)]
pub struct Fov {
    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub left: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub right: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub top: f32,

    #[schema(min = 0., max = 90., step = 0.1, gui = "UpDown")]
    pub bottom: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MotionData {
    pub orientation: Quat,
    pub position: Vec3,
    pub linear_velocity: Option<Vec3>,
    pub angular_velocity: Option<Vec3>,
}

#[derive(Serialize, Deserialize)]
pub struct Haptics {
    pub path: u64,
    pub duration: Duration,
    pub frequency: f32,
    pub amplitude: f32,
}
