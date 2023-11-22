use crate::dashboard::ServerRequest;
use alvr_common::ConnectionState;
use alvr_gui_common::theme::{self, log_colors};
use alvr_packets::ClientListAction;
use alvr_session::{ClientConnectionConfig, SessionConfig};
use eframe::{
    egui::{self, Frame, Grid, Layout, RichText, TextEdit, Ui, Window},
    emath::{Align, Align2},
    epaint::Color32,
};

struct EditPopupState {
    new_client: bool,
    hostname: String,
    ips: Vec<String>,
}

pub struct ConnectionsTab {
    new_clients: Option<Vec<(String, ClientConnectionConfig)>>,
    trusted_clients: Option<Vec<(String, ClientConnectionConfig)>>,
    edit_popup_state: Option<EditPopupState>,
}

impl ConnectionsTab {
    pub fn new() -> Self {
        Self {
            new_clients: None,
            trusted_clients: None,
            edit_popup_state: None,
        }
    }

    pub fn update_client_list(&mut self, session: &SessionConfig) {
        let (trusted_clients, untrusted_clients) =
            session
                .client_connections
                .clone()
                .into_iter()
                .partition::<Vec<_>, _>(|(_, data)| data.trusted);

        self.trusted_clients = Some(trusted_clients);
        self.new_clients = Some(untrusted_clients);
    }

    pub fn ui(&mut self, ui: &mut Ui, connected_to_server: bool) -> Vec<ServerRequest> {
        let mut requests = vec![];

        if self.new_clients.is_none() {
            requests.push(ServerRequest::GetSession);
        }

        if !connected_to_server {
            Frame::group(ui.style())
                .fill(log_colors::WARNING_LIGHT)
                .show(ui, |ui| {
                    Grid::new(0).num_columns(2).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.heading(
                                RichText::new(
                                    "The streamer is not connected! Clients will not be discovered",
                                )
                                .color(Color32::BLACK),
                            );
                        });

                        #[cfg(not(target_arch = "wasm32"))]
                        ui.with_layout(Layout::right_to_left(eframe::emath::Align::Center), |ui| {
                            if ui.button("Launch SteamVR").clicked() {
                                crate::steamvr_launcher::LAUNCHER.lock().launch_steamvr();
                            }
                        });
                    });
                });
        }

        ui.vertical_centered_justified(|ui| {
            if let Some(clients) = &self.new_clients {
                if let Some(request) = new_clients_section(ui, clients) {
                    requests.push(request);
                }
            }

            ui.add_space(10.0);

            if let Some(clients) = &mut self.trusted_clients {
                if let Some(request) =
                    trusted_clients_section(ui, clients, &mut self.edit_popup_state)
                {
                    requests.push(request);
                }
            }
        });

        if let Some(mut state) = self.edit_popup_state.take() {
            Window::new("Edit connection")
                .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.columns(2, |ui| {
                        ui[0].label("Hostname:");
                        ui[1].add_enabled(
                            state.new_client,
                            TextEdit::singleline(&mut state.hostname),
                        );
                        ui[0].label("IP Addresses:");
                        for address in &mut state.ips {
                            ui[1].text_edit_singleline(address);
                        }
                        if ui[1].button("Add new").clicked() {
                            state.ips.push("192.168.X.X".to_string());
                        }
                    });
                    ui.columns(2, |ui| {
                        if ui[0].button("Cancel").clicked() {
                            return;
                        }

                        if ui[1].button("Save").clicked() {
                            let manual_ips =
                                state.ips.iter().filter_map(|s| s.parse().ok()).collect();

                            if state.new_client {
                                requests.push(ServerRequest::UpdateClientList {
                                    hostname: state.hostname,
                                    action: ClientListAction::AddIfMissing {
                                        trusted: true,
                                        manual_ips,
                                    },
                                });
                            } else {
                                requests.push(ServerRequest::UpdateClientList {
                                    hostname: state.hostname,
                                    action: ClientListAction::SetManualIps(manual_ips),
                                });
                            }
                        } else {
                            self.edit_popup_state = Some(state);
                        }
                    })
                });
        }

        requests
    }
}

