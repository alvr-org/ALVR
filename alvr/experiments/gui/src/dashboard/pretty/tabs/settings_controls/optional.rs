use iced::button;

use super::{reset, SettingControl};

pub struct Control {
    default_set: bool,
    set: bool,
    set_button_state: button::State,
    unset_button_state: button::State,
    inner_control: SettingControl,
    reset_control: reset::Control,
}
