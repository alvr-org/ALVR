use std::{
    env, fs,
    path::PathBuf,
    sync::{mpsc, Arc},
    task::Poll,
    thread,
    time::Duration,
};

use dashboard::dashboard::DashboardResponse;
use futures_util::{StreamExt, TryStreamExt};
use tokio_tungstenite::connect_async;

struct ALVRDashboard {
    dashboard: dashboard::dashboard::Dashboard,
    tx2: mpsc::Sender<GuiMsg>,
    rx1: mpsc::Receiver<WorkerMsg>,
}

enum GuiMsg {
    Dashboard(dashboard::dashboard::DashboardResponse),
    GetSession,
    GetDrivers,
    Quit,
}

enum WorkerMsg {
    Event(Vec<alvr_events::Event>),
    SessionResponse(alvr_session::SessionDesc),
    DriverResponse(Vec<String>),
}

impl ALVRDashboard {
    fn new(
        cc: &eframe::CreationContext<'_>,
        tx2: mpsc::Sender<GuiMsg>,
        rx1: mpsc::Receiver<WorkerMsg>,
    ) -> Self {
        let dir = PathBuf::from(env::var("DIR").unwrap());

        tx2.send(GuiMsg::GetSession).unwrap();
        let session = loop {
            match rx1.recv().unwrap() {
                WorkerMsg::SessionResponse(session) => break session,
                _ => (),
            }
        };
        tx2.send(GuiMsg::GetDrivers).unwrap();
        let drivers = loop {
            match rx1.recv().unwrap() {
                WorkerMsg::DriverResponse(drivers) => break drivers,
                _ => (),
            }
        };

        let mut dashboard = dashboard::dashboard::Dashboard::new(
            session,
            drivers,
            Arc::new(
                dashboard::translation::TranslationBundle::new(
                    Some("en".to_string()),
                    &std::fs::read_to_string(dir.join("languages").join("list.json")).unwrap(),
                    |language_id| {
                        fs::read_to_string(
                            dir.join("languages").join(format!("{}.ftl", language_id)),
                        )
                        .unwrap()
                    },
                )
                .unwrap(),
            ),
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
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        for msg in self.rx1.try_iter() {
            match msg {
                _ => (),
            }
        }

        match self.dashboard.update(ctx, &[]) {
            Some(response) => {
                match response {
                    // These are the responses we don't want to pass to the worker thread
                    DashboardResponse::PresetInvocation(_) | DashboardResponse::SetupWizard(_) => {
                        ()
                    }
                    _ => {
                        self.tx2.send(GuiMsg::Dashboard(response)).unwrap();
                    }
                }
            }
            None => (),
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

    let handle = thread::spawn(|| http_thread(tx1, rx2));

    eframe::run_native(
        "ALVR Dashboard",
        native_options,
        Box::new(|cc| Box::new(ALVRDashboard::new(cc, tx2, rx1))),
    );

    handle.join().unwrap();
}

fn http_thread(tx1: mpsc::Sender<WorkerMsg>, rx2: mpsc::Receiver<GuiMsg>) {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let client = reqwest::Client::builder().build().unwrap();
        let (event_stream, _) =
            connect_async(url::Url::parse("ws://localhost:8082/api/events").unwrap())
                .await
                .unwrap();
        let (log_stream, _) =
            connect_async(url::Url::parse("ws://localhost:8082/api/log").unwrap())
                .await
                .unwrap();

        let (_, event_read) = event_stream.split();

        'main: loop {
            for msg in rx2.try_iter() {
                println!("Received MSG");
                match msg {
                    GuiMsg::Quit => break 'main,
                    GuiMsg::GetSession => {
                        let response = client
                            .get("http://localhost:8082/api/session/load")
                            .send()
                            .await
                            .unwrap();

                        let text = response.text().await.unwrap();

                        println!("{}", text);

                        tx1.send(WorkerMsg::SessionResponse(
                            serde_json::from_str::<alvr_session::SessionDesc>(&text).unwrap(),
                        ))
                        .unwrap();
                    }
                    GuiMsg::GetDrivers => {
                        tx1.send(WorkerMsg::DriverResponse(Vec::new())).unwrap();
                    }
                    GuiMsg::Dashboard(response) => match response {
                        DashboardResponse::SessionUpdated(session) => {
                            let text =
                                serde_json::to_string(&serde_json::json!({ "session": session }))
                                    .unwrap();
                            println!("{}", text);
                            let response = client
                                .get("http://localhost:8082/api/session/store")
                                .body(text)
                                .send()
                                .await
                                .unwrap();
                            if !response.status().is_success() {
                                println!(
                                    "HTTP request returned an error: {:?}",
                                    response.error_for_status().unwrap()
                                );
                            }
                        }
                        DashboardResponse::RestartSteamVR => {
                            client
                                .get("http://localhost:8082/restart-steamvr")
                                .send()
                                .await
                                .unwrap();
                        }
                        _ => (),
                    },
                }
            }
            // With each iteration we should sleep to not consume a thread fully
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
}
