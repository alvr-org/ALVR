mod basic_components;
mod components;

use self::components::{
    ConnectionsTab, InstallationTab, InstallationTabRequest, LogsTab, NotificationBar, SettingsTab,
    SetupWizard, SetupWizardRequest,
};
use crate::{dashboard::components::StatisticsTab, steamvr_launcher::LAUNCHER, theme, ServerEvent};
use alvr_common::RelaxedAtomic;
use alvr_events::EventType;
use alvr_session::SessionDesc;
use alvr_sockets::DashboardRequest;
use eframe::{
    egui::{
        self, style::Margin, Align, CentralPanel, Frame, Layout, RichText, ScrollArea, SidePanel,
        Stroke,
    },
    epaint::Color32,
};
use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{atomic::AtomicUsize, mpsc, Arc},
    thread,
};

#[derive(Clone)]
pub struct DisplayString {
    pub id: String,
    pub display: String,
}

impl From<(String, String)> for DisplayString {
    fn from((id, display): (String, String)) -> Self {
        Self { id, display }
    }
}

impl Deref for DisplayString {
    type Target = String;

    fn deref(&self) -> &String {
        &self.id
    }
}

fn get_id() -> usize {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
    just_opened: bool,
    connected_to_server: bool,
    server_restarting: Arc<RelaxedAtomic>,
    selected_tab: Tab,
    tab_labels: BTreeMap<Tab, &'static str>,
    connections_tab: ConnectionsTab,
    statistics_tab: StatisticsTab,
    settings_tab: SettingsTab,
    installation_tab: InstallationTab,
    logs_tab: LogsTab,
    notification_bar: NotificationBar,
    setup_wizard: SetupWizard,
    setup_wizard_open: bool,
    session: SessionDesc,
    dashboard_requests_sender: mpsc::Sender<DashboardRequest>,
    server_events_receiver: mpsc::Receiver<ServerEvent>,
}

impl Dashboard {
    pub fn new(
        creation_context: &eframe::CreationContext<'_>,
        dashboard_requests_sender: mpsc::Sender<DashboardRequest>,
        server_events_receiver: mpsc::Receiver<ServerEvent>,
    ) -> Self {
        dashboard_requests_sender
            .send(DashboardRequest::GetSession)
            .unwrap();
        dashboard_requests_sender
            .send(DashboardRequest::GetAudioDevices)
            .unwrap();

        theme::set_theme(&creation_context.egui_ctx);

        Self {
            just_opened: true,
            connected_to_server: false,
            server_restarting: Arc::new(RelaxedAtomic::new(false)),
            selected_tab: Tab::Connections,
            tab_labels: [
                (Tab::Connections, "🔌  Connections"),
                (Tab::Statistics, "📈  Statistics"),
                (Tab::Settings, "⚙  Settings"),
                (Tab::Installation, "💾  Installation"),
                (Tab::Logs, "📝  Logs"),
                (Tab::About, "ℹ  About"),
            ]
            .into_iter()
            .collect(),
            connections_tab: ConnectionsTab::new(),
            statistics_tab: StatisticsTab::new(),
            settings_tab: SettingsTab::new(),
            installation_tab: InstallationTab::new(),
            logs_tab: LogsTab::new(),
            notification_bar: NotificationBar::new(),
            setup_wizard: SetupWizard::new(),
            setup_wizard_open: false,
            session: SessionDesc::default(),
            dashboard_requests_sender,
            server_events_receiver,
        }
    }
}

impl eframe::App for Dashboard {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        for event in self.server_events_receiver.try_iter() {
            match event {
                ServerEvent::Event(event) => {
                    self.logs_tab.push_event(event.clone());

                    match event.event_type {
                        EventType::GraphStatistics(graph_statistics) => self
                            .statistics_tab
                            .update_graph_statistics(graph_statistics),
                        EventType::Statistics(statistics) => {
                            self.statistics_tab.update_statistics(statistics)
                        }
                        EventType::Session(session) => {
                            let settings = session.to_settings();

                            self.settings_tab.update_session(&session.session_settings);
                            self.logs_tab.update_settings(&settings);
                            self.notification_bar.update_settings(&settings);
                            if self.just_opened {
                                if settings.extra.open_setup_wizard {
                                    self.setup_wizard_open = true;
                                }

                                self.just_opened = false;
                            }

                            self.session = *session;
                        }
                        EventType::ServerRequestsSelfRestart => {
                            if !self.server_restarting.value() {
                                self.server_restarting.set(true);

                                let server_restarting = Arc::clone(&self.server_restarting);
                                thread::spawn(move || {
                                    LAUNCHER.lock().restart_steamvr();

                                    server_restarting.set(false);
                                });
                            }
                        }
                        EventType::Log(event) => {
                            self.notification_bar.push_notification(event);
                        }
                        _ => (),
                    }
                }
                ServerEvent::PingResponseConnected => {
                    self.connected_to_server = true;
                    self.installation_tab.update_drivers();
                }
                ServerEvent::PingResponseDisconnected => {
                    self.connected_to_server = false;
                    self.installation_tab.update_drivers();
                }
                ServerEvent::AudioDevicesUpdated(list) => {
                    self.settings_tab.update_audio_devices(list);
                }
                _ => (),
            }
        }

