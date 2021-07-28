use egui::Ui;

use crate::dashboard::{Dashboard, DashboardResponse};

pub fn about_tab(ui: &mut Ui) -> Option<DashboardResponse> {
    ui.label("todo");

    None
}
