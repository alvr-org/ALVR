use alvr_gui_common::theme::log_colors;
use eframe::{
    egui::{Frame, Label, RichText, Ui},
    epaint::Color32,
};

pub fn notice(ui: &mut Ui, text: &str) {
    Frame::group(ui.style())
        .fill(log_colors::WARNING_LIGHT)
        .show(ui, |ui| {
            ui.add(Label::new(RichText::new(text).size(11.0).color(Color32::BLACK)).wrap());
        });
}
