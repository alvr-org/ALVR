use egui::Ui;

use crate::DisplayString;

// todo: use a custom widget
pub fn button_group_clicked(ui: &mut Ui, options: &[DisplayString], selection: &mut String) -> bool {
    let mut clicked = false;
    for id in options {
        if ui
            .selectable_value(selection, (**id).clone(), &id.display)
            .clicked()
        {
            *selection = (**id).to_owned();
            clicked = true;
        }
    }

    clicked
}
