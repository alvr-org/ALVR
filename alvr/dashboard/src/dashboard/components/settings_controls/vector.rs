use crate::dashboard::components::collapsible;

use super::{reset, NestingInfo, SettingControl, INDENTATION_STEP};
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::SchemaNode;
use eframe::{
    egui::{self, Layout, Ui},
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

        {
            let session_array_mut = session_fragment["content"].as_array().unwrap();

            while session_array_mut.len() > self.controls.len() {
                let mut nesting_info = self.nesting_info.clone();
                nesting_info.path.push("content".into());
                nesting_info.path.push(self.controls.len().into());

                self.controls.push(SettingControl::new(
                    nesting_info,
                    self.default_element.clone(),
                ))
            }
            while session_array_mut.len() < self.controls.len() {
                self.controls.pop();
            }
        }

        fn get_content_request(
            nesting_info: &NestingInfo,
            elements: Vec<json::Value>,
        ) -> Option<PathValuePair> {
            super::get_single_value(nesting_info, "content".into(), json::Value::Array(elements))
        }

        if !collapsed {
            ui.end_row();

            let mut idx = 0;
            while idx < self.controls.len() {
                let response = ui
                    .horizontal(|ui| {
                        ui.add_space(INDENTATION_STEP * self.nesting_info.indentation_level as f32);

                        let response = ui.button("❌");

                        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                            let session_content =
                                session_fragment["content"].as_array_mut().unwrap();

                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                            let up_clicked = ui
                                .add_visible_ui(idx > 0, |ui| ui.small_button("⬆"))
                                .inner
                                .clicked();
                            let down_clicked = ui
                                .add_visible_ui(idx < self.controls.len() - 1, |ui| {
                                    ui.small_button("⬇")
                                })
                                .inner
                                .clicked();

                            if up_clicked || down_clicked {
                                let mut session_content = session_content.clone();
                                session_content
                                    .swap(idx, if up_clicked { idx - 1 } else { idx + 1 });

                                request = get_content_request(&self.nesting_info, session_content);
                            }
                        });

                        response
                    })
                    .inner;

                let session_array_mut = session_fragment["content"].as_array_mut().unwrap();

                if response.clicked() {
                    session_array_mut.remove(idx);
                    self.controls.remove(idx);
                } else {
                    request = self.controls[idx]
                        .ui(ui, &mut session_array_mut[idx], true)
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
