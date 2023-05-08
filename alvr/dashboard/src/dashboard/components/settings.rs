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
    game_audio_preset: PresetControl,
    microphone_preset: PresetControl,
    eye_face_tracking_preset: PresetControl,
    advanced_grid_id: usize,
    session_settings_json: json::Value,
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
            presets_grid_id: get_id(),
            resolution_preset: PresetControl::new(builtin_schema::resolution_schema()),
            framerate_preset: PresetControl::new(builtin_schema::framerate_schema()),
            encoder_preset: PresetControl::new(builtin_schema::encoder_preset_schema()),
            game_audio_preset: PresetControl::new(builtin_schema::null_preset_schema()),
            microphone_preset: PresetControl::new(builtin_schema::null_preset_schema()),
            eye_face_tracking_preset: PresetControl::new(builtin_schema::eye_face_tracking_schema()),
            advanced_grid_id: get_id(),
            session_settings_json: json::to_value(session_settings).unwrap(),
            root_control: SettingControl::new(nesting_info, schema),
        }
    }

    pub fn update_session(&mut self, session_settings: &SessionSettings) {
        self.session_settings_json = json::to_value(session_settings).unwrap();

        self.resolution_preset
            .update_session_settings(&self.session_settings_json);
        self.framerate_preset
            .update_session_settings(&self.session_settings_json);
        self.encoder_preset
            .update_session_settings(&self.session_settings_json);
        self.game_audio_preset
            .update_session_settings(&self.session_settings_json);
        self.microphone_preset
            .update_session_settings(&self.session_settings_json);
        self.eye_face_tracking_preset
            .update_session_settings(&self.session_settings_json);
    }

    pub fn update_audio_devices(&mut self, list: AudioDevicesList) {
        let mut all_devices = list.output.clone();
        all_devices.extend(list.input);

        self.game_audio_preset = PresetControl::new(builtin_schema::game_audio_schema(all_devices));
        self.game_audio_preset
            .update_session_settings(&self.session_settings_json);

        self.microphone_preset = PresetControl::new(builtin_schema::microphone_schema(list.output));
        self.microphone_preset
            .update_session_settings(&self.session_settings_json);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<ServerRequest> {
        let mut requests = vec![];

        ui.heading("Presets");
        ScrollArea::new([true, false])
            .id_source(self.presets_grid_id)
            .show(ui, |ui| {
                Grid::new(self.presets_grid_id)
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        requests.extend(self.resolution_preset.ui(ui));
                        ui.end_row();

                        requests.extend(self.framerate_preset.ui(ui));
                        ui.end_row();

                        requests.extend(self.encoder_preset.ui(ui));
                        ui.end_row();

                        requests.extend(self.game_audio_preset.ui(ui));
                        ui.end_row();

                        requests.extend(self.microphone_preset.ui(ui));
                        ui.end_row();

                        requests.extend(self.eye_face_tracking_preset.ui(ui));
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
                        if let Some(request) =
                            self.root_control
                                .ui(ui, &mut self.session_settings_json, false)
                        {
                            requests.push(request);
                        }

                        ui.end_row();
                    })
            });

        if !requests.is_empty() {
            Some(ServerRequest::SetValues(requests))
        } else {
            None
        }
    }
}
