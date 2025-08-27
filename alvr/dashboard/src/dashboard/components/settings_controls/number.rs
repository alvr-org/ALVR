use super::{NestingInfo, reset};
use crate::dashboard::components::f64_eq;
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::{NumberType, NumericGuiType};
use eframe::{
    egui::{DragValue, Layout, Slider, Ui, vec2},
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

            let mut is_editing = false;
            let mut finished_editing = false;

            if let NumericGuiType::Slider {
                range,
                step,
                logarithmic,
            } = &self.gui_type
            {
                let mut slider = Slider::new(editing_value_mut, range.clone())
                    .logarithmic(*logarithmic)
                    .show_value(false);

                if let Some(step) = step {
                    slider = slider.step_by(*step);
                }
                if !matches!(self.ty, NumberType::Float) {
                    slider = slider.integer();
                }
                if let Some(suffix) = &self.suffix {
                    slider = slider.suffix(suffix);
                }

                ui.scope(|ui| {
                    ui.style_mut().spacing.interact_size.y = 20.0;
                    let slider_response = ui.add(slider);

                    is_editing = slider_response.drag_started() || slider_response.gained_focus();
                    finished_editing =
                        slider_response.drag_stopped() || slider_response.lost_focus();
                });
            }

            let mut drag_value = DragValue::new(editing_value_mut);

            if !matches!(self.ty, NumberType::Float) {
                drag_value = drag_value.fixed_decimals(0);
            }
            if let Some(suffix) = &self.suffix {
                drag_value = drag_value.suffix(suffix);
            }

            let drag_value_response = ui.add(drag_value);

            if is_editing || drag_value_response.drag_started() || drag_value_response.gained_focus() {
                self.editing_value_f64 = Some(session_value)
            } else if finished_editing || drag_value_response.drag_stopped() || drag_value_response.lost_focus() {
                request = get_request(&self.nesting_info, *editing_value_mut, self.ty);
                *session_fragment = to_json_value(*editing_value_mut, self.ty);

                self.editing_value_f64 = None;
            }

            if reset::reset_button(
                ui,
                !f64_eq(session_value, self.default),
                &self.default_string,
            )
            .clicked()
            {
                request = get_request(&self.nesting_info, self.default, self.ty);
            }
        });

        request
    }
}
