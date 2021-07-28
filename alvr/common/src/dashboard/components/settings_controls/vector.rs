use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;
use settings_schema::VectorDefault;

use super::SettingContainer;

pub struct Vector {}

impl SettingContainer for Vector {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}
