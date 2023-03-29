use crate::{firewall, steamvr_launcher::LAUNCHER, theme};
use alvr_common::prelude::*;
use eframe::{
    egui::{Frame, Grid, Layout, RichText, Ui},
    emath::Align,
};
use std::path::PathBuf;

pub enum InstallationTabRequest {
    OpenSetupWizard,
}

pub struct InstallationTab {
    drivers: Vec<PathBuf>,
}

impl InstallationTab {
    pub fn new() -> Self {
        let mut this = Self { drivers: vec![] };

        this.update_drivers();

        this
    }

    pub fn update_drivers(&mut self) {
        if let Ok(paths) = alvr_commands::get_registered_drivers() {
            self.drivers = paths;
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<InstallationTabRequest> {
        let mut response = None;
        ui.vertical_centered_justified(|ui| {
            if ui.button("Run setup wizard").clicked() {
                response = Some(InstallationTabRequest::OpenSetupWizard);
            }
            ui.columns(2, |ui| {
                if ui[0].button("Add firewall rules").clicked() {
                    if firewall::firewall_rules(true).is_ok() {
                        info!("Setting firewall rules succeeded!");
                    } else {
                        error!("Setting firewall rules failed!");
                    }
                }
                if ui[1].button("Remove firewall rules").clicked() {
                    if firewall::firewall_rules(false).is_ok() {
                        info!("Removing firewall rules succeeded!");
                    } else {
                        error!("Removing firewall rules failed!");
                    }
                }
            });

            let mut did_driver_action = false;
            Frame::group(ui.style())
                .fill(theme::SECTION_BG)
                .show(ui, |ui| {
                    ui.label(RichText::new("Registered drivers").size(18.0));
                    Grid::new(0).num_columns(2).show(ui, |ui| {
                        for driver_path in &self.drivers {
                            ui.label(driver_path.to_string_lossy());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Remove").clicked() {
                                    LAUNCHER.lock().unregister_driver(driver_path.clone());
                                    did_driver_action = true;
                                }
                            });
                            ui.end_row();
                        }
                    });

                    if ui.button("Register ALVR driver").clicked() {
                        LAUNCHER.lock().register_alvr_driver();
                        did_driver_action = true;
                    }
                });
            if did_driver_action {
                self.update_drivers();
            }
        });

        response
    }
}
