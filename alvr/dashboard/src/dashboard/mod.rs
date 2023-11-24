mod basic_components;
mod components;

use self::components::{
    ConnectionsTab, LogsTab, NotificationBar, SettingsTab, SetupWizard, SetupWizardRequest,
};
use crate::{dashboard::components::StatisticsTab, DataSources};
use alvr_common::parking_lot::{Condvar, Mutex};
use alvr_events::EventType;
use alvr_gui_common::theme;
use alvr_packets::{PathValuePair, ServerRequest};
use alvr_session::SessionConfig;
use eframe::egui::{
    self, style::Margin, Align, CentralPanel, Frame, Layout, RichText, SidePanel, Stroke,
};
use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{atomic::AtomicUsize, Arc},
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
    #[cfg(not(target_arch = "wasm32"))]
    Installation,
    Logs,
    Debug,
    About,
}

pub struct Dashboard {
    data_sources: DataSources,
    just_opened: bool,
    server_restarting: Arc<Mutex<bool>>,
    server_restarting_condvar: Arc<Condvar>,
    selected_tab: Tab,
    tab_labels: BTreeMap<Tab, &'static str>,
    connections_tab: ConnectionsTab,
    statistics_tab: StatisticsTab,
    settings_tab: SettingsTab,
    #[cfg(not(target_arch = "wasm32"))]
    installation_tab: components::InstallationTab,
    logs_tab: LogsTab,
    notification_bar: NotificationBar,
    setup_wizard: SetupWizard,
    setup_wizard_open: bool,
    session: Option<SessionConfig>,
}

impl Dashboard {
    pub fn new(creation_context: &eframe::CreationContext<'_>, data_sources: DataSources) -> Self {
        alvr_gui_common::theme::set_theme(&creation_context.egui_ctx);

        // Audio devices need to be queried early to mitigate buggy/slow hardware queries on Linux.
        data_sources.request(ServerRequest::GetSession);
        data_sources.request(ServerRequest::GetAudioDevices);

        Self {
            data_sources,
            just_opened: true,
            server_restarting: Arc::new(Mutex::new(false)),
            server_restarting_condvar: Arc::new(Condvar::new()),
            selected_tab: Tab::Connections,
            tab_labels: [
                (Tab::Connections, "🔌  Connections"),
                (Tab::Statistics, "📈  Statistics"),
                (Tab::Settings, "⚙  Settings"),
                #[cfg(not(target_arch = "wasm32"))]
                (Tab::Installation, "💾  Installation"),
                (Tab::Logs, "📝  Logs"),
                (Tab::Debug, "🐞  Debug"),
                (Tab::About, "ℹ  About"),
            ]
            .into_iter()
            .collect(),
            connections_tab: ConnectionsTab::new(),
            statistics_tab: StatisticsTab::new(),
            settings_tab: SettingsTab::new(),
            #[cfg(not(target_arch = "wasm32"))]
            installation_tab: components::InstallationTab::new(),
            logs_tab: LogsTab::new(),
            notification_bar: NotificationBar::new(),
            setup_wizard: SetupWizard::new(),
            setup_wizard_open: false,
            session: None,
        }
    }

    // This call may block
    fn restart_steamvr(&self, requests: &mut Vec<ServerRequest>) {
        requests.push(ServerRequest::RestartSteamvr);

        let mut server_restarting_lock = self.server_restarting.lock();

        if *server_restarting_lock {
            self.server_restarting_condvar
                .wait(&mut server_restarting_lock);
        }

        *server_restarting_lock = true;

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn({
            let server_restarting = Arc::clone(&self.server_restarting);
            let condvar = Arc::clone(&self.server_restarting_condvar);
            move || {
                crate::steamvr_launcher::LAUNCHER.lock().restart_steamvr();

                *server_restarting.lock() = false;
                condvar.notify_one();
            }
        });
    }
}

impl eframe::App for Dashboard {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        let mut requests = vec![];

        let connected_to_server = self.data_sources.server_connected();

