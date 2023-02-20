// use super::{Section, SettingsContext, SettingsResponse};
use super::{presets::Presets, NestingInfo, SettingControl};
use crate::dashboard::{get_id, DashboardRequest};
use alvr_session::{SessionSettings, Settings};
use alvr_sockets::AudioDevicesList;
use eframe::egui::{Grid, ScrollArea, Ui};
use serde_json as json;

pub struct SettingsTab {
    advanced_grid_id: usize,
    session_settings_json: json::Value,
    presets: Presets,
    root_control: SettingControl,
}

impl SettingsTab {
    pub fn new() -> Self {
        let session_settings = alvr_session::session_settings_default();

        let nesting_info = NestingInfo {
            path: vec!["session_settings".into()],
            indentation_level: 0,
        };
        let schema = Settings::schema(session_settings.clone());

        Self {
            advanced_grid_id: get_id(),
            session_settings_json: json::to_value(session_settings).unwrap(),
            presets: Presets::new(),
            root_control: SettingControl::new(nesting_info, schema),
        }
    }

    pub fn update_session(&mut self, session_settings: &SessionSettings) {
        self.session_settings_json = json::to_value(session_settings).unwrap();
        self.presets.session_updated(session_settings);
    }

    pub fn update_audio_devices(&mut self, list: AudioDevicesList) {
        self.presets.update_audio_devices(list);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Vec<DashboardRequest> {
        ui.heading("Presets");
        let mut requests = self.presets.ui(ui);

        ui.add_space(15.0);

        ui.heading("Advanced");
        ScrollArea::new([true, false]).show(ui, |ui| {
            Grid::new(self.advanced_grid_id)
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    let request = self
                        .root_control
                        .ui(ui, &mut self.session_settings_json, false);

                    if let Some(request) = request {
                        requests.push(request);
                    }

                    ui.end_row();
                })
        });

        requests
    }
}
