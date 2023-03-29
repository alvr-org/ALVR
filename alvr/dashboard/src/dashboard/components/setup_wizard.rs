use crate::firewall;
use alvr_common::prelude::*;
use eframe::{
    egui::{Button, Label, Layout, RichText, Ui},
    emath::Align,
};

pub enum SetupWizardRequest {
    // Dashboard(DashboardRequest),
    Close { finished: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Welcome = 0,
    HardwareRequirements = 1,
    SoftwareRequirements = 2,
    Firewall = 3,
    // PerformancePreset,
    Recommendations = 4,
    Finished = 5,
}

fn index_to_page(index: usize) -> Page {
    match index {
        0 => Page::Welcome,
        1 => Page::HardwareRequirements,
        2 => Page::SoftwareRequirements,
        3 => Page::Firewall,
        4 => Page::Recommendations,
        5 => Page::Finished,
        _ => unreachable!(),
    }
}

fn page_content(
    ui: &mut Ui,
    subtitle: &str,
    paragraph: &str,
    interactible_content: impl Fn(&mut Ui),
) {
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        ui.add_space(60.0);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.add_space(60.0);
            ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                ui.add_space(15.0);
                ui.heading(RichText::new(subtitle).size(20.0));
                ui.add(Label::new(RichText::new(paragraph).size(14.0)).wrap(true));
                ui.add_space(30.0);
                ui.vertical_centered(interactible_content);
            });
        })
    });
}

pub struct SetupWizard {
    page: Page,
    finished: bool,
}

impl SetupWizard {
    pub fn new() -> Self {
        Self {
            page: Page::Welcome,
            finished: false,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<SetupWizardRequest> {
        let mut response = None;

        ui.horizontal(|ui| {
            ui.add_space(60.0);
            ui.vertical(|ui| {
                ui.add_space(30.0);
                ui.heading(RichText::new("Welcome to ALVR").size(30.0));
                ui.add_space(5.0);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(15.0);
                if ui.button("❌").clicked() {
                    response = Some(SetupWizardRequest::Close {
                        finished: self.finished,
                    });
                }
            })
        });
        ui.separator();
        match &self.page {
            Page::Welcome => page_content(
                ui,
                "This setup wizard will help you setup ALVR.",
                "",
                |_| (),
            ),
            Page::HardwareRequirements => page_content(
                ui,
                "Hardware requirements",
                r"ALVR requires a dedicated and recent graphics card.
Make sure you have at least one output audio device.",
                |_| (),
            ),
            Page::SoftwareRequirements => page_content(
                ui,
                "Software requirements",
                r"To stream the Quest microphone on Windows you need to install VB-Audio Virtual Cable.
On Linux some feaures are not working and should be disabled (foveated encoding and color correction) and some need a proper environment setup to have them working (game audio and microphone streaming).",
                |_| (),
            ),
            Page::Firewall => page_content(
                ui,
                "Firewall",
                r"To communicate with the headset, some firewall rules need to be set.
This requires administrator rights!",
                |ui| {
                    if ui.button("Add firewall rules").clicked() {
                        if firewall::firewall_rules(true).is_ok() {
                            info!("Setting firewall rules succeeded!");
                        } else {
                            error!("Setting firewall rules failed!");
                        }
                    }
                },
            ),
            //             Page::PerformancePreset => {
            //                 ui.label(
            //                     r#"Performance preset
            // Please choose preset that fits your setup. This will adjust some settings for you.
            // "#,
            //                 );
            //                 ui.horizontal(|ui| {
            //                     // TODO correct preset strings
            //                     if ui.button("Compatibility").clicked() {
            //                         // response = Some(DashboardRequest::PresetInvocation(
            //                         //     "compatibility".to_string(),
            //                         // ));
            //                     }
            //                     if ui.button("Visual quality").clicked() {
            //                         // response = Some(DashboardRequest::PresetInvocation(
            //                         //     "visual_quality".to_string(),
            //                         // ));
            //                     }
            //                 });
            //             }
            Page::Recommendations => page_content(
                ui,
                "Recommendations",
                r"ALVR supports multiple types of PC hardware and headsets but not all work correctly with default settings. For example some AMD video cards work only with the HEVC codec and GearVR does not support foveated encoding. Please try tweaking different settings if your ALVR experience is broken or not optimal.",
                |_| (),
            ),
            Page::Finished => page_content(
                ui,
                "Finished",
                r#"You can always restart this setup wizard from the "Installation" tab on the left."#,
                |_| (),
            ),
        };

        ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                if ui
                    .add_visible(self.page != Page::Finished, Button::new("Next"))
                    .clicked()
                {
                    self.page = index_to_page(self.page as usize + 1);
                    if self.page == Page::Finished {
                        self.finished = true;
                    }
                }
                if ui
                    .add_visible(self.page != Page::HardwareRequirements, Button::new("Back"))
                    .clicked()
                {
                    self.page = index_to_page(self.page as usize - 1);
                }
            });
            ui.separator();
        });

        response
    }
}
