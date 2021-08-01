use crate::dashboard::basic_components;

use super::{
    EmptyContainer, EmptyControl, SettingContainer, SettingControl, SettingsContext,
    SettingsResponse,
};
use egui::Ui;
use serde_json as json;
use settings_schema::EntryData;
use std::collections::HashMap;

pub struct ChoiceControl {
    default: String,
    display_default: String,
    variants_list: Vec<(String, String)>,
    controls: HashMap<String, (Box<dyn SettingControl>, bool)>,
}

impl ChoiceControl {
    pub fn new(
        default: String,
        variants_schema: Vec<(String, Option<EntryData>)>,
        session_fragment: json::Value,
    ) -> Self {
        let mut session_variants =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();

        Self {
            default: default.clone(),
            display_default: default,
            variants_list: variants_schema
                .iter()
                .map(|(name, _)| {
                    let display_name = name.clone();

                    (name.clone(), display_name)
                })
                .collect(),
            controls: variants_schema
                .into_iter()
                .map(|(name, data)| {
                    if let Some(data) = data {
                        (
                            name.clone(),
                            (
                                super::create_setting_control(
                                    data.content,
                                    session_variants.remove(&name).unwrap(),
                                ),
                                data.advanced,
                            ),
                        )
                    } else {
                        (name, (Box::new(EmptyControl) as _, false))
                    }
                })
                .collect(),
        }
    }
}

impl SettingControl for ChoiceControl {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let mut session_variants =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();
        let mut variant =
            json::from_value(session_variants.get("variant").cloned().unwrap()).unwrap();

        let response =
            basic_components::button_group_clicked(ui, &self.variants_list, &mut variant).then(
                || {
                    session_variants
                        .insert("variant".to_owned(), json::to_value(&variant).unwrap());

                    super::into_fragment(&session_variants)
                },
            );

        let response = super::reset_clicked(
            ui,
            &variant,
            &self.default,
            &format!("\"{}\"", self.display_default),
        )
        .then(|| {
            session_variants.insert("variant".to_owned(), json::to_value(&self.default).unwrap());
            super::into_fragment(&session_variants)
        })
        .or(response);

        let (control, advanced) = self.controls.get_mut(&variant).unwrap();
        let session_variant = session_variants
            .get(&variant)
            .cloned()
            .unwrap_or(json::Value::Null);

        (!*advanced || context.advanced)
            .then(|| {
                super::map_fragment(
                    control.ui(ui, session_variant, context),
                    |session_variant| {
                        session_variants.insert(variant, session_variant);

                        session_variants
                    },
                )
            })
            .flatten()
            .or(response)
    }
}

pub struct ChoiceContainer {
    containers: HashMap<String, (Box<dyn SettingContainer>, bool)>,
}

impl ChoiceContainer {
    pub fn new(
        variants_schema: Vec<(String, Option<EntryData>)>,
        session_fragment: json::Value,
    ) -> Self {
        let mut session_variants =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();

        Self {
            containers: variants_schema
                .into_iter()
                .map(|(name, data)| {
                    if let Some(data) = data {
                        (
                            name.clone(),
                            (
                                super::create_setting_container(
                                    data.content,
                                    session_variants.remove(&name).unwrap(),
                                ),
                                data.advanced,
                            ),
                        )
                    } else {
                        (name, (Box::new(EmptyContainer) as _, false))
                    }
                })
                .collect(),
        }
    }
}

impl SettingContainer for ChoiceContainer {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let mut session_variants =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();
        let variant = json::from_value(session_variants.get("variant").cloned().unwrap()).unwrap();

        let (control, advanced) = self.containers.get_mut(&variant).unwrap();
        let session_variant = session_variants
            .get(&variant)
            .cloned()
            .unwrap_or(json::Value::Null);

        (!*advanced || context.advanced)
            .then(|| {
                super::map_fragment(
                    control.ui(ui, session_variant, context),
                    |session_variant| {
                        session_variants.insert(variant, session_variant);

                        session_variants
                    },
                )
            })
            .flatten()
    }
}
