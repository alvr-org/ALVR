use super::{SettingControl, SettingsContext, SettingsResponse};
use egui::{emath::Numeric, DragValue, Slider, Ui};
use serde::{de::DeserializeOwned, Serialize};
use serde_json as json;
use settings_schema::NumericGuiType;
use std::ops::RangeInclusive;

#[derive(Clone)]
pub enum NumericWidgetType<T> {
    Slider {
        range: RangeInclusive<T>,
        step: f64,
    },
    TextBox {
        range: Option<RangeInclusive<T>>,
        step: Option<f64>,
    },
}

pub struct NumericWidget<T> {
    value: T,
    default: T,
    numeric_type: NumericWidgetType<T>,
}

impl<T: DeserializeOwned + Copy + Numeric> NumericWidget<T> {
    pub fn new(
        session_fragment: json::Value,
        default: T,
        min: Option<T>,
        max: Option<T>,
        step: Option<T>,
        gui: Option<NumericGuiType>,
        integer: bool,
    ) -> Self {
        let step = if let Some(step) = step {
            Some(step.to_f64())
        } else {
            integer.then(|| 1_f64)
        };

        let initial_value = json::from_value::<T>(session_fragment).unwrap();

        let gui = gui.unwrap_or(NumericGuiType::Slider);

        let numeric_type = if let (Some(min), Some(max), Some(step)) = (min, max, step) {
            let range = min..=max;

            match gui {
                NumericGuiType::Slider => NumericWidgetType::Slider { range, step },
                NumericGuiType::UpDown => NumericWidgetType::TextBox {
                    range: Some(range),
                    step: Some(step),
                },
                NumericGuiType::TextBox => NumericWidgetType::TextBox {
                    range: Some(range),
                    step: None,
                },
            }
        } else {
            let range = if let (Some(min), Some(max)) = (min, max) {
                Some(min..=max)
            } else {
                None
            };

            let step = if matches!(gui, NumericGuiType::TextBox) {
                None
            } else {
                step
            };

            NumericWidgetType::TextBox { range, step }
        };

        Self {
            value: initial_value,
            default,
            numeric_type,
        }
    }
}

impl<T: Serialize + DeserializeOwned + Numeric + ToString> SettingControl for NumericWidget<T> {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        _: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let response = match self.numeric_type.clone() {
            NumericWidgetType::Slider { range, step } => {
                // todo: handle step
                let slider_response =
                    ui.add(Slider::new(&mut self.value, range).clamp_to_range(true));

                if slider_response.drag_released() || slider_response.lost_focus() {
                    Some(super::into_fragment(self.value))
                } else {
                    // Drag events are not captured for the included textbox, so its value gets overridden. todo: make a PR
                    if !slider_response.dragged() && !slider_response.has_focus() {
                        self.value = json::from_value::<T>(session_fragment).unwrap();
                    }

                    None
                }
            }
            NumericWidgetType::TextBox { range, step } => {
                // egui supports prefixes/suffixes. todo: add support for suffixes in
                // settings-schema
                let mut textbox = DragValue::new(&mut self.value);
                if let Some(range) = range {
                    textbox = textbox.clamp_range(range);
                }
                if let Some(step) = step {
                    textbox = textbox.speed(step.to_f64());
                } else {
                    textbox = textbox.speed(0);
                }

                let res = ui.add(textbox);

                if res.drag_released() || res.lost_focus() {
                    Some(super::into_fragment(self.value))
                } else {
                    if !res.dragged() && !res.has_focus() {
                        // if not currently interacting with the control, overwrite the value from
                        // the session continuously
                        self.value = json::from_value::<T>(session_fragment).unwrap();
                    }
                    None
                }
            }
        };

        super::reset_clicked(ui, &self.value, &self.default, &self.default.to_string())
            .then(|| super::into_fragment(self.default))
            .or(response)
    }
}
