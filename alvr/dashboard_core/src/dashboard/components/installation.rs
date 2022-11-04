use crate::{dashboard::DashboardResponse, translation::TranslationBundle};
use egui::{Color32, Frame, RichText, Ui};

pub struct InstallationTab {}

pub enum InstallationResponse {
    UnregisterDriver(String),
    RegisterDriver,
    SetupWizard,
    AddFirewallRules,
    RemoveFirewallRules,
}

impl InstallationTab {
    pub fn new(trans: &TranslationBundle) -> Self {
        Self {}
    }

    pub fn ui(&self, ui: &mut Ui, drivers: &Vec<String>) -> Option<DashboardResponse> {
        let mut response = None;
        ui.vertical(|ui| {
            if ui.button("Run setup wizard").clicked() {
                response = Some(InstallationResponse::SetupWizard);
            }
            ui.horizontal(|ui| {
                if ui.button("Add firewall rules").clicked() {
                    response = Some(InstallationResponse::AddFirewallRules);
                }
                if ui.button("Remove firewall rules").clicked() {
                    response = Some(InstallationResponse::RemoveFirewallRules);
                }
            });
            Frame::group(ui.style())
                .fill(Color32::DARK_GRAY.linear_multiply(0.2))
                .show(ui, |ui| {
                    ui.label(RichText::new("Registered drivers").size(20.0));
                    for driver in drivers {
                        ui.horizontal(|ui| {
                            ui.label(driver);
                            if ui.button("Remove").clicked() {
                                response =
                                    Some(InstallationResponse::UnregisterDriver(driver.to_owned()))
                            }
                        });
                    }
                });
            if ui.button("Register ALVR driver").clicked() {
                response = Some(InstallationResponse::RegisterDriver);
            }
        });
        match response {
            Some(response) => Some(DashboardResponse::Installation(response)),
            None => None,
        }
    }
}
