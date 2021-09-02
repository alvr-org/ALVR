use serde::{Deserialize, Serialize};
use settings_schema::{EntryData, SettingsSchema};

#[derive(SettingsSchema, Serialize, Deserialize, PartialEq, Default, Clone)]
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
