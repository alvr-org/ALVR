use super::{
    notice,
    presets::{builtin_schema, PresetControl},
    NestingInfo, SettingControl,
};
use crate::dashboard::{get_id, ServerRequest};
use alvr_packets::AudioDevicesList;
use alvr_session::{SessionSettings, Settings};
use eframe::egui::{Grid, ScrollArea, Ui};
use serde_json as json;

pub struct SettingsTab {
    presets_grid_id: usize,
    resolution_preset: PresetControl,
    framerate_preset: PresetControl,
    encoder_preset: PresetControl,
    game_audio_preset: Option<PresetControl>,
    microphone_preset: Option<PresetControl>,
    eye_face_tracking_preset: PresetControl,
    advanced_grid_id: usize,
    session_settings_json: Option<json::Value>,
    root_control: SettingControl,
}

impl SettingsTab {
    pub fn new() -> Self {
        let nesting_info = NestingInfo {
            path: vec!["session_settings".into()],
            indentation_level: 0,
        };
        let schema = Settings::schema(alvr_session::session_settings_default());

        Self {
            presets_grid_id: get_id(),
            resolution_preset: PresetControl::new(builtin_schema::resolution_schema()),
            framerate_preset: PresetControl::new(builtin_schema::framerate_schema()),
            encoder_preset: PresetControl::new(builtin_schema::encoder_preset_schema()),
            game_audio_preset: None,
            microphone_preset: None,
            eye_face_tracking_preset: PresetControl::new(builtin_schema::eye_face_tracking_schema()),
            advanced_grid_id: get_id(),
            session_settings_json: None,
            root_control: SettingControl::new(nesting_info, schema),
        }
    }

    pub fn update_session(&mut self, session_settings: &SessionSettings) {
        let settings_json = json::to_value(session_settings).unwrap();

        self.resolution_preset
            .update_session_settings(&settings_json);
        self.framerate_preset
            .update_session_settings(&settings_json);
        self.encoder_preset.update_session_settings(&settings_json);
        if let Some(preset) = self.game_audio_preset.as_mut() {
            preset.update_session_settings(&settings_json)
        }
        if let Some(preset) = self.microphone_preset.as_mut() {
            preset.update_session_settings(&settings_json)
        }
        self.eye_face_tracking_preset
            .update_session_settings(&settings_json);

        self.session_settings_json = Some(settings_json);
    }

    pub fn update_audio_devices(&mut self, list: AudioDevicesList) {
        let mut all_devices = list.output.clone();
        all_devices.extend(list.input);

        let settings_json = self
            .session_settings_json
            .clone()
            .unwrap_or_else(|| json::to_value(alvr_session::session_settings_default()).unwrap());

        let mut preset = PresetControl::new(builtin_schema::game_audio_schema(all_devices));
        preset.update_session_settings(&settings_json);
        self.game_audio_preset = Some(preset);

        let mut preset = PresetControl::new(builtin_schema::microphone_schema(list.output));
        preset.update_session_settings(&settings_json);
        self.microphone_preset = Some(preset);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Vec<ServerRequest> {
        let mut requests = vec![];

        if self.session_settings_json.is_none() {
            requests.push(ServerRequest::GetSession);
        }

        if self.game_audio_preset.is_none() {
            requests.push(ServerRequest::GetAudioDevices);
        }

        let mut path_value_pairs = vec![];

        ui.heading("Presets");
        ScrollArea::new([true, false])
            .id_source(self.presets_grid_id)
            .show(ui, |ui| {
                Grid::new(self.presets_grid_id)
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        path_value_pairs.extend(self.resolution_preset.ui(ui));
                        ui.end_row();

                        path_value_pairs.extend(self.framerate_preset.ui(ui));
                        ui.end_row();

                        path_value_pairs.extend(self.encoder_preset.ui(ui));
                        ui.end_row();

                        if let Some(preset) = &mut self.game_audio_preset {
                            path_value_pairs.extend(preset.ui(ui));
                            ui.end_row();
                        }

                        if let Some(preset) = &mut self.microphone_preset {
                            path_value_pairs.extend(preset.ui(ui));
                            ui.end_row();
                        }

                        path_value_pairs.extend(self.eye_face_tracking_preset.ui(ui));
                        ui.end_row();
                    })
            });

        ui.add_space(15.0);

        ui.horizontal(|ui| {
            ui.heading("All Settings (Advanced)");
            notice::notice(ui, "Changing some advanced settings may break ALVR");
        });
        ScrollArea::new([true, false])
            .id_source(self.advanced_grid_id)
            .show(ui, |ui| {
                Grid::new(self.advanced_grid_id)
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        if let Some(json) = &mut self.session_settings_json {
                            if let Some(pair) = self.root_control.ui(ui, json, false) {
                                path_value_pairs.push(pair);
                            }
                        }

                        ui.end_row();
                    })
            });

        if !path_value_pairs.is_empty() {
            requests.push(ServerRequest::SetValues(path_value_pairs));
        }

        requests
    }
}
