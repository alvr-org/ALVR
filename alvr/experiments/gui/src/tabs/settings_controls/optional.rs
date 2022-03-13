use super::{reset, SettingControl};
use iced::button;

pub struct Control {
    default_set: bool,
    set: bool,
    set_button_state: button::State,
    unset_button_state: button::State,
    inner_control: SettingControl,
    reset_control: reset::Control,
}
