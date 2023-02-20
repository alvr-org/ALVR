use eframe::egui::{Align, Align2, Layout, Ui, Window};

pub enum ModalResponse {
    Ok,
    Cancel,
}

pub fn modal(
    ui: &mut Ui,
    title: &str,
    content: impl FnOnce(&mut Ui, f32), // arg 2: available width
    do_not_show_again: Option<&mut bool>,
    visible: &mut bool,
) -> Option<ModalResponse> {
    let mut response = None;
    if *visible {
        Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_width(200_f32)
            .anchor(Align2::CENTER_CENTER, (0_f32, 0_f32))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.add_space(10_f32);
                    content(ui, ui.available_width() - 8_f32); // extra offset to avoid window resizing. todo: find origin
                    ui.add_space(10_f32);

                    if let Some(do_not_show_again) = do_not_show_again {
                        ui.checkbox(do_not_show_again, "Do not ask again");
                    }

                    ui.columns(2, |cols| {
                        for (i, col) in cols.iter_mut().enumerate() {
                            col.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                if i == 0 {
                                    if ui.button("Cancel").clicked() {
                                        *visible = false;
                                        response = Some(ModalResponse::Cancel);
                                    }
                                } else if ui.button("OK").clicked() {
                                    *visible = false;
                                    response = Some(ModalResponse::Ok);
                                }
                            });
                        }
                    });
                });
            });
    }

    response
}
