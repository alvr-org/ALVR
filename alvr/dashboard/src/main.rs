#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{
    sync::{mpsc, Arc},
    thread,
};

mod worker;

use alvr_dashboard::dashboard::DashboardResponse;

struct ALVRDashboard {
    dashboard: alvr_dashboard::dashboard::Dashboard,
    tx2: mpsc::Sender<GuiMsg>,
    rx1: mpsc::Receiver<WorkerMsg>,
}

pub enum GuiMsg {
    Dashboard(alvr_dashboard::dashboard::DashboardResponse),
    GetSession,
    GetDrivers,
    Quit,
}

pub enum WorkerMsg {
    Event(alvr_events::EventType),
    SessionResponse(alvr_session::SessionDesc),
    DriverResponse(Vec<String>),
    LostConnection(String),
    Connected,
}

impl ALVRDashboard {
    fn new(
        cc: &eframe::CreationContext<'_>,
        tx2: mpsc::Sender<GuiMsg>,
        rx1: mpsc::Receiver<WorkerMsg>,
    ) -> Self {
        tx2.send(GuiMsg::GetSession).unwrap();
        let session = loop {
            match rx1.recv().unwrap() {
                WorkerMsg::SessionResponse(session) => break session,
                WorkerMsg::LostConnection(_) => break alvr_session::SessionDesc::default(),
                _ => (),
            }
        };
        tx2.send(GuiMsg::GetDrivers).unwrap();
        let (drivers, connected) = loop {
            match rx1.recv().unwrap() {
                WorkerMsg::DriverResponse(drivers) => break (drivers, None),
                WorkerMsg::LostConnection(why) => break (Vec::new(), Some(why)),
                _ => (),
            }
        };

        let mut dashboard = alvr_dashboard::dashboard::Dashboard::new(
            session,
            drivers,
            Arc::new(
                alvr_dashboard::translation::TranslationBundle::new(
                    Some("en".to_string()),
                    r#"{ "en": "English" }"#,
                    |_language_id| "".to_string(),
                )
                .unwrap(),
            ),
            connected,
        );
        dashboard.setup(&cc.egui_ctx);

        Self {
            dashboard,
            tx2,
            rx1,
        }
    }
}

impl eframe::App for ALVRDashboard {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        for msg in self.rx1.try_iter() {
            match msg {
                WorkerMsg::Event(event) => {
                    self.dashboard.new_event(event);
                }
                WorkerMsg::DriverResponse(drivers) => {
                    self.dashboard.new_drivers(drivers);
                }
                WorkerMsg::LostConnection(why) => {
                    self.dashboard.connection_status(Some(why));
                }
                WorkerMsg::Connected => {
                    self.dashboard.connection_status(None);
                    self.tx2.send(GuiMsg::GetSession).unwrap();
                    self.tx2.send(GuiMsg::GetDrivers).unwrap();
                }
                WorkerMsg::SessionResponse(session) => {
                    self.dashboard
                        .new_event(alvr_events::EventType::Session(Box::new(session)));
                }
            }
        }

        if let Some(response) = self.dashboard.update(ctx) {
            match response {
                // These are the responses we don't want to pass to the worker thread
                DashboardResponse::PresetInvocation(_) | DashboardResponse::SetupWizard(_) => {}
                _ => {
                    self.tx2.send(GuiMsg::Dashboard(response)).unwrap();
                }
            }
        }
    }

    fn on_close_event(&mut self) -> bool {
        self.tx2.send(GuiMsg::Quit).unwrap();
        true
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();

    let (tx1, rx1) = mpsc::channel::<WorkerMsg>();
    let (tx2, rx2) = mpsc::channel::<GuiMsg>();

    let handle = thread::spawn(|| worker::http_thread(tx1, rx2));

    eframe::run_native(
        "ALVR Dashboard",
        native_options,
        Box::new(|cc| Box::new(ALVRDashboard::new(cc, tx2, rx1))),
    );

    handle.join().unwrap();
}
