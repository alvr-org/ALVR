use super::{reset, NestingInfo, SettingControl, INDENTATION_STEP};
use crate::dashboard::components::{
    collapsible,
    up_down::{self, UpDownResult},
};
use alvr_packets::PathValuePair;
use alvr_session::settings_schema::SchemaNode;
use eframe::{
    egui::{Layout, TextEdit, Ui},
    emath::Align,
};
use serde_json as json;

struct Entry {
    editing_key: Option<String>,
    control: SettingControl,
}

pub struct Control {
    nesting_info: NestingInfo,
    default_key: String,
    default_value: SchemaNode,
    default: Vec<json::Value>,
    controls: Vec<Entry>,
}

impl Control {
    pub fn new(
        nesting_info: NestingInfo,
        default_key: String,
        default_value: SchemaNode,
        default: Vec<(String, json::Value)>,
    ) -> Self {
        Self {
            nesting_info,
            default_key,
            default_value,
            default: default
                .into_iter()
                .map(|pair| json::to_value(pair).unwrap())
                .collect(),
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
            entries: Vec<json::Value>,
        ) -> Option<PathValuePair> {
            super::get_single_value(
                nesting_info,
                "content".into(),
                json::to_value(entries).unwrap(),
            )
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
            nesting_info.path.extend_from_slice(&[
                "content".into(),
                self.controls.len().into(),
                1.into(),
            ]);

            self.controls.push(Entry {
                editing_key: None,
                control: SettingControl::new(nesting_info, self.default_value.clone()),
            });
        }
        while session_content.len() < self.controls.len() {
            self.controls.pop();
        }

        if !collapsed {
            ui.end_row();

            let mut idx = 0;
            while idx < self.controls.len() {
                let delete_entry = ui
                    .horizontal(|ui| {
                        ui.add_space(INDENTATION_STEP * self.nesting_info.indentation_level as f32);

                        let delete_entry = ui.button("❌").clicked();

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

                        let json::Value::String(text_mut) = &mut session_content[idx][0] else {
                            unreachable!()
                        };

                        let editing_key_mut = &mut self.controls[idx].editing_key;

                        let textbox = if let Some(editing_key_mut) = editing_key_mut {
                            TextEdit::singleline(editing_key_mut)
                        } else {
                            TextEdit::singleline(text_mut)
                        };

                        let response = ui.add(textbox.desired_width(f32::INFINITY));
                        if response.lost_focus() {
                            if let Some(editing_key_mut) = editing_key_mut {
                                let mut nesting_info = self.nesting_info.clone();
                                nesting_info
                                    .path
                                    .extend_from_slice(&["content".into(), idx.into()]);

                                request = super::get_single_value(
                                    &nesting_info,
                                    0.into(),
                                    json::Value::String(editing_key_mut.clone()),
                                );

                                *text_mut = editing_key_mut.clone();
                            }

                            *editing_key_mut = None;
                        }
                        if response.gained_focus() {
                            *editing_key_mut = Some(text_mut.clone());
                        };

                        delete_entry
                    })
                    .inner;

                if delete_entry {
                    session_content.remove(idx);
                    self.controls.remove(idx);

                    request = get_content_request(&self.nesting_info, session_content.clone());
                } else {
                    request = self.controls[idx]
                        .control
                        .ui(ui, &mut session_content[idx][1], true)
                        .or(request);
                }

                ui.end_row();

                idx += 1;
            }

            ui.label(" ");
            if ui.button("Add entry").clicked() {
                let mut session_content =
                    session_fragment["content"].as_array_mut().unwrap().clone();
                session_content.push(json::Value::Array(vec![
                    json::Value::String(self.default_key.clone()),
                    session_fragment["value"].clone(),
                ]));

                request = get_content_request(&self.nesting_info, session_content);
            }
        }

        request
    }
}
