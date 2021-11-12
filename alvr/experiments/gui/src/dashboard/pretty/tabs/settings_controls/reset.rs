use super::{SettingControlEvent, SettingControlEventType};
use crate::dashboard::pretty::theme::ButtonStyle;
use iced::{button, Button, Element, Text};

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

    pub fn view(&mut self) -> Element<SettingControlEvent> {
        let button =
            Button::new(&mut self.button_state, Text::new("Reset")).style(ButtonStyle::Secondary);

        if self.enabled {
            button
                .on_press(SettingControlEvent {
                    path: vec![],
                    event_type: SettingControlEventType::ResetClick,
                })
                .into()
        } else {
            button.into()
        }
    }
}
