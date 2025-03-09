use eframe::{
    egui::{self, Button, Layout, Ui},
    emath::Align,
};

#[derive(PartialEq, Eq)]
pub enum UpDownResult {
    Up,
    Down,
    None,
}

pub fn up_down_buttons(ui: &mut Ui, index: usize, count: usize) -> UpDownResult {
    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        let up_clicked = ui
            .add_visible(index > 0, Button::new("⬆").small())
            .clicked();
        let down_clicked = ui
            .add_visible(index < count - 1, Button::new("⬇").small())
            .clicked();

        if up_clicked {
            UpDownResult::Up
        } else if down_clicked {
            UpDownResult::Down
        } else {
            UpDownResult::None
        }
    })
    .inner
}
