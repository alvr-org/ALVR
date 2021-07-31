use egui::Ui;

// todo: use a custom widget
pub fn button_group_clicked(
    ui: &mut Ui,
    options: &[(String, String)],
    selection: &mut String,
) -> bool {
    let mut clicked = false;
    for (name, display_name) in options {
        if ui
            .selectable_value(selection, name.clone(), display_name)
            .clicked()
        {
            *selection = (*name).to_owned();
            clicked = true;
        }
    }

    clicked
}
