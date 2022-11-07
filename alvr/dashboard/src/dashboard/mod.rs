mod basic_components;
mod components;

use self::components::{
    AboutTab, ConnectionsTab, InstallationTab, LogsTab, SettingsTab, SetupWizard,
};
use crate::{
    dashboard::components::StatisticsTab,
    theme,
    translation::{self, TranslationBundle},
};
use alvr_events::{Event, EventSeverity, EventType, LogEvent};
use alvr_session::{ClientConnectionDesc, LogLevel, SessionDesc};
use egui::{
    style::Margin, Align, CentralPanel, Context, Frame, Label, Layout, RichText, ScrollArea,
    SidePanel, Stroke,
};
use std::{collections::BTreeMap, sync::Arc};

const NOTIFICATION_BAR_HEIGHT: f32 = 30.0;

#[derive(Debug)]
pub enum FirewallRulesResponse {
    Add,
    Remove,
}

#[derive(Debug)]
pub enum DriverResponse {
    RegisterAlvr,
    Unregister(String),
}

#[derive(Debug)]
pub enum ConnectionsResponse {
    AddOrUpdate {
        name: String,
        client_desc: ClientConnectionDesc,
    },
    RemoveEntry(String),
}

#[derive(Debug)]
pub enum SetupWizardResponse {
    Start,
    Close,
}

#[derive(Debug)]
pub enum DashboardResponse {
    Connections(ConnectionsResponse),
    SessionUpdated(Box<SessionDesc>),
    PresetInvocation(String),
    Driver(DriverResponse),
    Firewall(FirewallRulesResponse),
    RestartSteamVR,
    SetupWizard(SetupWizardResponse),
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

pub struct Dashboard {
    selected_tab: Tab,
    tab_labels: BTreeMap<Tab, &'static str>,
    connections_tab: ConnectionsTab,
    statistics_tab: StatisticsTab,
    settings_tab: SettingsTab,
    installation_tab: InstallationTab,
    logs_tab: LogsTab,
    about_tab: AboutTab,
    notification: Option<LogEvent>,
    setup_wizard: Option<SetupWizard>,
    session: SessionDesc,
    drivers: Vec<String>,
}

impl Dashboard {
    pub fn new(
        session: SessionDesc,
        drivers: Vec<String>,
        translation_bundle: Arc<TranslationBundle>,
    ) -> Self {
        let t = translation::get_shared_translation(&translation_bundle);

        Self {
            selected_tab: Tab::Connections,
            tab_labels: [
                (Tab::Connections, "🔌 Connections"),
                (Tab::Statistics, "📈 Statistics"),
                (Tab::Settings, "⚙ Settings"),
                (Tab::Installation, "💾 Installation"),
                (Tab::Logs, "📝 Logs"),
                (Tab::About, "ℹ About"),
            ]
            .into_iter()
            .map(|val| val.clone())
            .collect(),
            connections_tab: ConnectionsTab::new(),
            statistics_tab: StatisticsTab::new(),
            settings_tab: SettingsTab::new(
                &session.session_settings,
                Arc::clone(&t),
                &translation_bundle,
            ),
            installation_tab: InstallationTab::new(),
            logs_tab: LogsTab::new(),
            about_tab: AboutTab::new(),
            notification: None,
            setup_wizard: if session.setup_wizard {
                Some(SetupWizard::new())
            } else {
                None
            },
            session,
            drivers,
        }
    }

    pub fn setup(&mut self, ctx: &Context) {
        theme::set_theme(ctx);
    }

