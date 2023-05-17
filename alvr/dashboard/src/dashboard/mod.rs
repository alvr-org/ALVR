mod basic_components;
mod components;

use self::components::{
    ConnectionsTab, InstallationTab, InstallationTabRequest, LogsTab, NotificationBar, SettingsTab,
    SetupWizard, SetupWizardRequest,
};
use crate::{dashboard::components::StatisticsTab, theme, DataSources};
use alvr_common::RelaxedAtomic;
use alvr_events::EventType;
use alvr_packets::{PathValuePair, ServerRequest};
use alvr_session::SessionDesc;
use eframe::egui::{
    self, style::Margin, Align, CentralPanel, Frame, Layout, RichText, ScrollArea, SidePanel,
    Stroke,
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
    Installation,
    Logs,
    Debug,
    About,
}

pub struct Dashboard {
    data_sources: DataSources,
    just_opened: bool,
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
}

impl Dashboard {
    pub fn new(creation_context: &eframe::CreationContext<'_>, data_sources: DataSources) -> Self {
        data_sources.request(ServerRequest::GetSession);
        data_sources.request(ServerRequest::GetAudioDevices);
        data_sources.request(ServerRequest::GetDriverList);

        theme::set_theme(&creation_context.egui_ctx);

        Self {
            data_sources,
            just_opened: true,
            server_restarting: Arc::new(RelaxedAtomic::new(false)),
            selected_tab: Tab::Connections,
            tab_labels: [
                (Tab::Connections, "🔌  Connections"),
                (Tab::Statistics, "📈  Statistics"),
                (Tab::Settings, "⚙  Settings"),
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
            installation_tab: InstallationTab::new(),
            logs_tab: LogsTab::new(),
            notification_bar: NotificationBar::new(),
            setup_wizard: SetupWizard::new(),
            setup_wizard_open: false,
            session: SessionDesc::default(),
        }
    }
}

impl eframe::App for Dashboard {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        let mut requests = vec![];

        let connected_to_server = self.data_sources.server_connected();

        while let Some(event) = self.data_sources.poll_event() {
            self.logs_tab.push_event(event.clone());

            match event.event_type {
                EventType::Log(event) => {
                    self.notification_bar.push_notification(event);
                }
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
                        if settings.open_setup_wizard {
                            self.setup_wizard_open = true;
                        }

                        self.just_opened = false;
                    }

                    self.session = *session;
                }
                EventType::ServerRequestsSelfRestart => {
                    if !self.server_restarting.value() {
                        self.server_restarting.set(true);

                        #[cfg(not(target_arch = "wasm32"))]
                        std::thread::spawn({
                            let server_restarting = Arc::clone(&self.server_restarting);
                            move || {
                                crate::steamvr_launcher::LAUNCHER.lock().restart_steamvr();

                                server_restarting.set(false);
                            }
                        });
                    }
                }
                EventType::AudioDevices(list) => self.settings_tab.update_audio_devices(list),
                EventType::DriversList(list) => self.installation_tab.update_drivers(list),
                EventType::Tracking(_) | EventType::Buttons(_) | EventType::Haptics(_) => (),
            }
        }

        if self.server_restarting.value() {
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
                            use eframe::epaint::Color32;

                            ui.add_space(5.0);

                            if connected_to_server {
                                if ui.button("Restart SteamVR").clicked() {
                                    requests.push(ServerRequest::RestartSteamvr);
                                }
                            } else if ui.button("Launch SteamVR").clicked() {
                                crate::steamvr_launcher::LAUNCHER.lock().launch_steamvr();
                            }

                            ui.horizontal(|ui| {
                                ui.add_space(5.0);
                                ui.label("Streamer:");
                                ui.add_space(-10.0);
                                if connected_to_server {
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
                                if let Some(request) =
                                    self.connections_tab
                                        .ui(ui, &self.session, connected_to_server)
                                {
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
                                for request in self.installation_tab.ui(ui) {
                                    match request {
                                        InstallationTabRequest::OpenSetupWizard => {
                                            self.setup_wizard_open = true
                                        }
                                        InstallationTabRequest::ServerRequest(request) => {
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
                        })
                    })
                });
        }

        for request in requests {
            self.data_sources.request(request);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_close_event(&mut self) -> bool {
        if crate::data_sources::get_local_data_source()
            .settings()
            .steamvr_launcher
            .open_close_steamvr_with_dashboard
        {
            self.data_sources.request(ServerRequest::ShutdownSteamvr);

            crate::steamvr_launcher::LAUNCHER
                .lock()
                .ensure_steamvr_shutdown()
        }

        true
    }
}
