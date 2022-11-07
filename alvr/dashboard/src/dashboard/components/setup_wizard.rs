use egui::{Layout, Ui};

use crate::dashboard::{DashboardResponse, FirewallRulesResponse, SetupWizardResponse};

enum Page {
    Welcome,
    SoftwareRequirements,
    Firewall,
    PerformancePreset,
    Recommendations,
    Finished,
}

pub struct SetupWizard {
    page: Page,
}

impl SetupWizard {
    pub fn new() -> Self {
        Self {
            page: Page::Welcome,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<DashboardResponse> {
        use Page::*;
        let mut response = None;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Welcome to ALVR");
                ui.label("This setup will help with a basic setup of ALVR");
            });
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("❌").clicked() {
                    response = Some(DashboardResponse::SetupWizard(SetupWizardResponse::Close));
                }
            })
        });
        ui.separator();
        match &self.page {
            Welcome => {
                ui.label(
                    r#"Hardware requirements
ALVR requires a dedicated and recent graphics card.

Make sure you have at least one output audio device."#,
                );
            }
            SoftwareRequirements => {
                ui.label(r#"Software requirements
To stream the Quest microphone on Windows you need to install VB-Audio Virtual Cable.
On Linux some feaures are not working and should be disabled (foveated encoding and color correction) and some need a proper environment setup to have them working (game audio and microphone streaming)."#);
            }
            Firewall => {
                ui.label(
                    r#"Firewall
To communicate with the headset, some firewall rules need to be set.
This requires administrator rights!"#,
                );
                if ui.button("Add firewall rules").clicked() {
                    response = Some(DashboardResponse::Firewall(FirewallRulesResponse::Add));
                }
            }
            PerformancePreset => {
                ui.label(
                    r#"Performance preset
Please choose preset that fits your setup. This will adjust some settings for you.
"#,
                );
                ui.horizontal(|ui| {
                    // TODO correct preset strings
                    if ui.button("Compatibility").clicked() {
                        response = Some(DashboardResponse::PresetInvocation(
                            "compatibility".to_string(),
                        ));
                    }
                    if ui.button("Visual quality").clicked() {
                        response = Some(DashboardResponse::PresetInvocation(
                            "visual_quality".to_string(),
                        ));
                    }
                });
            }
            Recommendations => {
                ui.label(r#"Recommendations
ALVR supports multiple types of PC hardware and headsets but not all work correctly with default settings. For example some AMD video cards work only with the HEVC codec and GearVR does not support foveated encoding. Please try tweaking different settings if your ALVR experience is broken or not optimal."#);
            }
            Finished => {
                ui.label(
                    r#"Finished
You can always restart this setup wizard from the "Installation" tab on the left"#,
                );
            }
        };
        ui.with_layout(Layout::bottom_up(egui::Align::Max), |ui| {
            ui.add_space(20.0);
            ui.horizontal(|ui| match self.page {
                Welcome => {
                    if ui.button("Next").clicked() {
                        self.page = SoftwareRequirements;
                    }
                }
                SoftwareRequirements => {
                    if ui.button("Next").clicked() {
                        self.page = Firewall;
                    }
                    if ui.button("Back").clicked() {
                        self.page = Welcome;
                    }
                }
                Firewall => {
                    if ui.button("Next").clicked() {
                        self.page = PerformancePreset;
                    }
                    if ui.button("Back").clicked() {
                        self.page = SoftwareRequirements;
                    }
                }
                PerformancePreset => {
                    if ui.button("Next").clicked() {
                        self.page = Recommendations;
                    }
                    if ui.button("Back").clicked() {
                        self.page = Firewall;
                    }
                }
                Recommendations => {
                    if ui.button("Next").clicked() {
                        self.page = Finished;
                    }

                    if ui.button("Back").clicked() {
                        self.page = PerformancePreset;
                    }
                }
                Finished => {
                    if ui.button("Close").clicked() {
                        response = Some(DashboardResponse::SetupWizard(SetupWizardResponse::Close));
                    }
                    if ui.button("Back").clicked() {
                        self.page = Recommendations;
                    }
                }
            });
            ui.separator();
        });
        response
    }
}
