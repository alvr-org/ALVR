use std::{
    env, fs,
    path::PathBuf,
    sync::{
        mpsc::{self},
        Arc,
    },
    thread,
    time::Duration,
};

use dashboard::dashboard::DashboardResponse;
use futures_util::StreamExt;
use tokio::sync::mpsc::error::TryRecvError;
use tokio_tungstenite::{connect_async, tungstenite};

const BASE_URL: &str = "http://localhost:8082";
const BASE_WS_URL: &str = "ws://localhost:8082";

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
        let dir = PathBuf::from(env::var("DIR").unwrap());

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

        match self.dashboard.update(ctx) {
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

async fn websocket_task<T: serde::de::DeserializeOwned + std::fmt::Debug>(
    url: url::Url,
    sender: tokio::sync::mpsc::Sender<T>,
    mut recv: tokio::sync::broadcast::Receiver<()>,
) {
    let (event_stream, _) = connect_async(url).await.unwrap();
    let (_, event_read) = event_stream.split();

    tokio::select! {
        _ = event_read.for_each(|msg| async {
            match msg {
                Ok(
                tungstenite::Message::Text(text)) => {
                    let event = serde_json::from_str::<T>(&text).unwrap();

                    sender.send(event).await.unwrap();
                }
                Ok(_) => (),
                Err(_why) => (),
            }
        }) => {},
        _ = recv.recv() => {},
    };
}

fn http_thread(tx1: mpsc::Sender<WorkerMsg>, rx2: mpsc::Receiver<GuiMsg>) {
    use tokio::sync::{broadcast, mpsc};
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let client = reqwest::Client::builder().build().unwrap();

        // Communication with the event thread
        let (broadcast_tx, _) = broadcast::channel(1);
        let mut event_rx = None;

        let mut connected = false;

        'main: loop {
            match client.get(BASE_URL).send().await {
                Ok(_) => {
                    if !connected {
                        let (event_tx, _event_rx) = mpsc::channel::<alvr_events::EventType>(1);
                        tokio::task::spawn(websocket_task(
                            url::Url::parse(&format!("{}/api/events", BASE_WS_URL)).unwrap(),
                            event_tx,
                            broadcast_tx.subscribe(),
                        ));
                        event_rx = Some(_event_rx);
                        tx1.send(WorkerMsg::Connected).unwrap();
                        connected = true;
                    }
                }
                Err(why) => {
                    let _ = broadcast_tx.send(());
                    connected = false;

                    // We still check for the exit signal from the Gui thread
                    for msg in rx2.try_iter() {
                        if let GuiMsg::Quit = msg {
                            break 'main;
                        }
                    }

                    tx1.send(WorkerMsg::LostConnection(format!("{}", why)))
                        .unwrap();
                }
            }

            // If we are not connected, don't even attempt to continue normal working order
            if !connected {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            loop {
                match event_rx.as_mut().unwrap().try_recv() {
                    Ok(event) => {
                        tx1.send(WorkerMsg::Event(event)).unwrap();
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(_) => break,
                }
            }

            for msg in rx2.try_iter() {
                match msg {
                    GuiMsg::Quit => break 'main,
                    GuiMsg::GetSession => {
                        let response = client
                            .get(format!("{}/api/session/load", BASE_URL))
                            .send()
                            .await
                            .unwrap();

                        tx1.send(WorkerMsg::SessionResponse(
                            response.json::<alvr_session::SessionDesc>().await.unwrap(),
                        ))
                        .unwrap();
                    }
                    GuiMsg::GetDrivers => {
                        let response = client
                            .get(format!("{}/api/driver/list", BASE_URL))
                            .send()
                            .await
                            .unwrap();

                        let vec: Vec<String> = response.json().await.unwrap();

                        tx1.send(WorkerMsg::DriverResponse(vec)).unwrap();
                    }
                    GuiMsg::Dashboard(response) => match response {
                        DashboardResponse::SessionUpdated(session) => {
                            let text = serde_json::to_string(&session).unwrap();
                            let response = client
                                .get(format!("{}/api/session/store", BASE_URL))
                                .body(format!("{{\"session\": {}}}", text))
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
                                .get(format!("{}/restart-steamvr", BASE_URL))
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
        // Shutdown the event thread if needed, an error would only mean that the event thread is already dead so we ignore it
        let _ = broadcast_tx.send(());
    });
}
