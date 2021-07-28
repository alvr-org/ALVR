use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

use super::SettingContainer;

pub struct Text {
    default: String,
}

impl Text {
    pub fn new(default: String) -> Self {
        Self { default }
    }
}

impl SettingContainer for Text {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}