fn new_clients_section(
    ui: &mut Ui,
    clients: &[(String, ClientConnectionConfig)],
) -> Option<ServerRequest> {
    let mut request = None;

    Frame::group(ui.style())
        .fill(theme::SECTION_BG)
        .show(ui, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.add_space(5.0);
                ui.heading("New clients");
            });
            for (hostname, _) in clients {
                Frame::group(ui.style())
                    .fill(theme::DARKER_BG)
                    .inner_margin(egui::vec2(15.0, 12.0))
                    .show(ui, |ui| {
                        Grid::new(format!("{}-new-clients", hostname))
                            .num_columns(2)
                            .spacing(egui::vec2(8.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(hostname);
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("Trust").clicked() {
                                        request = Some(ServerRequest::UpdateClientList {
                                            hostname: hostname.clone(),
                                            action: ClientListAction::Trust,
                                        });
                                    };
                                });
                                ui.end_row();
                            });
                    });
            }
        });

    request
}

fn trusted_clients_section(
    ui: &mut Ui,
    clients: &mut [(String, ClientConnectionConfig)],
    edit_popup_state: &mut Option<EditPopupState>,
) -> Option<ServerRequest> {
    let mut request = None;

    Frame::group(ui.style())
        .fill(theme::SECTION_BG)
        .show(ui, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.add_space(5.0);
                ui.heading("Trusted clients");
            });

            ui.vertical(|ui| {
                for (hostname, data) in clients {
                    Frame::group(ui.style())
                        .fill(theme::DARKER_BG)
                        .inner_margin(egui::vec2(15.0, 12.0))
                        .show(ui, |ui| {
                            Grid::new(format!("{}-clients", hostname))
                                .num_columns(2)
                                .spacing(egui::vec2(8.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label(&data.display_name);
                                    ui.horizontal(|ui| {
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| match data.connection_state {
                                                ConnectionState::Disconnected => {
                                                    ui.colored_label(Color32::GRAY, "Disconnected")
                                                }
                                                ConnectionState::Connecting => ui.colored_label(
                                                    log_colors::WARNING_LIGHT,
                                                    "Connecting",
                                                ),
                                                ConnectionState::Connected => {
                                                    ui.colored_label(theme::OK_GREEN, "Connected")
                                                }
                                                ConnectionState::Streaming => {
                                                    ui.colored_label(theme::OK_GREEN, "Streaming")
                                                }
                                                ConnectionState::Disconnecting { .. } => ui
                                                    .colored_label(
                                                        log_colors::WARNING_LIGHT,
                                                        "Disconnecting",
                                                    ),
                                            },
                                        );
                                    });

                                    ui.end_row();

                                    ui.label(format!(
                                        "{hostname}: {}",
                                        data.current_ip
                                            .map(|ip| ip.to_string())
                                            .unwrap_or_else(|| "Unknown IP".into()),
                                    ));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.button("Remove").clicked() {
                                            request = Some(ServerRequest::UpdateClientList {
                                                hostname: hostname.clone(),
                                                action: ClientListAction::RemoveEntry,
                                            });
                                        }
                                        if ui.button("Edit").clicked() {
                                            *edit_popup_state = Some(EditPopupState {
                                                new_client: false,
                                                hostname: hostname.to_owned(),
                                                ips: data
                                                    .manual_ips
                                                    .iter()
                                                    .map(|addr| addr.to_string())
                                                    .collect::<Vec<String>>(),
                                            });
                                        }
                                    });
                                });
                        });
                }
            });

            if ui.button("Add client manually").clicked() {
                *edit_popup_state = Some(EditPopupState {
                    hostname: "XXXX.client.alvr".into(),
                    new_client: true,
                    ips: Vec::new(),
                });
            }
        });

    request
}
