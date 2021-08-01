use egui::{Button, Ui};
use serde::Serialize;

pub fn reset_clicked<T: PartialEq + Serialize>(
    ui: &mut Ui,
    value: &T,
    default: &T,
    display_default: &str,
) -> bool {
    ui.add(Button::new("⟲").enabled(*value != *default))
        .on_hover_text(format!("Reset to {}", display_default))
        .clicked()
}
