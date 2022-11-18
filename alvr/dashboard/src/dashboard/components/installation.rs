use crate::{
    dashboard::{DashboardResponse, DriverResponse, FirewallRulesResponse, SetupWizardResponse},
    theme,
};
use egui::{Frame, RichText, Ui};

pub struct InstallationTab {}

impl InstallationTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ui(&self, ui: &mut Ui, drivers: &Vec<String>) -> Option<DashboardResponse> {
        let mut response = None;
        ui.vertical(|ui| {
            if ui.button("Run setup wizard").clicked() {
                response = Some(DashboardResponse::SetupWizard(SetupWizardResponse::Start));
            }
            ui.horizontal(|ui| {
                if ui.button("Add firewall rules").clicked() {
                    response = Some(DashboardResponse::Firewall(FirewallRulesResponse::Add));
                }
                if ui.button("Remove firewall rules").clicked() {
                    response = Some(DashboardResponse::Firewall(FirewallRulesResponse::Remove));
                }
            });
            Frame::group(ui.style())
                .fill(theme::LIGHTER_BG)
                .show(ui, |ui| {
                    ui.label(RichText::new("Registered drivers").size(18.0));
                    for driver in drivers {
                        ui.horizontal(|ui| {
                            ui.label(driver);
                            if ui.button("Remove").clicked() {
                                response = Some(DashboardResponse::Driver(
                                    DriverResponse::Unregister(driver.to_owned()),
                                ));
                            }
                        });
                    }
                });
            if ui.button("Register ALVR driver").clicked() {
                response = Some(DashboardResponse::Driver(DriverResponse::RegisterAlvr));
            }
        });
        response
    }
}
