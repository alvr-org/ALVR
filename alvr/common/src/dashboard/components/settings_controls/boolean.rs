use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::SettingContainer;

pub struct Boolean {}

impl SettingContainer for Boolean {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}