use crate::{dashboard::DashboardResponse, translation::TranslationBundle};
use egui::Ui;

pub struct AboutTab {}

impl AboutTab {
    pub fn new(trans_bundle: &TranslationBundle) -> Self {
        Self {}
    }

    pub fn ui(&self, ui: &mut Ui) -> Option<DashboardResponse> {
        None
    }
}
