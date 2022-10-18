use crate::{dashboard::DashboardResponse, translation::TranslationBundle};
use alvr_session::{ClientConnectionDesc, ConnectionDesc, SessionDesc};
use egui::{Align, Color32, Frame, Layout, Resize, RichText, Style, Ui};
use std::cmp;

const MIN_CLIENT_CARD_WIDTH: f32 = 200_f32;

fn client_card(ui: &mut Ui, trusted: bool) -> Option<DashboardResponse> {
    ui.group(|ui| {
        Resize::default()
            .fixed_size((MIN_CLIENT_CARD_WIDTH, MIN_CLIENT_CARD_WIDTH))
            .resizable(false)
            .show(ui, |ui| {
                let trusted_text = if trusted { "Trusted" } else { "New" };
                ui.label(trusted_text);

                ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
                    let action_text = if trusted { "Configure" } else { "Trust" };
                    ui.button(action_text);
                });
            });
    });

    None
}

pub struct ConnectionsTab {}

impl ConnectionsTab {
    pub fn new(trans: &TranslationBundle) -> Self {
        Self {}
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
            .fill(Color32::DARK_GRAY.linear_multiply(0.2))
            .show(ui, |ui| {
                ui.label(RichText::new("New clients").size(20.0));
                for (name, client_desc) in trusted {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        ui.button("Trust");
                    });
                }
            });
        ui.add_space(10.0);
        Frame::group(ui.style())
            .fill(Color32::DARK_GRAY.linear_multiply(0.2))
            .show(ui, |ui| {
                ui.label(RichText::new("Trusted clients").size(20.0));
            });
        ui.add_space(10.0);
        if ui.button("Add client manually").clicked() {}
        response
    }
}
