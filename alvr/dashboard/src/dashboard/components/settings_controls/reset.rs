use eframe::{
    egui::{self, Button, Layout, Response, Ui, vec2},
    emath::Align,
};

pub fn reset_button(ui: &mut Ui, enabled: bool, default_str: &str) -> Response {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add_space(5.0);

        // let button_size = ui.spacing().interact_size.y;

        // ui.add_enabled(enabled, Button::new("⟲").min_size(vec2(button_size, button_size)))
        //     .on_hover_text(format!("Reset to {default_str}."))

        ui.scope(|ui| {
            ui.style_mut().spacing.interact_size = vec2(35.0, 35.0);
            let height = ui.style_mut().spacing.interact_size.y;
            ui.add_enabled(enabled, Button::new("⟲").min_size(vec2(height, height)))
                .on_hover_text(format!("Reset to {default_str}"))
        }).inner
        // let button_height = ui.spacing().interact_size.y;
        // ui.style_mut().spacing.interact_size = vec2(10.0, 50.0);
        
    })
    .inner
}
