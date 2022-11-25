use crate::{dashboard::ConnectionsResponse, dashboard::DashboardResponse, theme};
use alvr_session::{ClientConnectionDesc, SessionDesc};
use egui::{Align, Frame, Layout, Resize, RichText, Ui, Window};
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr},
};

struct EditPopupState {
    hostname: String,
    display_name: String,
    ip_addresses: Vec<String>,
}

pub struct ConnectionsTab {
    edit_popup_state: Option<EditPopupState>,
}

impl ConnectionsTab {
    pub fn new() -> Self {
        Self {
            edit_popup_state: None,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, session: &SessionDesc) -> Option<DashboardResponse> {
        let mut response = None;

        // Get the different types of clients from the session
        let trusted: Vec<(&String, &ClientConnectionDesc)> = session
            .client_connections
            .iter()
            .filter_map(|(name, client_desc)| {
                if client_desc.trusted {
                    Some((name, client_desc))
                } else {
                    None
                }
            })
            .collect();
        let new: Vec<(&String, &ClientConnectionDesc)> = session
            .client_connections
            .iter()
            .filter_map(|(name, client_desc)| {
                if !client_desc.trusted {
                    Some((name, client_desc))
                } else {
                    None
                }
            })
            .collect();

        Frame::group(ui.style())
            .fill(theme::SECTION_BG)
            .show(ui, |ui| {
                ui.label(RichText::new("Trusted clients").size(18.0));
                for (name, client_desc) in trusted {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        if ui.button("Edit").clicked() {
                            self.edit_popup_state = Some(EditPopupState {
                                hostname: name.to_owned(),
                                display_name: client_desc.display_name.to_owned(),
                                ip_addresses: client_desc
                                    .manual_ips
                                    .iter()
                                    .map(|addr| addr.to_string())
                                    .collect::<Vec<String>>(),
                            });
                        }
                        if ui.button("Remove").clicked() {
                            response = Some(DashboardResponse::Connections(
                                ConnectionsResponse::RemoveEntry(name.clone()),
                            ));
                        }
                    });
                }
            });
        ui.add_space(10.0);
        Frame::group(ui.style())
            .fill(theme::SECTION_BG)
            .show(ui, |ui| {
                ui.label(RichText::new("New clients").size(18.0));
                for (name, client_desc) in new {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        if ui.button("Trust").clicked() {
                            let mut client_desc = client_desc.clone();
                            client_desc.trusted = true;
                            response = Some(DashboardResponse::Connections(
                                ConnectionsResponse::AddOrUpdate {
                                    name: name.clone(),
                                    client_desc: client_desc.clone(),
                                },
                            ));
                        };
                    });
                }
            });
        ui.add_space(10.0);
        if ui.button("Add client manually").clicked() {
            self.edit_popup_state = Some(EditPopupState {
                hostname: "x.client.alvr".to_string(),
                display_name: "Oculus Quest 2".to_string(),
                ip_addresses: Vec::new(),
            });
        }

        // We use this to close the popup if that is needed
        let mut close_popup = false;

        match self.edit_popup_state.as_mut() {
            Some(state) => {
                Window::new("Edit connection")
                    .anchor(egui::Align2::CENTER_CENTER, (0.0, 0.0))
                    .resizable(false)
                    .collapsible(false)
                    .show(ui.ctx(), |ui| {
                        ui.columns(2, |ui| {
                            ui[0].label("Hostname:");
                            ui[1].text_edit_singleline(&mut state.hostname);
                            ui[0].label("Display name:");
                            ui[1].text_edit_singleline(&mut state.display_name);
                            ui[0].label("IP Addresses");
                            if ui[1].button("Add new").clicked() {
                                state.ip_addresses.push("127.0.0.1".to_string());
                            }
                        });
                        for address in &mut state.ip_addresses {
                            ui.text_edit_singleline(address);
                        }
                        ui.columns(2, |ui| {
                            if ui[0].button("Ok").clicked() {
                                let mut ip_addresses = HashSet::new();

                                for address in &state.ip_addresses {
                                    let parts: Vec<&str> = address.splitn(4, ".").collect();
                                    let mut raw_addr: [u8; 4] = [0, 0, 0, 0];

                                    for i in 0..4 {
                                        match parts.get(i) {
                                            Some(num) => {
                                                raw_addr[i] = num.parse::<u8>().unwrap_or(0);
                                            }
                                            None => (),
                                        }
                                    }

                                    let addr = IpAddr::V4(Ipv4Addr::from(raw_addr));

                                    ip_addresses.insert(addr);
                                }

                                response = Some(DashboardResponse::Connections(
                                    ConnectionsResponse::AddOrUpdate {
                                        name: state.hostname.clone(),
                                        client_desc: ClientConnectionDesc {
                                            display_name: state.display_name.clone(),
                                            current_ip: None,
                                            manual_ips: ip_addresses,
                                            trusted: true,
                                        },
                                    },
                                ));

                                close_popup = true;
                            }
                            if ui[1].button("Cancel").clicked() {
                                close_popup = true;
                            }
                        })
                    });
            }
            None => (),
        }

        if close_popup {
            self.edit_popup_state = None;
        }

        response
    }
}
