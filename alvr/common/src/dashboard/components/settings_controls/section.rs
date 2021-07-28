use super::{SettingContainer, SettingControl};
use crate::dashboard::DashboardResponse;
use egui::Ui;
use serde_json as json;
use settings_schema::EntryData;
use std::collections::HashMap;

struct Entry {
    name: String,
    display_name: String,
    advanced: String,
    setting_control: Box<dyn SettingControl>,
    setting_container: Box<dyn SettingContainer>,
}

pub struct Section {
    entries: Vec<Entry>,
}

impl Section {
    pub fn new(entries: Vec<(String, Option<EntryData>)>) -> Self {
        Self { entries: vec![] }
    }
}

impl SettingContainer for Section {
    fn update(
        &mut self,
        ui: &mut Ui,
        session: json::Value,
        advanced: bool,
    ) -> Option<DashboardResponse> {
        None
    }
}
