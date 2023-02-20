use crate::dashboard::DisplayString;
use eframe::egui::Ui;

// todo: use a custom widget
pub fn button_group_clicked(
    ui: &mut Ui,
    options: &[DisplayString],
    selection: &mut String,
) -> bool {
    let mut clicked = false;
    for id in options {
        if ui
            .selectable_value(selection, (**id).clone(), &id.display)
            .clicked()
        {
            clicked = true;
        }
    }

    clicked
}
