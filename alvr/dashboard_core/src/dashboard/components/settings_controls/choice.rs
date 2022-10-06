use crate::{dashboard::basic_components, translation::TranslationBundle, LocalizedId};

use super::{
    EmptyContainer, EmptyControl, SettingContainer, SettingControl, SettingsContext,
    SettingsResponse,
};
use egui::Ui;
use serde_json as json;
use settings_schema::EntryData;
use std::collections::HashMap;

pub struct ChoiceControl {
    default: LocalizedId,
    variant_labels: Vec<LocalizedId>,
    controls: HashMap<String, (Box<dyn SettingControl>, bool)>,
}

impl ChoiceControl {
    pub fn new(
        default: String,
        variants_schema: Vec<(String, Option<EntryData>)>,
        session_fragment: json::Value,
        trans_path: &str,
        trans: &TranslationBundle,
    ) -> Self {
        let mut session_variants =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();

        Self {
            default: LocalizedId {
                id: default.clone(),
                trans: trans.attribute(trans_path, &default),
            },
            variant_labels: variants_schema
                .iter()
                .map(|(id, _)| LocalizedId {
                    id: id.clone(),
                    trans: trans.attribute(trans_path, id),
                })
                .collect(),
            controls: variants_schema
                .into_iter()
                .map(|(id, data)| {
                    if let Some(data) = data {
                        (
                            id.clone(),
                            (
                                super::create_setting_control(
                                    data.content,
                                    session_variants.remove(&id).unwrap(),
                                    &format!("{}-{}", trans_path, id),
                                    trans,
                                ),
                                data.advanced,
                            ),
                        )
                    } else {
                        (id, (Box::new(EmptyControl) as _, false))
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
        ctx: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let mut session_variants =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();
        let mut variant =
            json::from_value(session_variants.get("variant").cloned().unwrap()).unwrap();

        let response =
            basic_components::button_group_clicked(ui, &self.variant_labels, &mut variant).then(
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
            &format!("\"{}\"", self.default.trans),
            &ctx.t,
        )
        .then(|| {
            session_variants.insert(
                "variant".to_owned(),
                json::to_value(&*self.default).unwrap(),
            );
            super::into_fragment(&session_variants)
        })
        .or(response);

        let (control, advanced) = self.controls.get_mut(&variant).unwrap();
        let session_variant = session_variants
            .get(&variant)
            .cloned()
            .unwrap_or(json::Value::Null);

        (!*advanced || ctx.advanced)
            .then(|| {
                super::map_fragment(control.ui(ui, session_variant, ctx), |session_variant| {
                    session_variants.insert(variant, session_variant);

                    session_variants
                })
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
        trans_path: &str,
        trans: &TranslationBundle,
    ) -> Self {
        let mut session_variants =
            json::from_value::<HashMap<String, json::Value>>(session_fragment).unwrap();

        Self {
            containers: variants_schema
                .into_iter()
                .map(|(id, data)| {
                    if let Some(data) = data {
                        (
                            id.clone(),
                            (
                                super::create_setting_container(
                                    data.content,
                                    session_variants.remove(&id).unwrap(),
                                    &format!("{}-{}", trans_path, id),
                                    trans,
                                ),
                                data.advanced,
                            ),
                        )
                    } else {
                        (id, (Box::new(EmptyContainer) as _, false))
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
