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

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub enum OpenvrPropValue {
    Bool(bool),
    Float(f32),
    Int32(i32),
    Uint64(u64),
    Vector3([f32; 3]),
    Double(f64),
    String(String),
}

#[derive(Serialize, Deserialize)]
pub struct MotionData {
    pub orientation: Quat,
    pub position: Vec3,
    pub linear_velocity: Option<Vec3>,
    pub angular_velocity: Option<Vec3>,
}
