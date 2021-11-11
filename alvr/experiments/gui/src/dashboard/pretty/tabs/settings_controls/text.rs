use super::{reset, DrawingData, DrawingResult, InitData, UpdatingData, ROW_HEIGHT_UNITS};
use crate::dashboard::pretty::tabs::{SettingControlEvent, SettingControlEventType};
use iced::{text_input, Space, Text, TextInput};
use serde_json as json;

pub struct Control {
    default: String,
    value: String,
    control_state: text_input::State,
    reset_control: reset::Control,
}

impl Control {
    pub fn new(data: InitData<String>) -> Self {
        Self {
            default: data.schema,
            value: "".into(),
            control_state: text_input::State::new(),
            reset_control: reset::Control::new(),
        }
    }

    pub fn update(&mut self, data: UpdatingData) {
        match data.event {
            SettingControlEventType::SessionUpdated(session) => {
                self.value = json::from_value(session).unwrap()
            }
            SettingControlEventType::ResetClick => todo!(),
            SettingControlEventType::ApplyValue => todo!(),
            _ => unimplemented!(),
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let inline = TextInput::new(&mut self.control_state, "".into(), &self.value, |s| {
            SettingControlEvent {
                path: vec![],
                event_type: SettingControlEventType::TextChanged(s),
            }
        })
        .size(ROW_HEIGHT_UNITS);

        DrawingResult {
            inline: Some(inline.into()),
            left: Space::with_height(0.into()).into(),
            right: Space::with_height(0.into()).into(),
        }
    }
}
