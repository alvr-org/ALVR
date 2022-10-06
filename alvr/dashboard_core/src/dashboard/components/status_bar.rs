use crate::{dashboard::DashboardResponse, translation::TranslationBundle};
use egui::{Color32, Frame, Ui};

pub struct StatusBar {}

impl StatusBar {
    pub fn new(trans_bundle: &TranslationBundle) -> Self {
        Self {}
    }

    pub fn ui(&self, ui: &mut Ui) {
        Frame::default()
            .fill(Color32::GREEN)
            .show(ui, |ui| ui.label("Yoooo"));
    }
}
