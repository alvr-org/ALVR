mod basic_components;
mod components;

use std::{collections::VecDeque, net::IpAddr};

use basic_components::{modal, ModalResponse};
use egui::{
    Align, CentralPanel, ComboBox, CtxRef, Layout, Response, ScrollArea, SidePanel, Visuals,
};

use crate::{
    data::{SessionDesc, Theme},
    logging::Event,
};

use self::components::SettingsTab;

pub enum ClientListAction {
    AddIfMissing { display_name: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub struct ConnectionsResponse {
    pub hostname: String,
    pub action: ClientListAction,
}

pub enum FirewallRulesResponse {
    Add,
    Remove,
}

pub enum DriverResponse {
    RegisterAlvr,
    Unregister(String),
}

pub enum DashboardResponse {
    Connections(ConnectionsResponse),
    SessionUpdated(Box<SessionDesc>),
    PresetInvocation(String),
    Driver(DriverResponse),
    FirewallRules(FirewallRulesResponse),
    RestartSteamVR,
    UpdateServer { url: String },
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    Connections,
    Statistics,
    Settings,
    Installation,
    Logs,
    About,
}

fn tab_icon(tab: Tab) -> &'static str {
    match tab {
        Tab::Connections => "🔌",
        Tab::Statistics => "📈",
        Tab::Settings => "⚙",
        Tab::Installation => "💾",
        Tab::Logs => "📝",
        Tab::About => "ℹ",
    }
}

struct LanguageModalState {
    visible: bool,
    selection: String,
}

pub struct Dashboard {
    tab: Tab,
    language_modal_state: LanguageModalState,
    event_buffer: VecDeque<Event>,
    settings_tab: SettingsTab,
    last_language: String,
    last_theme: Theme,
}

impl Dashboard {
    pub fn new(session: &SessionDesc) -> Self {
        let language = session.locale.clone();
        let theme = session.to_settings().extra.theme;

        Self {
            tab: Tab::Connections,
            language_modal_state: LanguageModalState {
                visible: false,
                selection: language.clone(),
            },
            event_buffer: VecDeque::new(),
            settings_tab: SettingsTab::new(&session.session_settings),
            last_language: language,
            last_theme: theme,
        }
    }

    pub fn setup(&mut self, ctx: &CtxRef) {
        let mut style = (*ctx.style()).clone();
        style.spacing.slider_width = 200_f32; // slider width can only be set globally
        ctx.set_style(style);

        if self.last_theme == Theme::Classic {
            ctx.set_visuals(Visuals::light());
        } else {
            ctx.set_visuals(Visuals::dark());
        }
    }

    pub fn update(
        &mut self,
        ctx: &CtxRef,
        session: &SessionDesc,
        new_events: &[Event],
    ) -> Option<DashboardResponse> {
        let response = SidePanel::left("side_panel")
            .resizable(false)
            .show(ctx, |ui| {
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
                    ui.selectable_value(&mut self.tab, tab, format!("{} {:?}", tab_icon(tab), tab));
                }

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    if ui.selectable_label(false, "🌐 Language").clicked() {
                        self.language_modal_state = LanguageModalState {
                            visible: true,
                            selection: session.locale.clone(),
                        };
                    }
                });

                language_modal(ctx, &mut self.language_modal_state, &session)
            })
            .inner;

        let response = CentralPanel::default()
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                    ui.heading(format!("{:?}", self.tab));
                    ScrollArea::auto_sized().show(ui, |ui| match self.tab {
                        Tab::Connections => components::connections_tab(ui),
                        Tab::Statistics => components::statistics_tab(ui),
                        Tab::Settings => self.settings_tab.ui(ui, session),
                        Tab::Installation => components::installation_tab(ui),
                        Tab::Logs => components::logs_tab(ui),
                        Tab::About => components::about_tab(ui),
                    })
                })
                .inner
            })
            .inner
            .or(response);

        if let Some(DashboardResponse::SessionUpdated(session)) = &response {
            let settings = session.to_settings();

            let theme = settings.extra.theme;
            if theme != self.last_theme {
                self.last_theme = theme;

                if self.last_theme == Theme::Classic {
                    ctx.set_visuals(Visuals::light());
                } else {
                    ctx.set_visuals(Visuals::dark());
                }
            }

            if session.locale != self.last_language {
                *self = Dashboard::new(session)
            }
        }

        response
    }
}

fn language_display(code: &str) -> &str {
    match code {
        "en" => "English",
        "it" => "Italiano",
        _ => "",
    }
}

fn language_modal(
    ctx: &CtxRef,
    state: &mut LanguageModalState,
    session: &SessionDesc,
) -> Option<DashboardResponse> {
    let LanguageModalState { visible, selection } = state;

    let maybe_response = modal(
        ctx,
        "Select a language",
        |ui, available_width| {
            const COMBO_WIDTH: f32 = 100_f32;

            // comboboxes do not respoect parent layout, use manual spaces

            ui.with_layout(Layout::left_to_right().with_cross_align(Align::TOP), |ui| {
                ui.add_space((available_width - COMBO_WIDTH) / 2_f32);
                ComboBox::from_id_source("language_select")
                    .width(COMBO_WIDTH)
                    .selected_text(language_display(selection))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(selection, "en".into(), language_display("en"));
                        ui.selectable_value(selection, "it".into(), language_display("it"));
                        ui.selectable_value(selection, "it".into(), language_display("it"));
                        ui.selectable_value(selection, "it".into(), language_display("it"));
                        ui.selectable_value(selection, "it".into(), language_display("it"));
                        ui.selectable_value(selection, "it".into(), language_display("it"));
                        ui.selectable_value(selection, "it".into(), language_display("it"));
                        ui.selectable_value(selection, "it".into(), language_display("it"));
                    });
            });
        },
        None,
        visible,
    );

    if matches!(maybe_response, Some(ModalResponse::Ok)) {
        let mut session = session.clone();
        session.locale = selection.clone();

        Some(DashboardResponse::SessionUpdated(Box::new(session)))
    } else {
        None
    }
}
