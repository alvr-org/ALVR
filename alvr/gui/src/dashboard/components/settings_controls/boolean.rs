use super::{SettingControl, SettingsContext, SettingsResponse};
use crate::dashboard::basic_components;
use egui::Ui;
use serde_json as json;

pub struct Boolean {
    default: bool,
}

impl Boolean {
    pub fn new(default: bool) -> Self {
        Self { default }
    }
}

impl SettingControl for Boolean {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        _: &SettingsContext,
    ) -> Option<SettingsResponse> {
        let mut on = json::from_value(session_fragment).unwrap();
        let response = basic_components::switch(ui, &mut on)
            .clicked()
            .then(|| super::into_fragment(on));

        super::reset_clicked(
            ui,
            &on,
            &self.default,
            if self.default { "ON" } else { "OFF" },
        )
        .then(|| super::into_fragment(self.default))
        .or(response)
    }
}
