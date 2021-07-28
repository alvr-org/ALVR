use super::{settings_controls::create_setting_container, Section, SettingContainer};
use crate::{
    dashboard::{basic_components::tabs, DashboardResponse},
    data::{self, SessionDesc},
};
use egui::Ui;
use serde_json as json;
use settings_schema::SchemaNode;
use std::collections::HashMap;

pub struct SettingsTab {
    selected_tab: String,
    advanced: bool,
    tabs: Vec<(String, String, Box<dyn SettingContainer>)>,
}

impl SettingsTab {
    pub fn new() -> Self {
        let schema = data::settings_schema(data::session_settings_default());

        if let SchemaNode::Section { entries } = schema {
            Self {
                selected_tab: entries[0].0.clone(),
                advanced: false,
                // todo: get translation
                tabs: entries
                    .into_iter()
                    .filter_map(|(name, data)| {
                        if let Some(data) = data {
                            Some((
                                name.clone(),
                                name.clone(),
                                create_setting_container(data.content),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect(),
            }
        } else {
            panic!("Invalid schema!")
        }
    }

    pub fn update(&mut self, ui: &mut Ui, session: &SessionDesc) -> Option<DashboardResponse> {
        let selected_tab = &mut self.selected_tab;
        // let tabs_entries = &mut self.tabs;
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
        let advanced = &mut self.advanced;

        let mut content_session = json::from_value::<HashMap<String, json::Value>>(
            json::to_value(&session.session_settings).unwrap(),
        )
        .unwrap()
        .remove(selected_tab)
        .unwrap();

        tabs(
            ui,
            tabs_list,
            selected_tab,
            {
                let advanced = *advanced;
                move |ui| content.update(ui, content_session, advanced)
            },
            |ui| {
                if ui.selectable_label(*advanced, "Advanced").clicked() {
                    *advanced = !*advanced;
                }
            },
        )
    }
}
