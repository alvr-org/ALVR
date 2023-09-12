use super::{reset, NestingInfo, SettingControl, INDENTATION_STEP};
use crate::dashboard::components::{
    collapsible,
    up_down::{self, UpDownResult},
};
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::SchemaNode;
use eframe::{
    egui::{Layout, Ui},
    emath::Align,
};
use serde_json as json;

pub struct Control {
    nesting_info: NestingInfo,
    default_element: SchemaNode,
    default: Vec<json::Value>,
    controls: Vec<SettingControl>,
}

impl Control {
    pub fn new(
        nesting_info: NestingInfo,
        default_element: SchemaNode,
        default: Vec<json::Value>,
    ) -> Self {
        Self {
            nesting_info,
            default_element,
            default,
            controls: vec![],
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: &mut json::Value,
        allow_inline: bool,
    ) -> Option<PathValuePair> {
        super::grid_flow_inline(ui, allow_inline);

        fn get_content_request(
            nesting_info: &NestingInfo,
            elements: Vec<json::Value>,
        ) -> Option<PathValuePair> {
            super::get_single_value(nesting_info, "content".into(), json::Value::Array(elements))
        }

        let mut request = None;
        let collapsed = ui
            .with_layout(Layout::left_to_right(Align::Center), |ui| {
                let collapsed = collapsible::collapsible_button(
                    ui,
                    &self.nesting_info,
                    session_fragment,
                    &mut request,
                );

                if reset::reset_button(ui, true, "default list").clicked() {
                    request = get_content_request(&self.nesting_info, self.default.clone())
                }

                collapsed
            })
            .inner;

        let session_content = session_fragment["content"].as_array_mut().unwrap();

        while session_content.len() > self.controls.len() {
            let mut nesting_info = self.nesting_info.clone();
            nesting_info.path.push("content".into());
            nesting_info.path.push(self.controls.len().into());

            self.controls.push(SettingControl::new(
                nesting_info,
                self.default_element.clone(),
            ))
        }
        while session_content.len() < self.controls.len() {
            self.controls.pop();
        }

        if !collapsed {
            ui.end_row();

            let mut idx = 0;
            while idx < self.controls.len() {
                let delete_element = ui
                    .horizontal(|ui| {
                        ui.add_space(INDENTATION_STEP * self.nesting_info.indentation_level as f32);

                        let delete_element = ui.button("❌").clicked();

                        let up_down_result = up_down::up_down_buttons(ui, idx, self.controls.len());

                        if up_down_result != UpDownResult::None {
                            if up_down_result == UpDownResult::Up {
                                session_content.swap(idx, idx - 1);
                            } else {
                                session_content.swap(idx, idx + 1);
                            }

                            request =
                                get_content_request(&self.nesting_info, session_content.clone());
                        }

                        delete_element
                    })
                    .inner;

                if delete_element {
                    session_content.remove(idx);
                    self.controls.remove(idx);

                    request = get_content_request(&self.nesting_info, session_content.clone());
                } else {
                    request = self.controls[idx]
                        .ui(ui, &mut session_content[idx], true)
                        .or(request);
                }

                ui.end_row();

                idx += 1;
            }

            ui.label(" ");
            if ui.button("Add element").clicked() {
                let mut session_content =
                    session_fragment["content"].as_array_mut().unwrap().clone();
                session_content.push(session_fragment["element"].clone());

                request = get_content_request(&self.nesting_info, session_content);
            }
        }

        request
    }
}
