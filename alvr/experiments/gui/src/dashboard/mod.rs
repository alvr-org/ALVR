mod basic_components;
mod components;

use self::components::{AboutTab, ConnectionsTab, InstallationTab, LogsTab, SettingsTab};
use crate::{
    dashboard::components::StatisticsTab,
    theme,
    translation::{self, SharedTranslation, TranslationBundle},
};
use alvr_common::logging::Event;
use alvr_session::{SessionDesc, Theme};
use basic_components::ModalResponse;
use egui::{Align, CentralPanel, ComboBox, CtxRef, Layout, ScrollArea, SidePanel, Ui};
use std::{
    array::IntoIter,
    collections::{BTreeMap, VecDeque},
    net::IpAddr,
    sync::Arc,
};

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Tab {
    Connections,
    Statistics,
    Settings,
    Installation,
    Logs,
    About,
}

struct LanguageModalState {
    visible: bool,
    selection: Option<String>,
}

pub struct Dashboard {
    selected_tab: Tab,
    language_modal_state: LanguageModalState,
    event_buffer: VecDeque<Event>,
    tab_labels: BTreeMap<Tab, String>,
    language_label: String,
    connections_tab: ConnectionsTab,
    statistics_tab: StatisticsTab,
    settings_tab: SettingsTab,
    installation_tab: InstallationTab,
    logs_tab: LogsTab,
    about_tab: AboutTab,
    last_theme: Theme,
    t: Arc<SharedTranslation>,
    language_prompt_trans: String,
    system_language_trans: String,
    trans_bundle: Arc<TranslationBundle>,
}

impl Dashboard {
    pub fn new(session: &SessionDesc, translation_bundle: Arc<TranslationBundle>) -> Self {
        let language = if session.locale == "system" {
            None
        } else {
            Some(session.locale.clone())
        };

        let theme = session.to_settings().extra.theme;

        let t = translation::get_shared_translation(&translation_bundle);

        Self {
            selected_tab: Tab::Connections,
            language_modal_state: LanguageModalState {
                visible: false,
                selection: language,
            },
            event_buffer: VecDeque::new(),
            tab_labels: IntoIter::new([
                (
                    Tab::Connections,
                    format!("🔌 {}", translation_bundle.get("connections")),
                ),
                (
                    Tab::Statistics,
                    format!("📈 {}", translation_bundle.get("statistics")),
                ),
                (
                    Tab::Settings,
                    format!("⚙ {}", translation_bundle.get("settings")),
                ),
                (
                    Tab::Installation,
                    format!("💾 {}", translation_bundle.get("installation")),
                ),
                (Tab::Logs, format!("📝 {}", translation_bundle.get("logs"))),
                (Tab::About, format!("ℹ {}", translation_bundle.get("about"))),
            ])
            .collect(),
            language_label: format!("🌐 {}", translation_bundle.get("language")),
            connections_tab: ConnectionsTab::new(&translation_bundle),
            statistics_tab: StatisticsTab::new(&translation_bundle),
            settings_tab: SettingsTab::new(
                &session.session_settings,
                Arc::clone(&t),
                &translation_bundle,
            ),
            installation_tab: InstallationTab::new(&translation_bundle),
            logs_tab: LogsTab::new(&translation_bundle),
            about_tab: AboutTab::new(&translation_bundle),
            last_theme: theme,
            t,
            language_prompt_trans: translation_bundle.attribute("language", "prompt"),
            system_language_trans: translation_bundle.attribute("language", "system"),
            trans_bundle: translation_bundle,
        }
    }

    pub fn setup(&mut self, ctx: &CtxRef) {
        theme::set_theme(ctx, self.last_theme);
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

                for (tab, label) in &self.tab_labels {
                    ui.selectable_value(&mut self.selected_tab, *tab, label);
                }

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    if ui.selectable_label(false, &self.language_label).clicked() {
                        self.language_modal_state = LanguageModalState {
                            visible: true,
                            selection: if session.locale == "system" {
                                None
                            } else {
                                Some(session.locale.clone())
                            },
                        };
                    }
                });

                language_modal(
                    ui,
                    &mut self.language_modal_state,
                    self.trans_bundle.languages(),
                    session,
                    &self.t,
                    &self.language_prompt_trans,
                    &self.system_language_trans,
                )
            })
            .inner;

        let response = CentralPanel::default()
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                    ui.heading(self.tab_labels.get(&self.selected_tab).unwrap());
                    ScrollArea::auto_sized().show(ui, |ui| match self.selected_tab {
                        Tab::Connections => self.connections_tab.ui(ui, session),
                        Tab::Statistics => self.statistics_tab.ui(ui),
                        Tab::Settings => self.settings_tab.ui(ui, session),
                        Tab::Installation => self.installation_tab.ui(ui),
                        Tab::Logs => self.logs_tab.ui(ui),
                        Tab::About => self.about_tab.ui(ui),
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

                theme::set_theme(ctx, theme);
            }
        }

        response
    }
}

fn language_modal(
    ctx: &mut Ui,
    state: &mut LanguageModalState,
    languages: &BTreeMap<String, String>,
    session: &SessionDesc,
    t: &SharedTranslation,
    prompt_trans: &str,
    system_trans: &str,
) -> Option<DashboardResponse> {
    let LanguageModalState { visible, selection } = state;

    let maybe_response = basic_components::modal(
        ctx,
        prompt_trans,
        |ui, available_width| {
            const COMBO_WIDTH: f32 = 100_f32;

            // comboboxes do not respoect parent layout, use manual spaces

            ui.with_layout(Layout::left_to_right().with_cross_align(Align::TOP), |ui| {
                ui.add_space((available_width - COMBO_WIDTH) / 2_f32);

                let selection_text = match selection {
                    Some(language) => languages.get(language).unwrap(),
                    None => system_trans,
                };

                ComboBox::from_id_source("language_select")
                    .width(COMBO_WIDTH)
                    .selected_text(selection_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(selection, None, system_trans);

                        for (name, trans) in languages {
                            ui.selectable_value(selection, Some(name.clone()), trans);
                        }
                    });
            });
        },
        None,
        visible,
        t,
    );

    if matches!(maybe_response, Some(ModalResponse::Ok)) {
        let mut session = session.clone();
        session.locale = selection.clone().unwrap_or_else(|| "system".into());

        Some(DashboardResponse::SessionUpdated(Box::new(session)))
    } else {
        None
    }
}
