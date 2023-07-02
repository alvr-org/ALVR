use super::{reset, NestingInfo};
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::{NumberType, NumericGuiType};
use eframe::{
    egui::{DragValue, Layout, Slider, Ui},
    emath::Align,
};
use json::Number;
use serde_json as json;

fn to_json_value(number: f64, ty: NumberType) -> json::Value {
    match ty {
        NumberType::UnsignedInteger => json::Value::from(number.abs() as u64),
        NumberType::SignedInteger => json::Value::from(number as i64),
        NumberType::Float => json::Value::Number(Number::from_f64(number).unwrap()),
    }
}

pub struct Control {
    nesting_info: NestingInfo,
    editing_value_f64: Option<f64>,
    default: f64,
    default_string: String,
    ty: NumberType,
    gui_type: NumericGuiType,
    suffix: Option<String>,
}

impl Control {
    pub fn new(
        nesting_info: NestingInfo,
        default: f64,
        ty: NumberType,
        gui: NumericGuiType,
        suffix: Option<String>,
    ) -> Self {
        let default_string = format!("{default}{}", suffix.clone().unwrap_or_default());

        Self {
            nesting_info,
            editing_value_f64: None,
            default,
            default_string,
            ty,
            gui_type: gui,
            suffix,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<PathValuePair> {
        super::grid_flow_inline(ui, allow_inline);

        let mut session_value = session_fragment.as_f64().unwrap();

        let mut request = None;

        fn get_request(
            nesting_info: &NestingInfo,
            number: f64,
            ty: NumberType,
        ) -> Option<PathValuePair> {
            Some(PathValuePair {
                path: nesting_info.path.clone(),
                value: to_json_value(number, ty),
            })
        }

        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let editing_value_mut = if let Some(editing_value_mut) = &mut self.editing_value_f64 {
                editing_value_mut
            } else {
                &mut session_value
            };

            let response = match &self.gui_type {
                NumericGuiType::Slider {
                    range,
                    step,
                    logarithmic,
                } => {
                    let mut slider =
                        Slider::new(editing_value_mut, range.clone()).logarithmic(*logarithmic);

                    if let Some(step) = step {
                        slider = slider.step_by(*step);
                    }
                    if !matches!(self.ty, NumberType::Float) {
                        slider = slider.integer();
                    }
                    if let Some(suffix) = &self.suffix {
                        slider = slider.suffix(suffix);
                    }

                    // todo: investigate why the slider does not get centered vertically
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| ui.add(slider))
                        .inner
                }
                NumericGuiType::TextBox => {
                    let mut textbox = DragValue::new(editing_value_mut);

                    if !matches!(self.ty, NumberType::Float) {
                        textbox = textbox.fixed_decimals(0);
                    }
                    if let Some(suffix) = &self.suffix {
                        textbox = textbox.suffix(suffix);
                    }

                    ui.add(textbox)
                }
            };
            if response.drag_started() || response.gained_focus() {
                self.editing_value_f64 = Some(session_value)
            } else if response.drag_released() || response.lost_focus() {
                request = get_request(&self.nesting_info, *editing_value_mut, self.ty);
                *session_fragment = to_json_value(*editing_value_mut, self.ty);

                self.editing_value_f64 = None;
            }

            if reset::reset_button(ui, session_value != self.default, &self.default_string)
                .clicked()
            {
                request = get_request(&self.nesting_info, self.default, self.ty);
            }
        });

        request
    }
}
