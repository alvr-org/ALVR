use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::SettingContainer;

pub struct NumericControl {}

impl NumericControl {
    pub fn update(ui: &mut Ui, session: &json::Value, advanced: bool) -> Option<DashboardResponse> {
        None
    }
}

pub struct NumericContainer {}

impl SettingContainer for NumericContainer {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}
