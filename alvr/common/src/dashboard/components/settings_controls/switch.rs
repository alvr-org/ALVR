use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::SettingContainer;

pub struct SwitchControl {}

impl SwitchControl {
    pub fn update(
        ui: &mut Ui,
        session: &json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}

pub struct SwitchContainer {}

impl SettingContainer for SwitchContainer {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}