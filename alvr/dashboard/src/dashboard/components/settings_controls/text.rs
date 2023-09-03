use super::{reset, NestingInfo};
use alvr_packets::PathValuePair;
use eframe::{
    egui::{Layout, TextEdit, Ui},
    emath::Align,
};
use serde_json as json;

pub struct Control {
    nesting_info: NestingInfo,
    editing_value: Option<String>,
    default: String,
    default_string: String,
}

impl Control {
    pub fn new(nesting_info: NestingInfo, default: String) -> Self {
        let default_string = format!("\"{default}\"");

        Self {
            nesting_info,
            editing_value: None,
            default,
            default_string,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<PathValuePair> {
        super::grid_flow_inline(ui, allow_inline);

        let json::Value::String(text_mut) = session_fragment else {
            unreachable!()
        };

        let mut request = None;

        fn get_request(nesting_info: &NestingInfo, text: &str) -> Option<PathValuePair> {
            Some(PathValuePair {
                path: nesting_info.path.clone(),
                value: json::Value::String(text.to_owned()),
            })
        }

        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let textbox = if let Some(editing_value_mut) = &mut self.editing_value {
                TextEdit::singleline(editing_value_mut)
            } else {
                TextEdit::singleline(text_mut)
            };

            let response = ui.add(textbox.desired_width(250.));
            if response.lost_focus() {
                if let Some(editing_value_mut) = &mut self.editing_value {
                    request = get_request(&self.nesting_info, editing_value_mut);
                    *text_mut = editing_value_mut.clone();
                }

                self.editing_value = None;
            }
            if response.gained_focus() {
                self.editing_value = Some(text_mut.clone());
            };

            if reset::reset_button(ui, *text_mut != self.default, &self.default_string).clicked() {
                request = get_request(&self.nesting_info, &self.default);
            }
        });

        request
    }
}
