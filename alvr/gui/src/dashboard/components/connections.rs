use crate::{dashboard::DashboardResponse, translation::TranslationBundle};
use alvr_common::data::SessionDesc;
use egui::{Align, Layout, Resize, Ui};
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
        let available_width = ui.available_width();

        let clients_count = 4;

        let cols_count = cmp::max((available_width / MIN_CLIENT_CARD_WIDTH) as usize, 1);

        ui.add_space(20_f32);
        ui.columns(cols_count, |cols| {
            for (col, col_ui) in cols.iter_mut().enumerate() {
                col_ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - MIN_CLIENT_CARD_WIDTH) / 2_f32);

                    ui.vertical(|ui| {
                        for row in 0..(clients_count / cols_count + 1) {
                            if row * cols_count + col < clients_count {
                                client_card(ui, false);
                                ui.add_space(20_f32);
                            }
                        }
                    });
                });
            }
        });

        None
    }
}
