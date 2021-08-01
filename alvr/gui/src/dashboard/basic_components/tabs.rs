use egui::{Align, InnerResponse, Layout, Response, Ui};

pub fn tabs<R>(
    ui: &mut Ui,
    tabs: Vec<(String, String)>,
    selected_tab: &mut String,
    content: impl FnOnce(&mut Ui) -> R,
    right_slot: impl FnOnce(&mut Ui),
) -> R {
    ui.with_layout(
        Layout::top_down(Align::LEFT).with_cross_justify(true),
        |ui| {
            ui.with_layout(Layout::left_to_right().with_cross_align(Align::TOP), |ui| {
                for (name, display_name) in tabs {
                    ui.selectable_value(selected_tab, name, display_name);
                }

                ui.with_layout(
                    Layout::right_to_left().with_cross_align(Align::TOP),
                    right_slot,
                );
            });

            ui.separator();

            content(ui)
        },
    )
    .inner
}
