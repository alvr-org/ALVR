use std::collections::VecDeque;

use crate::translation::TranslationBundle;

use super::{SettingContainer, SettingControl, SettingsContext, SettingsResponse};
use egui::{Grid, Ui};
use serde_json as json;
use settings_schema::SchemaNode;

pub struct Array {
    id: String,
    contents: Vec<(Box<dyn SettingControl>, Box<dyn SettingContainer>)>,
}

impl Array {
    pub fn new(
        schema_array: Vec<SchemaNode>,
        session_fragment: json::Value,
        trans_path: &str,
        trans: &TranslationBundle,
    ) -> Self {
        let mut session_array = json::from_value::<Vec<json::Value>>(session_fragment).unwrap();

        Self {
            id: format!("array{}", super::get_id()),
            contents: schema_array
                .into_iter()
                .map(|schema| {
                    let session_fragment = session_array.remove(0);
                    (
                        super::create_setting_control(
                            schema.clone(),
                            session_fragment.clone(),
                            trans_path,
                            trans,
                        ),
                        super::create_setting_container(
                            schema,
                            session_fragment,
                            trans_path,
                            trans,
                        ),
                    )
                })
                .collect(),
        }
    }
}

impl SettingContainer for Array {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        ctx: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let session_array = json::from_value::<VecDeque<json::Value>>(session_fragment).unwrap();

        super::container(ui, |ui| {
            ui.group(|ui| {
                Grid::new(&self.id)
                    .striped(true)
                    .show(ui, |ui| {
                        let mut response = None;
                        for (idx, (control, container)) in self.contents.iter_mut().enumerate() {
                            let session_fragment = session_array.get(idx).cloned().unwrap();
                            let entry_response = ui
                                .horizontal(|ui| control.ui(ui, session_fragment.clone(), ctx))
                                .inner;

                            let entry_response = ui
                                .horizontal(|ui| container.ui(ui, session_fragment, ctx))
                                .inner
                                .or(entry_response);

                            ui.end_row();

                            response = response.or_else({
                                || {
                                    super::map_fragment(entry_response, |res| {
                                        let mut session_array = session_array.clone();
                                        *session_array.get_mut(idx).unwrap() = res;
                                        session_array
                                    })
                                }
                            });
                        }

                        response
                    })
                    .inner
            })
            .inner
        })
    }
}