        if self.server_restarting.value() {
            CentralPanel::default().show(context, |ui| {
                // todo: find a way to center both vertically and horizontally
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading(RichText::new("StreamVR is restarting").size(30.0));
                });
            });

            return;
        }

        self.notification_bar.ui(context);

        let mut requests = vec![];

        if self.setup_wizard_open {
            CentralPanel::default().show(context, |ui| {
                if let Some(SetupWizardRequest::Close { finished }) = self.setup_wizard.ui(ui) {
                    if finished {
                        requests.push(DashboardRequest::SetSingleValue {
                            path: alvr_sockets::parse_path(
                                "session_settings.extra.open_setup_wizard",
                            ),
                            new_value: serde_json::Value::Bool(false),
                        });
                    }

                    self.setup_wizard_open = false;
                }
            });
        } else {
            SidePanel::left("side_panel")
                .resizable(false)
                .frame(
                    Frame::none()
                        .fill(theme::LIGHTER_BG)
                        .inner_margin(Margin::same(7.0))
                        .stroke(Stroke::new(1.0, theme::SEPARATOR_BG)),
                )
                .exact_width(150.0)
                .show(context, |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                        ui.add_space(13.0);
                        ui.heading(RichText::new("ALVR").size(25.0).strong());
                        egui::warn_if_debug_build(ui);
                    });

                    ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                        for (tab, label) in &self.tab_labels {
                            ui.selectable_value(&mut self.selected_tab, *tab, *label);
                        }
                    });

                    ui.with_layout(
                        Layout::bottom_up(Align::Center).with_cross_justify(true),
                        |ui| {
                            ui.add_space(5.0);
                            if self.connected_to_server {
                                if ui.button("Restart SteamVR").clicked() {
                                    requests.push(DashboardRequest::RestartSteamvr);
                                }
                            } else if ui.button("Launch SteamVR").clicked() {
                                thread::spawn(|| LAUNCHER.lock().launch_steamvr());
                            }

                            ui.horizontal(|ui| {
                                ui.add_space(5.0);
                                ui.label("Streamer:");
                                ui.add_space(-10.0);
                                if self.connected_to_server {
                                    ui.label(RichText::new("Connected").color(Color32::GREEN));
                                } else {
                                    ui.label(RichText::new("Disconnected").color(Color32::RED));
                                }
                            })
                        },
                    )
                });

            CentralPanel::default()
                .frame(
                    Frame::none()
                        .inner_margin(Margin::same(20.0))
                        .fill(theme::BG),
                )
                .show(context, |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                        ui.heading(
                            RichText::new(*self.tab_labels.get(&self.selected_tab).unwrap())
                                .size(25.0),
                        );
                        ScrollArea::new([false, true]).show(ui, |ui| match self.selected_tab {
                            Tab::Connections => {
                                if let Some(request) = self.connections_tab.ui(
                                    ui,
                                    &self.session,
                                    self.connected_to_server,
                                ) {
                                    requests.push(request);
                                }
                            }
                            Tab::Statistics => {
                                if let Some(request) = self.statistics_tab.ui(ui) {
                                    requests.push(request);
                                }
                            }
                            Tab::Settings => {
                                requests.extend(self.settings_tab.ui(ui));
                            }
                            Tab::Installation => {
                                if matches!(
                                    self.installation_tab.ui(ui),
                                    Some(InstallationTabRequest::OpenSetupWizard)
                                ) {
                                    self.setup_wizard_open = true;
                                }
                            }
                            Tab::Logs => self.logs_tab.ui(ui),
                            Tab::About => components::about_tab_ui(ui),
                        })
                    })
                });
        }

        for request in requests {
            self.dashboard_requests_sender.send(request).ok();
        }
    }

    fn on_close_event(&mut self) -> bool {
        true
    }
}
