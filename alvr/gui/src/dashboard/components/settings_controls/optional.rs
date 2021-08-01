use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::{SettingContainer, SettingsContext, SettingsResponse};

pub struct OptionalControl {}

impl OptionalControl {
    pub fn update(ui: &mut Ui, session: &json::Value, advanced: bool) -> Option<DashboardResponse> {
        None
    }
}

pub struct OptionalContainer {}

impl SettingContainer for OptionalContainer {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        None
    }
}
