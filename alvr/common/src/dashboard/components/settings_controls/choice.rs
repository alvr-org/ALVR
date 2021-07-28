use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::SettingContainer;

pub struct ChoiceControl {}

impl ChoiceControl {
    pub fn update(
        ui: &mut Ui,
        session: &json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}

pub struct ChoiceContainer {}

impl SettingContainer for ChoiceContainer {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}