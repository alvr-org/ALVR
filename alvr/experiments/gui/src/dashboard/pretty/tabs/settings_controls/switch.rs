use super::{reset, InitData, SettingControl};
use serde_json as json;
use settings_schema::{SchemaNode, SwitchDefault};

pub struct Control {
    default_enabled: bool,
    content_advanced: bool,
    enabled: bool,
    inner_control: SettingControl,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<(bool, bool, Box<SchemaNode>)>) -> Self {
        let (default_enabled, content_advanced, content_schema) = data.schema;
        // let session_switch = json::from_value::<SwitchDefault<json::Value>>(data.session).unwrap();

        Self {
            default_enabled,
            content_advanced,
            enabled: false,
            inner_control: SettingControl::new(InitData {
                schema: *content_schema,
                trans: (),
            }),
            reset_control: reset::Control::new(),
        }
    }
}
