use super::{InitData, SettingControl};
use iced::Container;
use settings_schema::SchemaNode;

pub struct Control {
    entries: Vec<SettingControl>,
}

impl Control {
    pub fn new(data: InitData<Vec<SchemaNode>>) -> Self {
        Self { entries: vec![] }
    }
}
