use super::{
    reset, DrawingData, DrawingResult, InitData, UpdatingData, ROW_HEIGHT, ROW_HEIGHT_UNITS,
};
use crate::dashboard::pretty::{
    tabs::{SettingControlEvent, SettingControlEventType},
    theme::TextInputStyle,
};
use iced::{text_input, Alignment, Length, Row, Space, Text, TextInput};
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
            SettingControlEventType::TextChanged(_) => todo!(),
            SettingControlEventType::ApplyValue => todo!(),
            _ => unimplemented!(),
        }
    }

    pub fn view(&mut self, _: &DrawingData) -> DrawingResult {
        let inline = Row::new()
            .push(
                Row::new()
                    .push(
                        TextInput::new(&mut self.control_state, "", &self.value, |s| {
                            SettingControlEvent {
                                path: vec![],
                                event_type: SettingControlEventType::TextChanged(s),
                            }
                        })
                        .padding([0, 5])
                        .style(TextInputStyle),
                    )
                    .padding([5, 0])
                    .width(Length::Fill),
            )
            .push(self.reset_control.view())
            .height(ROW_HEIGHT)
            .spacing(5)
            .align_items(Alignment::Center);

        DrawingResult {
            inline: Some(inline.into()),
            left: Space::with_height(0.into()).into(),
            right: Space::with_height(0.into()).into(),
        }
    }
}
