use egui::{Align, CtxRef, Layout, Ui, Window};

pub enum ModalResponse {
    ClickedOk,
    ClickedCancel,
    Nothing,
}
pub fn modal(
    ctx: &CtxRef,
    title: &str,
    content: impl FnOnce(&mut Ui),
    do_not_show_again: Option<&mut bool>,
) -> ModalResponse {
    let mut response = ModalResponse::Nothing;
    Window::new(title)
        .collapsible(false)
        .fixed_size((300_f32, 150_f32)) // because of a bug with nested layouts, the size must be fixed
        .anchor(egui::Align2::CENTER_CENTER, (0_f32, 0_f32))
        .show(ctx, |ui| {
            // todo: fix layout once the bug with nested layouts is fixed
            ui.with_layout(Layout::left_to_right(), |ui| {
                ui.add_space(10_f32);
                ui.with_layout(Layout::top_down_justified(Align::Center), content);
                ui.add_space(10_f32)
            });
            ui.separator();
            ui.with_layout(Layout::left_to_right(), |ui| {
                if let Some(do_not_show_again) = do_not_show_again {
                    ui.checkbox(do_not_show_again, "Do not show again");
                }

                ui.with_layout(Layout::right_to_left(), |ui| {
                    if ui.button("Cancel").clicked() {
                        response = ModalResponse::ClickedCancel;
                    }
                    if ui.button("OK").clicked() {
                        response = ModalResponse::ClickedOk;
                    }
                });
            });
        });

    response
}
