use super::{
    reset, DrawingResult, InitData, SettingControlEvent, SettingControlEventType, UpdatingData,
    ROW_HEIGHT,
};
use crate::dashboard::pretty::theme::{TextInputStyle, TooltipStyle};
use iced::{
    slider, text_input, tooltip::Position, Alignment, Length, Row, Slider, Space, Text, TextInput,
    Tooltip,
};
use num::FromPrimitive;
use serde::de::DeserializeOwned;
use serde_json as json;
use settings_schema::NumericGuiType;
use std::{any, fmt::Display, ops::RangeInclusive, str::FromStr};

struct SliderState<T> {
    state: slider::State,
    range: RangeInclusive<T>,
    step: T,
}

pub struct Control<T> {
    default: T,
    value: T,
    min: Option<T>,
    max: Option<T>,
    slider_state: Option<SliderState<T>>,
    text: String,
    textbox_state: text_input::State,
    reset_control: reset::Control,
}

impl<
        T: Copy
            + Display
            + FromStr
            + PartialEq
            + PartialOrd
            + DeserializeOwned
            + From<u8>
            + FromPrimitive
            + 'static,
    > Control<T>
where
    f64: From<T>,
{
    pub fn new(
        data: InitData<(T, Option<T>, Option<T>, Option<T>, Option<NumericGuiType>)>,
    ) -> Self {
        let (default, min, max, step, gui) = data.schema;

        let slider_state = if let (Some(min), Some(max), Some(step)) =
            (min.as_ref(), max.as_ref(), step.as_ref())
        {
            Some(SliderState {
                state: slider::State::new(),
                range: *min..=*max,
                step: *step,
            })
        } else {
            None
        };

        Self {
            default,
            value: default,
            min,
            max,
            slider_state,
            text: "".into(),
            textbox_state: text_input::State::new(),
            reset_control: reset::Control::new(),
        }
    }

    pub fn update(&mut self, data: UpdatingData) {
        match data.event {
            SettingControlEventType::SessionUpdated(session) => {
                self.value = json::from_value(session).unwrap();
                self.text = format!("{}", self.value);
                self.reset_control.update(self.value != self.default);
            }
            SettingControlEventType::TempValueChanged(text) => self.text = text,
            event => {
                let value = if let SettingControlEventType::ApplyValue = event {
                    if let Ok(mut value) = self.text.parse() {
                        if let Some(min) = self.min {
                            if value < min {
                                value = min;
                            }
                        }
                        if let Some(max) = self.max {
                            if value > max {
                                value = max;
                            }
                        }

                        value
                    } else {
                        self.value
                    }
                } else {
                    self.default
                };

                let mut value_string = value.to_string();

                if (any::type_name::<T>() == "f32" || any::type_name::<T>() == "f64")
                    && !value_string.contains('.')
                {
                    value_string.push_str(".0");
                }

                (data.request_handler)(format!(
                    r#"
                        let session = load_session();
                        {} = {};
                        store_session(session);
                    "#,
                    data.string_path, value_string
                ))
                .unwrap();
            }
        }
    }

    pub fn view(&mut self) -> DrawingResult {
        let mut text_input = TextInput::new(&mut self.textbox_state, "", &self.text, |s| {
            SettingControlEvent {
                path: vec![],
                event_type: SettingControlEventType::TempValueChanged(s),
            }
        })
        .padding([0, 5])
        .style(TextInputStyle)
        .on_submit(SettingControlEvent {
            path: vec![],
            event_type: SettingControlEventType::ApplyValue,
        });

        if self.slider_state.is_some() {
            text_input = text_input.width(50.into());
        }

        let slider = if let Some(slider_state) = &mut self.slider_state {
            let slider = Slider::new(
                &mut slider_state.state,
                slider_state.range.clone(),
                self.text.parse().unwrap_or(self.value),
                |value| SettingControlEvent {
                    path: vec![],
                    event_type: SettingControlEventType::TempValueChanged(format!("{}", value)),
                },
            )
            .step(slider_state.step)
            .on_release(SettingControlEvent {
                path: vec![],
                event_type: SettingControlEventType::ApplyValue,
            });

            vec![
                Text::new(slider_state.range.start().to_string())
                    .size(12)
                    .into(),
                slider.into(),
                Text::new(slider_state.range.end().to_string())
                    .size(12)
                    .into(),
            ]
        } else {
            vec![]
        };

        let inline = Row::new()
            .push(
                Row::with_children(slider)
                    .push(
                        Tooltip::new(
                            text_input,
                            "Press Enter to apply the value",
                            Position::Bottom,
                        )
                        .style(TooltipStyle),
                    )
                    .padding([5, 0])
                    .width(Length::Fill)
                    .spacing(5)
                    .align_items(Alignment::Center),
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
