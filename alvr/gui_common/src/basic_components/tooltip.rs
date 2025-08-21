use egui::{self, Id, Popup, PopupAnchor, Ui};

pub fn tooltip(ui: &mut Ui, id: &str, text: &str) {
    Popup::new(
        Id::new(id),
        ui.ctx().clone(),
        PopupAnchor::PointerFixed,
        ui.layer_id(),
    )
    .show(|ui| {
        ui.label(text);
    });
}