        while let Some(event) = self.data_sources.poll_event() {
            self.logs_tab.push_event(event.inner.clone());

            match event.inner.event_type {
                EventType::Log(log_event) => {
                    self.notification_bar
                        .push_notification(log_event, event.from_dashboard);
                }
                EventType::GraphStatistics(graph_statistics) => self
                    .statistics_tab
                    .update_graph_statistics(graph_statistics),
                EventType::StatisticsSummary(statistics) => {
                    self.statistics_tab.update_statistics(statistics)
                }
                EventType::Session(session) => {
                    let settings = session.to_settings();

                    self.connections_tab.update_client_list(&session);
                    self.settings_tab.update_session(&session.session_settings);
                    self.logs_tab.update_settings(&settings);
                    self.notification_bar.update_settings(&settings);
                    if self.just_opened {
                        if settings.open_setup_wizard {
                            self.setup_wizard_open = true;
                        }

                        self.just_opened = false;
                    }

                    self.session = Some(*session);
                }
                EventType::ServerRequestsSelfRestart => self.restart_steamvr(&mut requests),
                EventType::AudioDevices(list) => self.settings_tab.update_audio_devices(list),
                #[cfg(not(target_arch = "wasm32"))]
                EventType::DriversList(list) => self.installation_tab.update_drivers(list),
                _ => (),
            }
        }

        if *self.server_restarting.lock() {
            CentralPanel::default().show(context, |ui| {
                // todo: find a way to center both vertically and horizontally
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading(RichText::new("SteamVR is restarting").size(30.0));
                });
            });

            return;
        }

        self.notification_bar.ui(context);

        if self.setup_wizard_open {
            CentralPanel::default().show(context, |ui| {
                if let Some(request) = self.setup_wizard.ui(ui) {
                    match request {
                        SetupWizardRequest::ServerRequest(request) => {
                            requests.push(request);
                        }
                        SetupWizardRequest::Close { finished } => {
                            if finished {
                                requests.push(ServerRequest::SetValues(vec![PathValuePair {
                                    path: alvr_packets::parse_path(
                                        "session_settings.open_setup_wizard",
                                    ),
                                    value: serde_json::Value::Bool(false),
                                }]))
                            }

                            self.setup_wizard_open = false;
                        }
                    }
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

                    #[cfg(not(target_arch = "wasm32"))]
                    ui.with_layout(
                        Layout::bottom_up(Align::Center).with_cross_justify(true),
                        |ui| {
                            ui.add_space(5.0);

                            if connected_to_server {
                                if ui.button("Restart SteamVR").clicked() {
                                    self.restart_steamvr(&mut requests);
                                }
                            } else if ui.button("Launch SteamVR").clicked() {
                                crate::steamvr_launcher::LAUNCHER.lock().launch_steamvr();
                            }

                            ui.horizontal(|ui| {
                                ui.add_space(5.0);
                                ui.label(RichText::new("SteamVR:").size(13.0));
                                ui.add_space(-10.0);
                                if connected_to_server {
                                    ui.label(
                                        RichText::new("Connected")
                                            .color(theme::OK_GREEN)
                                            .size(13.0),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new("Disconnected")
                                            .color(theme::KO_RED)
                                            .size(13.0),
                                    );
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
                        match self.selected_tab {
                            Tab::Connections => {
                                requests.extend(self.connections_tab.ui(ui, connected_to_server));
                            }
                            Tab::Statistics => {
                                if let Some(request) = self.statistics_tab.ui(ui) {
                                    requests.push(request);
                                }
                            }
                            Tab::Settings => {
                                requests.extend(self.settings_tab.ui(ui));
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            Tab::Installation => {
                                for request in self.installation_tab.ui(ui) {
                                    match request {
                                        components::InstallationTabRequest::OpenSetupWizard => {
                                            self.setup_wizard_open = true
                                        }
                                        components::InstallationTabRequest::ServerRequest(
                                            request,
                                        ) => {
                                            requests.push(request);
                                        }
                                    }
                                }
                            }
                            Tab::Logs => self.logs_tab.ui(ui),
                            Tab::Debug => {
                                if let Some(request) = components::debug_tab_ui(ui) {
                                    requests.push(request);
                                }
                            }
                            Tab::About => components::about_tab_ui(ui),
                        }
                    })
                });
        }

        for request in requests {
            self.data_sources.request(request);
        }

        if context.input(|state| state.viewport().close_requested())
            && self
                .session
                .as_ref()
                .map(|s| {
                    s.to_settings()
                        .steamvr_launcher
                        .open_close_steamvr_with_dashboard
                })
                .unwrap_or(false)
        {
            self.data_sources.request(ServerRequest::ShutdownSteamvr);

            crate::steamvr_launcher::LAUNCHER
                .lock()
                .ensure_steamvr_shutdown()
        }
    }
}
