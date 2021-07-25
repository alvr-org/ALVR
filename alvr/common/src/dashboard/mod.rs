mod basic_components;
mod components;

use basic_components::{modal, ModalResponse};
use components::{connections_tab, statistics_tab};
use egui::{Align, CentralPanel, ComboBox, CtxRef, Layout, SidePanel};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    Connections,
    Statistics,
    Settings,
    Installation,
    Logs,
    About,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Connections
    }
}

#[derive(Default)]
struct LanguageModalState {
    visible: bool,
    selection: String,
}

#[derive(Default)]
pub struct Dashboard {
    tab: Tab,
    language_modal_state: LanguageModalState,
    language: String,
}

impl Dashboard {
    pub fn draw(&mut self, ctx: &CtxRef) {
        SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("ALVR");
            egui::warn_if_debug_build(ui);

            for tab in [
                Tab::Connections,
                Tab::Statistics,
                Tab::Settings,
                Tab::Installation,
                Tab::Logs,
                Tab::About,
            ] {
                ui.selectable_value(&mut self.tab, tab, format!("{:?}", tab));
            }

            // ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            //     if ui.selectable_label(false, "Language").clicked() {
            //         self.language_modal_state = LanguageModalState {
            //             visible: true,
            //             selection: self.language.clone(),
            //         };
            //     }
            // });

            if self.language_modal_state.visible {
                language_modal(ctx, &mut self.language_modal_state, &mut self.language);
            }
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("{:?}", self.tab));
            match self.tab {
                Tab::Connections => connections_tab(ui),
                Tab::Statistics => statistics_tab(ui),
                _ => (),
            }
        });
    }
}

fn language_display(code: &str) -> &str {
    match code {
        "en" => "English",
        "it" => "Italiano",
        _ => "",
    }
}

fn language_modal(ctx: &CtxRef, state: &mut LanguageModalState, language: &mut String) {
    let response = modal(
        ctx,
        "Select a language",
        |ui| {
            ComboBox::from_id_source("language_select")
                .selected_text(language_display(&state.selection))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.selection, "en".into(), language_display("en"));
                    ui.selectable_value(&mut state.selection, "it".into(), language_display("it"));
                });
        },
        None,
    );

    match response {
        ModalResponse::ClickedOk => {
            *language = state.selection.clone();
            state.visible = false;
        }
        ModalResponse::ClickedCancel => {
            state.visible = false;
        }
        ModalResponse::Nothing => (),
    }
}
