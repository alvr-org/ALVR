use super::{Section, SettingsContext, SettingsResponse};
use crate::dashboard::{basic_components, DashboardResponse};
use alvr_common::data::{self, SessionDesc, SessionSettings};
use egui::Ui;
use serde_json as json;
use settings_schema::SchemaNode;
use std::collections::HashMap;

pub struct SettingsTab {
    selected_tab: String,
    tabs: Vec<(String, String, Section)>,
    context: SettingsContext,
}

impl SettingsTab {
    pub fn new(session_settings: &SessionSettings) -> Self {
        let schema = data::settings_schema(data::session_settings_default());
        let mut session = json::from_value::<HashMap<String, json::Value>>(
            json::to_value(session_settings).unwrap(),
        )
        .unwrap();

        if let SchemaNode::Section { entries } = schema {
            Self {
                selected_tab: entries[0].0.clone(),
                // todo: get translation
                tabs: entries
                    .into_iter()
                    .filter_map(|(name, data)| {
                        data.map(|data| {
                            if let SchemaNode::Section { entries } = data.content {
                                (
                                    name.clone(),
                                    name.clone(),
                                    Section::new(entries, session.remove(&name).unwrap()),
                                )
                            } else {
                                panic!("Invalid schema!")
                            }
                        })
                    })
                    .collect(),
                context: SettingsContext {
                    advanced: false,
                    view_width: 0_f32,
                },
            }
        } else {
            panic!("Invalid schema!")
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, session: &SessionDesc) -> Option<DashboardResponse> {
        self.context.view_width = ui.available_width();

        let selected_tab = &mut self.selected_tab;
        let tabs_list = self
            .tabs
            .iter()
            .map(|(name, display_name, _)| (name.clone(), display_name.clone()))
            .collect();

        let content = &mut self
            .tabs
            .iter_mut()
            .find(|(name, _, _)| *name == *selected_tab)
            .unwrap()
            .2;

        let mut session_tabs = json::from_value::<HashMap<String, json::Value>>(
            json::to_value(&session.session_settings).unwrap(),
        )
        .unwrap();

        let mut advanced = self.context.advanced;

        let response = basic_components::tabs(
            ui,
            tabs_list,
            selected_tab,
            {
                let selected_tab = selected_tab.clone();
                let context = &self.context;
                move |ui| {
                    content
                        .ui_no_indentation(
                            ui,
                            session_tabs.get(&selected_tab).cloned().unwrap(),
                            context,
                        )
                        .and_then(|res| match res {
                            SettingsResponse::SessionFragment(tab_session) => {
                                session_tabs.insert(selected_tab, tab_session);

                                let mut session = session.clone();
                                let session_settings = if let Ok(value) =
                                    json::from_value(json::to_value(session_tabs).unwrap())
                                {
                                    value
                                } else {
                                    //Some numeric fields are not properly validated
                                    println!("Invalid value");
                                    return None;
                                };

                                session.session_settings = session_settings;

                                Some(DashboardResponse::SessionUpdated(Box::new(session)))
                            }
                            SettingsResponse::PresetInvocation(code) => {
                                Some(DashboardResponse::PresetInvocation(code))
                            }
                        })
                }
            },
            {
                |ui| {
                    if ui.selectable_label(advanced, "Advanced").clicked() {
                        advanced = !advanced;
                    }
                }
            },
        );

        self.context.advanced = advanced;

        response
    }
}
