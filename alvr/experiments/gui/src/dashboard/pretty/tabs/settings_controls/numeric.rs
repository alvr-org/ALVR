use crate::dashboard::pretty::{
    tabs::{InitData, SettingControlEventType},
    theme::TextInputStyle,
};

use super::{
    reset, DrawingData, DrawingResult, SettingControlEvent, UpdatingData, ROW_HEIGHT,
    ROW_HEIGHT_UNITS,
};
use iced::{slider, text_input, Alignment, Container, Length, Row, Space, Text, TextInput};
use iced_native::Widget;
use serde::de::DeserializeOwned;
use serde_json as json;
use settings_schema::NumericGuiType;
use std::{fmt::Display, ops::RangeInclusive};

struct SliderState<T> {
    state: slider::State,
    range: RangeInclusive<T>,
}

pub struct Control<T> {
    default: T,
    value: T,
    slider_state: Option<SliderState<T>>,
    text: String,
    textbox_state: text_input::State,
    reset_control: reset::Control,
}

impl<T: Copy + Display + PartialEq + DeserializeOwned> Control<T> {
    pub fn new(
        data: InitData<(T, Option<T>, Option<T>, Option<T>, Option<NumericGuiType>)>,
    ) -> Self {
        let (default, min, max, step, gui) = data.schema;

        Self {
            default: default.clone(),
            value: default,
            slider_state: None, // todo
            text: format!("{}", default),
            textbox_state: text_input::State::new(),
            reset_control: reset::Control::new(),
        }
    }

    pub fn update(&mut self, mut data: UpdatingData) {
        match data.event {
            SettingControlEventType::SessionUpdated(session) => {
                self.value = json::from_value(session).unwrap()
            }
            SettingControlEventType::ResetClick => todo!(),
            SettingControlEventType::IntegerChanged(_) => todo!(),
            SettingControlEventType::FloatChanged(_) => todo!(),
            SettingControlEventType::TextChanged(_) => todo!(),
            SettingControlEventType::ApplyValue => todo!(),
            _ => unreachable!(),
        }
    }

    pub fn view(&mut self, data: &DrawingData) -> DrawingResult {
        let inline = Row::new()
            .push(
                Row::new()
                    .push(
                        TextInput::new(&mut self.textbox_state, "", &self.text, |s| {
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
