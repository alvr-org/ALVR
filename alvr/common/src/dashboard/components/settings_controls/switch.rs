use super::{SettingContainer, SettingControl, SettingsContext, SettingsResponse};
use crate::dashboard::basic_components;
use egui::Ui;
use serde_json as json;
use settings_schema::{SchemaNode, SwitchDefault};

pub struct SwitchControl {
    default: bool,
    advanced: bool,
    control: Box<dyn SettingControl>,
}

impl SwitchControl {
    pub fn new(
        default_enabled: bool,
        content_advanced: bool,
        schema_content: SchemaNode,
        session: json::Value,
    ) -> Self {
        let session = json::from_value::<SwitchDefault<json::Value>>(session).unwrap();
        Self {
            default: default_enabled,
            advanced: content_advanced,
            control: super::create_setting_control(schema_content, session.content),
        }
    }
}

impl SettingControl for SwitchControl {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let mut session_switch =
            json::from_value::<SwitchDefault<json::Value>>(session_fragment).unwrap();
        let response = basic_components::switch(ui, &mut session_switch.enabled)
            .clicked()
            .then(|| super::into_fragment(&session_switch));

        let response = super::reset_clicked(
            ui,
            &session_switch.enabled,
            &self.default,
            if self.default { "ON" } else { "OFF" },
        )
        .then(|| {
            session_switch.enabled = self.default;
            super::into_fragment(&session_switch)
        })
        .or(response);

        (session_switch.enabled && (!self.advanced || context.advanced))
            .then(|| {
                super::map_fragment(
                    self.control.ui(ui, session_switch.content.clone(), context),
                    |content| {
                        session_switch.content = content;
                        session_switch
                    },
                )
            })
            .flatten()
            .or(response)
    }
}

pub struct SwitchContainer {
    advanced: bool,
    container: Box<dyn SettingContainer>,
}

impl SwitchContainer {
    pub fn new(content_advanced: bool, schema_content: SchemaNode, session: json::Value) -> Self {
        let session = json::from_value::<SwitchDefault<json::Value>>(session).unwrap();
        Self {
            advanced: content_advanced,
            container: super::create_setting_container(schema_content, session.content),
        }
    }
}

impl SettingContainer for SwitchContainer {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let mut session_switch =
            json::from_value::<SwitchDefault<json::Value>>(session_fragment).unwrap();

        (session_switch.enabled && (!self.advanced || context.advanced))
            .then(|| {
                super::map_fragment(
                    self.container
                        .ui(ui, session_switch.content.clone(), context),
                    |content| {
                        session_switch.content = content;
                        session_switch
                    },
                )
            })
            .flatten()
    }
}
