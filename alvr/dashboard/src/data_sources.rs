use alvr_common::{parking_lot::Mutex, prelude::*, StrResult};
use alvr_events::{Event, EventType};
use alvr_server_data::ServerDataManager;
use alvr_sockets::{AudioDevicesList, DashboardRequest, ServerResponse};
use eframe::egui;
use std::{
    env,
    io::ErrorKind,
    net::{SocketAddr, TcpStream},
    str::FromStr,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use tungstenite::http::Uri;

const REQUEST_TIMEOUT: Duration = Duration::from_millis(200);

pub enum ServerEvent {
    PingResponseConnected,
    PingResponseDisconnected,
    Event(alvr_events::Event),
    AudioDevicesUpdated(AudioDevicesList),
    ChannelTest,
}

enum DataSource {
    Local(ServerDataManager),
    Remote, // Note: the remote (server) is probably living as a separate process in the same PC
}

pub fn get_local_data_source() -> ServerDataManager {
    let session_file_path =
        alvr_filesystem::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap())
            .session();

    ServerDataManager::new(&session_file_path)
}

fn report_event(
    context: &egui::Context,
    sender: &mpsc::Sender<ServerEvent>,
    event: ServerEvent,
) -> StrResult {
    let res = sender.send(event);
    context.request_repaint();

    res.map_err(err!())
}

fn report_server_status(
    context: &egui::Context,
    sender: &mpsc::Sender<ServerEvent>,
    data_source: &Mutex<DataSource>,
    connected: bool,
) -> StrResult {
    let mut data_source_lock = data_source.lock();
    if connected && matches!(*data_source_lock, DataSource::Local(_)) {
        info!("Server connected");
        *data_source_lock = DataSource::Remote;

        report_event(context, sender, ServerEvent::PingResponseConnected)
    } else if !connected && matches!(*data_source_lock, DataSource::Remote) {
        info!("Server disconnected");
        *data_source_lock = DataSource::Local(get_local_data_source());

        report_event(context, sender, ServerEvent::PingResponseDisconnected)
    } else {
        Ok(())
    }
}

fn report_session(
    context: &egui::Context,
    sender: &mpsc::Sender<ServerEvent>,
    data_manager: &mut ServerDataManager,
) {
    report_event(
        context,
        sender,
        ServerEvent::Event(Event {
            timestamp: "".into(),
            event_type: EventType::Session(Box::new(data_manager.session().clone())),
        }),
    )
    .ok();
}

fn check_bail(sender: &mpsc::Sender<ServerEvent>) -> StrResult {
    sender.send(ServerEvent::ChannelTest).map_err(err!())
}

pub fn data_interop_thread(
    context: egui::Context,
    receiver: mpsc::Receiver<DashboardRequest>,
    sender: mpsc::Sender<ServerEvent>,
) {
    let server_data_manager = get_local_data_source();

    let port = server_data_manager.settings().connection.web_server_port;

    let data_source = Arc::new(Mutex::new(DataSource::Local(server_data_manager)));

    let events_thread = thread::spawn({
        let context = context.clone();
        let sender = sender.clone();
        let data_source = Arc::clone(&data_source);
        move || -> StrResult {
            loop {
                let uri = Uri::from_str(&format!("ws://127.0.0.1:{port}/api/events")).unwrap();

                let maybe_socket = TcpStream::connect_timeout(
                    &SocketAddr::from_str(&format!("127.0.0.1:{port}")).unwrap(),
                    Duration::from_millis(500),
                );
                let socket = match maybe_socket {
                    Ok(socket) => socket,
                    Err(_) => {
                        check_bail(&sender)?;

                        continue;
                    }
                };

                let mut ws = if let Ok((ws, _)) = tungstenite::client(uri, socket) {
                    ws
                } else {
                    check_bail(&sender)?;

                    thread::sleep(Duration::from_millis(500));

                    continue;
                };

                ws.get_mut().set_nonblocking(true).ok();

                loop {
                    match ws.read_message() {
                        Ok(tungstenite::Message::Text(json_string)) => {
                            if let Ok(event) = serde_json::from_str(&json_string) {
                                debug!("server event received: {event:?}");
                                report_event(&context, &sender, ServerEvent::Event(event))?;
                            }
                        }
                        Err(e) => {
                            if let tungstenite::Error::Io(e) = e {
                                if e.kind() == ErrorKind::WouldBlock {
                                    check_bail(&sender)?;

                                    thread::sleep(Duration::from_millis(50));

                                    continue;
                                }
                            }

                            report_server_status(&context, &sender, &data_source, false)?;
                            break;
                        }
                        _ => (),
                    }
                }
            }
        }
    });

    let dashboard_request_uri = format!("http://127.0.0.1:{port}/api/dashboard-request");
    let request_agent = ureq::AgentBuilder::new()
        .timeout_connect(REQUEST_TIMEOUT)
        .build();

    let ping_thread = thread::spawn({
        let context = context.clone();
        let sender = sender.clone();
        let data_source = Arc::clone(&data_source);
        let request_agent = request_agent.clone();
        let dashboard_request_uri = dashboard_request_uri.clone();
        move || -> StrResult {
            const PING_INTERVAL: Duration = Duration::from_secs(1);
            let mut deadline = Instant::now();

            loop {
                let response = request_agent
                    .get(&dashboard_request_uri)
                    .send_json(&DashboardRequest::Ping);

                report_server_status(&context, &sender, &data_source, response.is_ok())?;

                deadline += PING_INTERVAL;
                while Instant::now() < deadline {
                    check_bail(&sender)?;
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    });

    while let Ok(request) = receiver.recv() {
        debug!("Dashboard request: {request:?}");

        match request_agent
            .get(&dashboard_request_uri)
            .send_json(&request)
        {
            Ok(response) => {
                if let Ok(ServerResponse::AudioDevices(list)) =
                    response.into_json::<ServerResponse>()
                {
                    report_event(&context, &sender, ServerEvent::AudioDevicesUpdated(list)).ok();
                }
            }
            Err(_) => {
                if let DataSource::Local(data_manager) = &mut *data_source.lock() {
                    match request {
                        DashboardRequest::GetSession => {
                            report_session(&context, &sender, data_manager);
                        }
                        DashboardRequest::UpdateSession(session) => {
                            *data_manager.session_mut() = *session;

                            report_session(&context, &sender, data_manager);
                        }
                        DashboardRequest::SetSingleValue { path, new_value } => {
                            if let Err(e) = data_manager.set_single_value(path.clone(), new_value) {
                                error!("Path: {path:?}, error: {e}")
                            }

                            report_session(&context, &sender, data_manager);
                        }
                        DashboardRequest::ExecuteScript(code) => {
                            if let Err(e) = data_manager.execute_script(&code) {
                                error!("Error executing script: {e}");
                            }

                            report_session(&context, &sender, data_manager);
                        }
                        DashboardRequest::UpdateClientList { hostname, action } => {
                            data_manager.update_client_list(hostname, action);

                            report_session(&context, &sender, data_manager);
                        }
                        DashboardRequest::GetAudioDevices => {
                            if let Ok(list) = data_manager.get_audio_devices_list() {
                                report_event(
                                    &context,
                                    &sender,
                                    ServerEvent::AudioDevicesUpdated(list),
                                )
                                .ok();
                            }
                        }
                        _ => (),
                    }
                } else {
                    warn!("Request has been lost!");
                }
            }
        }
    }

    events_thread.join().ok();
    ping_thread.join().ok();
}
