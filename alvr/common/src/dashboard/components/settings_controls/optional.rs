use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::SettingContainer;

pub struct OptionalControl {}

impl OptionalControl {
    pub fn update(
        ui: &mut Ui,
        session: &json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}

pub struct OptionalContainer {}

impl SettingContainer for OptionalContainer {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}