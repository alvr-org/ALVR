use alvr_gui_common::theme;
use alvr_packets::{FirewallRulesAction, ServerRequest};
use eframe::{
    egui::{Frame, Grid, Layout, RichText, Ui},
    emath::Align,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const DRIVER_UPDATE_INTERVAL: Duration = Duration::from_secs(1);

pub enum InstallationTabRequest {
    OpenSetupWizard,
    ServerRequest(ServerRequest),
}

pub struct InstallationTab {
    drivers: Vec<PathBuf>,
    last_update_instant: Instant,
}

impl InstallationTab {
    pub fn new() -> Self {
        Self {
            drivers: vec![],
            last_update_instant: Instant::now(),
        }
    }

    pub fn update_drivers(&mut self, list: Vec<PathBuf>) {
        self.drivers = list;
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Vec<InstallationTabRequest> {
        let mut requests = vec![];

        let now = Instant::now();
        if now > self.last_update_instant + DRIVER_UPDATE_INTERVAL {
            requests.push(InstallationTabRequest::ServerRequest(
                ServerRequest::GetDriverList,
            ));

            self.last_update_instant = now;
        }

        ui.vertical_centered_justified(|ui| {
            if ui.button("Run setup wizard").clicked() {
                requests.push(InstallationTabRequest::OpenSetupWizard);
            }
            ui.columns(2, |ui| {
                if ui[0].button("Add firewall rules").clicked() {
                    requests.push(InstallationTabRequest::ServerRequest(
                        ServerRequest::FirewallRules(FirewallRulesAction::Add),
                    ));
                }
                if ui[1].button("Remove firewall rules").clicked() {
                    requests.push(InstallationTabRequest::ServerRequest(
                        ServerRequest::FirewallRules(FirewallRulesAction::Remove),
                    ));
                }
            });

            Frame::group(ui.style())
                .fill(theme::SECTION_BG)
                .show(ui, |ui| {
                    ui.label(RichText::new("Registered drivers").size(18.0));
                    Grid::new(0).num_columns(2).show(ui, |ui| {
                        for driver_path in &self.drivers {
                            ui.label(driver_path.to_string_lossy());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Remove").clicked() {
                                    requests.push(InstallationTabRequest::ServerRequest(
                                        ServerRequest::UnregisterDriver(driver_path.clone()),
                                    ));
                                }
                            });
                            ui.end_row();
                        }
                    });

                    if ui.button("Register ALVR driver").clicked() {
                        requests.push(InstallationTabRequest::ServerRequest(
                            ServerRequest::RegisterAlvrDriver,
                        ));
                    }
                });
        });

        requests
    }
}
