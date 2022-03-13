use super::{reset, SettingControl};
use iced::button;
use serde_json as json;

pub struct Control {
    default: Vec<(String, json::Value)>,
    default_entry: json::Value,
    entries: Vec<SettingControl>,
    up_button_state: button::State,
    down_button_state: button::State,
    add_button_state: button::State,
    reset_control: reset::Control,
}
