use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::{SettingContainer, SettingsContext, SettingsResponse};

pub struct Dictionary {}

impl SettingContainer for Dictionary {
    fn ui(
        &mut self,
        ui: &mut Ui,
        session_fragment: json::Value,
        context: &SettingsContext,
    ) -> Option<SettingsResponse> {
        None
    }
}
