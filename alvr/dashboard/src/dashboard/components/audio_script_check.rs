use alvr_packets::{PathValuePair, ServerRequest};
use eframe::egui::{Align, Layout, RichText, Ui};

pub struct AudioScriptCheck;

pub enum AudioScriptCheckRequest {
    ServerRequest(ServerRequest),
}

impl AudioScriptCheck {
    pub fn new() -> Self {
        Self {}
    }
    pub fn ui(&mut self, ui: &mut Ui) -> Option<AudioScriptCheckRequest> {
        let mut request = None;

        ui.horizontal(|ui| {
            ui.add_space(60.0);
            ui.vertical(|ui| {
                ui.add_space(30.0);
                ui.heading(RichText::new("Outdated audio script detected").size(30.0));
                ui.add_space(5.0);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(15.0);
                ui.button("❌")
            })
        });
        ui.separator();
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            ui.add_space(60.0);
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.add_space(60.0);
                ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                    ui.add_space(15.0);
                    ui.heading("To reset outdated On connect / On disconnect scripts press button bellow");
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("Reset scripts").clicked() {
                            request = Some(AudioScriptCheckRequest::ServerRequest(
                                ServerRequest::SetValues(vec![
                                    PathValuePair {
                                        path: alvr_packets::parse_path(
                                            "session_settings.connection.on_connect_script",
                                        ),
                                        value: serde_json::Value::String(String::default()),
                                    },
                                    PathValuePair {
                                        path: alvr_packets::parse_path(&format!(
                                            "session_settings.connection.{}",
                                            "on_disconnect_script"
                                        )),
                                        value: serde_json::Value::String(String::default()),
                                    },
                                ]),
                            ));
                        }
                    });
                });
            })
        });

        request
    }
}
