use super::{reset, SettingControl};
use iced::{button, text_input};
use serde_json as json;

struct Entry {
    text_state: text_input::State,
    control: SettingControl,
}

pub struct Control {
    default: json::Value,
    default_key: String,
    default_value: json::Value,
    entries: Vec<Entry>,
    up_button_state: button::State,
    down_button_state: button::State,
    add_button_state: button::State,
    reset_control: reset::Control,
}
