use crate::{dashboard::DashboardResponse, translation::TranslationBundle};
use egui::Ui;

pub struct LogsTab {}

impl LogsTab {
    pub fn new(trans: &TranslationBundle) -> Self {
        Self {}
    }

    pub fn ui(&self, ui: &mut Ui) -> Option<DashboardResponse> {
        None
    }
}
