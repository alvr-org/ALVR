use super::SettingContainer;
use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;

pub struct AudioDropdown {}

impl SettingContainer for AudioDropdown {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}
