use alvr_gui_common::theme::log_colors;
use eframe::{
    egui::{Frame, RichText, Ui},
    epaint::Color32,
};

// Returns true if buttons was clicked
pub fn notice(ui: &mut Ui, text: &str) {
    Frame::group(ui.style())
        .inner_margin(0.0)
        .fill(log_colors::WARNING_LIGHT)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.colored_label(Color32::BLACK, RichText::new(text).size(11.0));
                ui.add_space(-5.0);
            });
        });
}
