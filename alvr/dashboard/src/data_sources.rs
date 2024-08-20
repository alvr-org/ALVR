use alvr_common::{debug, error, info, parking_lot::Mutex, warn, RelaxedAtomic};
use alvr_events::{Event, EventType};
use alvr_packets::ServerRequest;
use alvr_server_io::ServerSessionManager;
use eframe::egui;
use std::{
    env,
    io::ErrorKind,
    net::{SocketAddr, TcpStream},
    str::FromStr,
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tungstenite::http::Uri;

const REQUEST_TIMEOUT: Duration = Duration::from_millis(200);

enum SessionSource {
    Local(Box<ServerSessionManager>),
    Remote, // Note: the remote (server) is probably living as a separate process in the same PC
}

pub fn get_local_session_source() -> ServerSessionManager {
    let session_file_path =
        alvr_filesystem::filesystem_layout_from_dashboard_exe(&env::current_exe().unwrap())
            .session();

    ServerSessionManager::new(Some(session_file_path))
}

fn report_event_local(
    context: &egui::Context,
    sender: &mpsc::Sender<PolledEvent>,
    event_type: EventType,
) {
    sender
        .send(PolledEvent {
            inner: Event {
                timestamp: "".into(),
                event_type,
            },
            from_dashboard: false,
        })
        .ok();
    context.request_repaint();
}

fn report_session_local(
    context: &egui::Context,
    sender: &mpsc::Sender<PolledEvent>,
    session_manager: &mut ServerSessionManager,
) {
    report_event_local(
        context,
        sender,
        EventType::Session(Box::new(session_manager.session().clone())),
    )
}

pub struct PolledEvent {
    pub inner: Event,
    pub from_dashboard: bool,
}

pub struct DataSources {
    running: Arc<RelaxedAtomic>,
    requests_sender: mpsc::Sender<ServerRequest>,
    events_receiver: mpsc::Receiver<PolledEvent>,
    server_connected: Arc<RelaxedAtomic>,
    requests_thread: Option<JoinHandle<()>>,
    events_thread: Option<JoinHandle<()>>,
    ping_thread: Option<JoinHandle<()>>,
}

impl DataSources {
    pub fn new(
        context: egui::Context,
        events_sender: mpsc::Sender<PolledEvent>,
        events_receiver: mpsc::Receiver<PolledEvent>,
    ) -> Self {
        let running = Arc::new(RelaxedAtomic::new(true));
        let (requests_sender, requests_receiver) = mpsc::channel();
        let server_connected = Arc::new(RelaxedAtomic::new(false));

        let session_manager = get_local_session_source();
        let port = session_manager.settings().connection.web_server_port;
        let session_source = Arc::new(Mutex::new(SessionSource::Local(Box::new(session_manager))));

        let requests_thread = thread::spawn({
            let running = Arc::clone(&running);
            let context = context.clone();
            let session_source = Arc::clone(&session_source);
            let events_sender = events_sender.clone();
            move || {
                let uri = format!("http://127.0.0.1:{port}/api/dashboard-request");
                let request_agent = ureq::AgentBuilder::new()
                    .timeout_connect(REQUEST_TIMEOUT)
                    .build();

                while running.value() {
                    while let Ok(request) = requests_receiver.try_recv() {
                        debug!("Dashboard request: {request:?}");

                        if let SessionSource::Local(session_manager) = &mut *session_source.lock() {
                            match request {
                                ServerRequest::Log(_) => (),
                                ServerRequest::GetSession => {
                                    report_session_local(&context, &events_sender, session_manager);
                                }
                                ServerRequest::UpdateSession(session) => {
                                    *session_manager.session_mut() = *session;

                                    report_session_local(&context, &events_sender, session_manager);
                                }
                                ServerRequest::SetValues(descs) => {
                                    if let Err(e) = session_manager.set_values(descs) {
                                        error!("Failed to set session value: {e}")
                                    }

                                    report_session_local(&context, &events_sender, session_manager);
                                }
                                ServerRequest::UpdateClientList { hostname, action } => {
                                    session_manager.update_client_list(hostname, action);

                                    report_session_local(&context, &events_sender, session_manager);
                                }
                                ServerRequest::GetAudioDevices => {
                                    if let Ok(list) = session_manager.get_audio_devices_list() {
                                        report_event_local(
                                            &context,
                                            &events_sender,
                                            EventType::AudioDevices(list),
                                        )
                                    }
                                }
                                ServerRequest::FirewallRules(action) => {
                                    if alvr_server_io::firewall_rules(action).is_ok() {
                                        info!("Setting firewall rules succeeded!");
                                    } else {
                                        error!("Setting firewall rules failed!");
                                    }
                                }
                                ServerRequest::RegisterAlvrDriver => {
                                    let alvr_driver_dir =
                                        alvr_filesystem::filesystem_layout_from_dashboard_exe(
                                            &env::current_exe().unwrap(),
                                        )
                                        .openvr_driver_root_dir;

                                    alvr_server_io::driver_registration(&[alvr_driver_dir], true)
                                        .ok();

                                    if let Ok(list) = alvr_server_io::get_registered_drivers() {
                                        report_event_local(
                                            &context,
                                            &events_sender,
                                            EventType::DriversList(list),
                                        )
                                    }
                                }
                                ServerRequest::UnregisterDriver(path) => {
                                    alvr_server_io::driver_registration(&[path], false).ok();

                                    if let Ok(list) = alvr_server_io::get_registered_drivers() {
                                        report_event_local(
                                            &context,
                                            &events_sender,
                                            EventType::DriversList(list),
                                        )
                                    }
                                }
                                ServerRequest::GetDriverList => {
                                    if let Ok(list) = alvr_server_io::get_registered_drivers() {
                                        report_event_local(
                                            &context,
                                            &events_sender,
                                            EventType::DriversList(list),
                                        )
                                    }
                                }
                                ServerRequest::CaptureFrame
                                | ServerRequest::InsertIdr
                                | ServerRequest::StartRecording
                                | ServerRequest::StopRecording => {
                                    warn!("Cannot perform action, streamer (SteamVR) is not connected.")
                                }
                                ServerRequest::RestartSteamvr | ServerRequest::ShutdownSteamvr => {
                                    warn!("Streamer not launched, can't signal SteamVR shutdown")
                                }
                            }
                        } else {
                            request_agent.get(&uri).send_json(&request).ok();
                        }
                    }

                    thread::sleep(Duration::from_millis(100));
                }
            }
        });

        let events_thread = thread::spawn({
            let running = Arc::clone(&running);
            move || {
                while running.value() {
                    let uri = Uri::from_str(&format!("ws://127.0.0.1:{port}/api/events")).unwrap();

                    let maybe_socket = TcpStream::connect_timeout(
                        &SocketAddr::from_str(&format!("127.0.0.1:{port}")).unwrap(),
                        Duration::from_millis(500),
                    );
                    let socket = if let Ok(socket) = maybe_socket {
                        socket
                    } else {
                        thread::sleep(Duration::from_millis(500));

                        continue;
                    };

                    let mut ws = if let Ok((ws, _)) = tungstenite::client(uri, socket) {
                        ws
                    } else {
                        thread::sleep(Duration::from_millis(500));

                        continue;
                    };

                    ws.get_mut().set_nonblocking(true).ok();

                    while running.value() {
                        match ws.read() {
                            Ok(tungstenite::Message::Text(json_string)) => {
                                if let Ok(event) = serde_json::from_str(&json_string) {
                                    debug!("Server event received: {:?}", event);
                                    events_sender
                                        .send(PolledEvent {
                                            inner: event,
                                            from_dashboard: false,
                                        })
                                        .ok();
                                    context.request_repaint();
                                }
                            }
                            Err(e) => {
                                if let tungstenite::Error::Io(e) = e {
                                    if e.kind() == ErrorKind::WouldBlock {
                                        thread::sleep(Duration::from_millis(50));

                                        continue;
                                    }
                                }

                                break;
                            }
                            _ => (),
                        }
                    }
                }
            }
        });

        let ping_thread = thread::spawn({
            let running = Arc::clone(&running);
            let data_source = Arc::clone(&session_source);
            let server_connected = Arc::clone(&server_connected);
            move || {
                const PING_INTERVAL: Duration = Duration::from_secs(1);
                let mut deadline = Instant::now();
                let uri = format!("http://127.0.0.1:{port}/api/ping");

                let request_agent = ureq::AgentBuilder::new()
                    .timeout_connect(REQUEST_TIMEOUT)
                    .build();

                loop {
                    let connected = request_agent.get(&uri).call().is_ok();

                    {
                        let mut data_source_lock = data_source.lock();
                        if connected && matches!(*data_source_lock, SessionSource::Local(_)) {
                            info!("Server connected");
                            *data_source_lock = SessionSource::Remote;
                        } else if !connected && matches!(*data_source_lock, SessionSource::Remote) {
                            info!("Server disconnected");
                            *data_source_lock =
                                SessionSource::Local(Box::new(get_local_session_source()));
                        }
                    }

                    server_connected.set(connected);

                    deadline += PING_INTERVAL;

                    while Instant::now() < deadline {
                        if !running.value() {
                            return;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });

        Self {
            requests_sender,
            events_receiver,
            server_connected,
            running,
            requests_thread: Some(requests_thread),
            events_thread: Some(events_thread),
            ping_thread: Some(ping_thread),
        }
    }

    pub fn request(&self, request: ServerRequest) {
        self.requests_sender.send(request).ok();
    }

    pub fn poll_event(&self) -> Option<PolledEvent> {
        self.events_receiver.try_recv().ok()
    }

    pub fn server_connected(&self) -> bool {
        self.server_connected.value()
    }
}

impl Drop for DataSources {
    fn drop(&mut self) {
        self.running.set(false);

        self.requests_thread.take().unwrap().join().ok();
        self.events_thread.take().unwrap().join().ok();
        self.ping_thread.take().unwrap().join().ok();
    }
}
