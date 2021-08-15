use egui::{Button, Ui};
use serde::Serialize;

use crate::translation::SharedTranslation;

pub fn reset_clicked<T: PartialEq + Serialize>(
    ui: &mut Ui,
    value: &T,
    default: &T,
    default_trans: &str,
    t: &SharedTranslation,
) -> bool {
    ui.add(Button::new("⟲").enabled(*value != *default))
        .on_hover_text(format!("{} {}", t.reset_to, default_trans))
        .clicked()
}
