use std::str::FromStr;

use crate::dashboard::{basic_components, get_id, DisplayString};
use alvr_session::SessionSettings;
use alvr_sockets::{AudioDevicesList, DashboardRequest};
use eframe::{
    egui::{ComboBox, Ui},
    emath::Numeric,
};
use json::Number;
use serde_json as json;

fn set_string_request(path: &str, value: &str) -> DashboardRequest {
    DashboardRequest::SetSingleValue {
        path: alvr_sockets::parse_path(path),
        new_value: json::Value::String(value.into()),
    }
}

fn set_f64_request(path: &str, value: f64) -> DashboardRequest {
    DashboardRequest::SetSingleValue {
        path: alvr_sockets::parse_path(path),
        new_value: json::Value::Number(Number::from_f64(value).unwrap()),
    }
}

pub struct Presets {
    resolution_list: Vec<DisplayString>,
    resolution_selection: String,
    audio_devices: AudioDevicesList,
    game_audio_selection: usize,
    game_audio_id: usize,
    microphone_input_selection: usize,
    microphone_input_id: usize,
    microphone_output_selection: usize,
    microphone_output_id: usize,
}

impl Presets {
    pub fn new() -> Self {
        Self {
            resolution_list: vec![
                DisplayString {
                    id: "0.25".into(),
                    display: "25%".into(),
                },
                DisplayString {
                    id: "0.5".into(),
                    display: "50%".into(),
                },
                DisplayString {
                    id: "0.75".into(),
                    display: "75%".into(),
                },
                DisplayString {
                    id: "1.0".into(),
                    display: "100%".into(),
                },
                DisplayString {
                    id: "1.25".into(),
                    display: "125%".into(),
                },
                DisplayString {
                    id: "1.5".into(),
                    display: "150%".into(),
                },
            ],
            resolution_selection: "75".into(),
            audio_devices: AudioDevicesList {
                output: vec![],
                input: vec![],
            },
            game_audio_selection: 0,
            game_audio_id: get_id(),
            microphone_input_selection: 0,
            microphone_input_id: get_id(),
            microphone_output_selection: 0,
            microphone_output_id: get_id(),
        }
    }

    pub fn session_updated(&mut self, session_settings: &SessionSettings) {
        // todo: udpate presets selection after settings change
    }

    pub fn update_audio_devices(&mut self, list: AudioDevicesList) {
        self.audio_devices = list;
        self.game_audio_selection = 0;
        self.microphone_input_selection = 0;
        self.microphone_output_selection = 0;
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Vec<DashboardRequest> {
        let mut requests = vec![];

        ui.columns(2, |ui| {
            ui[0].label("Resolution");
            ui[0].columns(6, |ui| {
                for (i, id) in self.resolution_list.iter().enumerate() {
                    if ui[i]
                        .selectable_value(
                            &mut self.resolution_selection,
                            (**id).clone(),
                            &id.display,
                        )
                        .clicked()
                    {
                        let value = self.resolution_selection.parse().unwrap();
                        requests.push(set_string_request(
                            "session_settings.video.render_resolution.variant",
                            "Scale",
                        ));
                        requests.push(set_f64_request(
                            "session_settings.video.render_resolution.Scale",
                            value,
                        ));
                        requests.push(set_string_request(
                            "session_settings.video.recommended_target_resolution.variant",
                            "Scale",
                        ));
                        requests.push(set_f64_request(
                            "session_settings.video.recommended_target_resolution.Scale",
                            value,
                        ));
                    }
                }
            });

            let response = ComboBox::new(self.game_audio_id, "Game audio").show_index(
                &mut ui[1],
                &mut self.game_audio_selection,
                self.audio_devices.output.len() + 1,
                |idx| {
                    if idx == 0 {
                        "Default".into()
                    } else {
                        self.audio_devices.output[idx - 1].clone()
                    }
                },
            );

            if response.changed() {
                if self.game_audio_selection == 0 {
                    requests.push(set_string_request(
                        "session_settings.audio.game_audio.content.device_id.variant",
                        "Default",
                    ));
                } else {
                    requests.push(set_string_request(
                        "session_settings.audio.game_audio.content.device_id.variant",
                        "Name",
                    ));
                    requests.push(set_string_request(
                        "session_settings.audio.game_audio.content.device_id.Name",
                        &self.audio_devices.output[self.game_audio_selection - 1],
                    ));
                }
            }
        });

        requests
    }
}
