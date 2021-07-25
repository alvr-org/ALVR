use egui::{Button, Color32, Layout, Ui};

// todo: use a custom widget
pub fn button_group(ui: &mut Ui, options: &[String], selection: &mut String) {
    ui.with_layout(Layout::left_to_right(), |ui| {
        for opt in options {
            let mut button = Button::new(opt);

            if *opt == *selection {
                button = button.fill(Color32::LIGHT_BLUE).text_color(Color32::BLACK);
            }

            if ui.add(button).clicked() {
                *selection = opt.clone();
            }
        }
    });
}
