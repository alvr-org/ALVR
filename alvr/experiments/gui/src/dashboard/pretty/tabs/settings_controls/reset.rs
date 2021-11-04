use iced::{button, Button, Element, Text};

use super::{SettingControlEvent, SettingControlEventType};

pub struct Control {
    enabled: bool,
    button_state: button::State,
}

impl Control {
    pub fn new() -> Self {
        Self {
            enabled: false,
            button_state: button::State::new(),
        }
    }

    pub fn update(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn view(&mut self) -> Element<()> {
        let button = Button::new(&mut self.button_state, Text::new("Reset"));

        if self.enabled {
            button.on_press(()).into()
        } else {
            button.into()
        }
    }
}