    pub fn update(&mut self, ctx: &Context, new_events: &[Event]) -> Option<DashboardResponse> {
        for event in new_events {
            match &event.event_type {
                EventType::GraphStatistics(graph_statistics) => self
                    .statistics_tab
                    .update_graph_statistics(graph_statistics.clone()),
                EventType::Statistics(statistics) => {
                    self.statistics_tab.update_statistics(statistics.clone())
                }
                EventType::Log(log) => {
                    self.logs_tab.update_logs(log.clone());
                    // Create a notification based on the notification level in the settings
                    match self.session.to_settings().extra.notification_level {
                        LogLevel::Debug => self.notification = Some(log.to_owned()),
                        LogLevel::Info => match log.severity {
                            EventSeverity::Info | EventSeverity::Warning | EventSeverity::Error => {
                                self.notification = Some(log.to_owned())
                            }
                            _ => (),
                        },
                        LogLevel::Warning => match log.severity {
                            EventSeverity::Warning | EventSeverity::Error => {
                                self.notification = Some(log.to_owned())
                            }
                            _ => (),
                        },
                        LogLevel::Error => match log.severity {
                            EventSeverity::Error => self.notification = Some(log.to_owned()),
                            _ => (),
                        },
                    }
                }
                _ => (),
            }
        }

        let mut response = match &mut self.setup_wizard {
            Some(setup_wizard) => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| setup_wizard.ui(ui))
                    .inner
            }
            None => {
                if match &self.notification {
                    Some(log) => {
                        let (fg, bg) = match log.severity {
                            EventSeverity::Debug => (theme::FG, theme::DEBUG),
                            EventSeverity::Info => (theme::BG, theme::INFO),
                            EventSeverity::Warning => (theme::BG, theme::WARNING),
                            EventSeverity::Error => (theme::FG, theme::ERROR),
                        };
                        egui::TopBottomPanel::bottom("bottom_panel")
                            .default_height(NOTIFICATION_BAR_HEIGHT)
                            .min_height(NOTIFICATION_BAR_HEIGHT)
                            .frame(
                                Frame::default()
                                    .inner_margin(Margin::same(5.0))
                                    .fill(bg)
                                    .stroke(Stroke::new(1.0, theme::SEPARATOR_BG)),
                            )
                            .show(ctx, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        Label::new(RichText::new(&log.content).color(fg))
                                            .wrap(true),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.button("❌").clicked() {
                                            true
                                        } else {
                                            false
                                        }
                                    })
                                    .inner
                                })
                                .inner
                            })
                            .inner
                    }
                    None => {
                        egui::TopBottomPanel::bottom("bottom_panel")
                            .default_height(NOTIFICATION_BAR_HEIGHT)
                            .min_height(NOTIFICATION_BAR_HEIGHT)
                            .frame(
                                Frame::default()
                                    .inner_margin(Margin::same(5.0))
                                    .fill(theme::BG)
                                    .stroke(Stroke::new(1.0, theme::SEPARATOR_BG)),
                            )
                            .show(ctx, |ui| ui.label("No new notifications"));
                        false
                    }
                } {
                    self.notification = None;
                }

                let response = SidePanel::left("side_panel")
                    .resizable(false)
                    .max_width(150.0)
                    .show(ctx, |ui| {
                        ui.heading("ALVR");
                        egui::warn_if_debug_build(ui);

                        ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                            for (tab, label) in &self.tab_labels {
                                ui.selectable_value(&mut self.selected_tab, *tab, *label);
                            }
                        });

                        ui.with_layout(
                            Layout::bottom_up(Align::Min).with_cross_justify(true),
                            |ui| {
                                ui.add_space(5.0);
                                if ui.button("Restart SteamVR").clicked() {
                                    Some(DashboardResponse::RestartSteamVR)
                                } else {
                                    None
                                }
                            },
                        )
                        .inner
                    })
                    .inner;

                let response = CentralPanel::default()
                    .show(ctx, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                            ui.heading(*self.tab_labels.get(&self.selected_tab).unwrap());
                            ScrollArea::new([false, true]).show(ui, |ui| match self.selected_tab {
                                Tab::Connections => self.connections_tab.ui(ui, &self.session),
                                Tab::Statistics => self.statistics_tab.ui(ui),
                                Tab::Settings => self.settings_tab.ui(ui, &self.session),
                                Tab::Installation => self.installation_tab.ui(ui, &self.drivers),
                                Tab::Logs => self.logs_tab.ui(ui),
                                Tab::About => self.about_tab.ui(ui, &self.session),
                            })
                        })
                        .inner
                    })
                    .inner
                    .inner
                    .or(response);
                response
            }
        };

        if let Some(DashboardResponse::SetupWizard(_response)) = &response {
            match _response {
                SetupWizardResponse::Close => {
                    self.setup_wizard = None;
                    let mut session = self.session.to_owned();
                    session.setup_wizard = false;
                    response = Some(DashboardResponse::SessionUpdated(Box::new(session)));
                }
                SetupWizardResponse::Start => self.setup_wizard = Some(SetupWizard::new()),
            }
        }
        response
    }
}
