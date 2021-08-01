use super::{SettingContainer, SettingsContext, SettingsResponse};
use egui::Ui;
use serde_json as json;

pub struct Array {}

impl SettingContainer for Array {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        None
    }
}
