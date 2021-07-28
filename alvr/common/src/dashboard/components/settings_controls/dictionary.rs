use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::SettingContainer;

pub struct Dictionary {}

impl SettingContainer for Dictionary {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}