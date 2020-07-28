mod logging_backend;
mod tail;

use alvr_common::{data::*, logging::*, sockets::*, *};
use futures::SinkExt;
use logging_backend::*;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tail::tail_stream;
use tokio::{
    stream::StreamExt,
    sync::mpsc::{self, *},
    time::timeout,
};
use warp::{
    body, fs as wfs,
    http::StatusCode,
    reply,
    ws::{Message, WebSocket, Ws},
    Filter,
};

type TMutex<T> = tokio::sync::Mutex<T>;

const WEB_GUI_DIR_STR: &str = "web_gui";
const TEST_CONNECTION_TIMEOUT: Duration = Duration::from_millis(500);

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

fn alvr_server_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn try_log_redirect(line: &str, level: log::Level) -> bool {
    let level_label = &format!("[{}]", level);
    if line.starts_with(level_label) {
        let untagged_line = &line[level_label.len() + 1..];
        if level == log::Level::Error {
            show_err::<(), _>(Err(untagged_line)).ok();
        } else {
            log::log!(level, "{}", untagged_line);
        }

        true
    } else {
        false
    }
}

async fn subscribed_to_log(mut socket: WebSocket, mut log_receiver: UnboundedReceiver<String>) {
    while let Some(line) = log_receiver.next().await {
        if let Err(e) = socket.send(Message::text(line)).await {
            info!("Failed to send log with websocket: {}", e);
            break;
        }
    }
}

async fn client_discovery(
    session_manager: Arc<Mutex<SessionManager>>,
    control_socket: Arc<TMutex<Option<ControlSocket<ClientControlPacket, ServerControlPacket>>>>,
) {
    let res = search_client_loop(None, |client_ip| {
        let session_manager = session_manager.clone();
        let control_socket = control_socket.clone();
        async move {
            let now_ms = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            {
                let session_manager_ref = &mut session_manager.lock().unwrap();
                let session_desc_ref = &mut session_manager_ref
                    .get_mut(SERVER_SESSION_UPDATE_ID, SessionUpdateType::ClientList);

                let maybe_known_client_ref = session_desc_ref
                    .last_clients
                    .iter_mut()
                    .find(|connection_desc| connection_desc.ip == client_ip.to_string());

                if let Some(known_client_ref) = maybe_known_client_ref {
                    known_client_ref.last_update_ms_since_epoch = now_ms as _;
                } else {
                    session_desc_ref.last_clients.push(ClientConnectionDesc {
                        trusted: false,
                        manually_added: false,
                        last_update_ms_since_epoch: now_ms as _,
                        ip: client_ip.to_string(),
                        device_name: None,
                    });

                    return;
                }
            }

            let control_socket_ref = &mut *control_socket.lock().await;
            if let Some(control_socket) = control_socket_ref {
                let send_command = control_socket.send(ServerControlPacket::Test);
                if let Ok(res) = timeout(TEST_CONNECTION_TIMEOUT, send_command).await {
                    if res.is_ok() {
                        return;
                    }
                }
            }

            // drop early to free the socket port
            *control_socket_ref = None;

            let settings = session_manager.lock().unwrap().get().to_settings();
            let maybe_control_socket = ControlSocket::connect_to_client(
                client_ip,
                |server_config: ServerConfigPacket, server_ip| {
                    let eye_width;
                    let eye_height;
                    match settings.video.render_resolution {
                        FrameSize::Scale(scale) => {
                            let (native_eye_width, native_eye_height) =
                                server_config.native_eye_resolution;
                            eye_width = native_eye_width as f32 * scale;
                            eye_height = native_eye_height as f32 * scale;
                        }
                        FrameSize::Absolute { width, height } => {
                            eye_width = width as f32 / 2_f32;
                            eye_height = height as f32 / 2_f32;
                        }
                    }
                    let eye_resolution = (align32(eye_width), align32(eye_height));

                    let web_gui_url = format!(
                        "http://{}:{}/",
                        server_ip, settings.connection.web_server_port
                    );

                    ClientConfigPacket {
                        settings,
                        eye_resolution,
                        web_gui_url,
                    }
                },
            )
            .await;

            if let Ok(control_socket) = show_err(maybe_control_socket) {
                *control_socket_ref = Some(control_socket);
            }
        }
    })
    .await;

    if let Err(e) = res {
        show_err::<(), _>(trace_str!("Error while listening for client: {}", e)).ok();
    }
}

async fn run(log_senders: Arc<Mutex<Vec<UnboundedSender<String>>>>) -> StrResult {
    // The lock on this mutex does not need to be held across await points, so I don't need a tokio
    // Mutex
    let session_manager = Arc::new(Mutex::new(SessionManager::new(&alvr_server_dir())));
    let control_socket = Arc::new(TMutex::new(None));

    tokio::spawn(client_discovery(session_manager.clone(), control_socket));

    let driver_log_redirect = tokio::spawn(
        tail_stream(&driver_log_path())?
            .map(|maybe_line: std::io::Result<String>| {
                if let Ok(line) = maybe_line {
                    if !(try_log_redirect(&line, log::Level::Error)
                        || try_log_redirect(&line, log::Level::Warn)
                        || try_log_redirect(&line, log::Level::Info)
                        || try_log_redirect(&line, log::Level::Debug)
                        || try_log_redirect(&line, log::Level::Trace))
                    {
                        try_log_redirect(&format!("[INFO] {}", line), log::Level::Info);
                    }
                }
            })
            .collect(),
    );

    let web_gui_dir = PathBuf::from(WEB_GUI_DIR_STR);
    let index_request = warp::path::end().and(wfs::file(web_gui_dir.join("index.html")));
    let files_requests = wfs::dir(web_gui_dir);

    let settings_schema_request = warp::path("settings-schema")
        .map(|| reply::json(&settings_schema(settings_cache_default())));

    let session_requests = warp::path("session").and(
        warp::get()
            .map({
                let session_manager = session_manager.clone();
                move || reply::json(session_manager.lock().unwrap().get())
            })
            .or(warp::path!(String / String).and(body::json()).map({
                let session_manager = session_manager.clone();
                move |update_type: String, update_author_id: String, value: serde_json::Value| {
                    if let Ok(update_type) = serde_json::from_str(&format!("\"{}\"", update_type)) {
                        let res = session_manager
                            .lock()
                            .unwrap()
                            .get_mut(&update_author_id, update_type)
                            .merge_from_json(value);
                        if let Err(e) = res {
                            warn!("{}", e);
                            // HTTP Code: WARNING
                            reply::with_status(reply(), StatusCode::from_u16(199).unwrap())
                        } else {
                            reply::with_status(reply(), StatusCode::OK)
                        }
                    } else {
                        reply::with_status(reply(), StatusCode::BAD_REQUEST)
                    }
                }
            })),
    );

    let log_subscription = warp::path("log").and(warp::ws()).map(move |ws: Ws| {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        log_senders.lock().unwrap().push(log_sender);
        ws.on_upgrade(|socket| subscribed_to_log(socket, log_receiver))
    });

    let driver_registration_requests =
        warp::path!("driver" / String).map(|action_string: String| {
            let res = show_err(match action_string.as_str() {
                "register" => alvr_xtask::driver_registration(&alvr_server_dir(), true),
                "unregister" => alvr_xtask::driver_registration(&alvr_server_dir(), false),
                "unregister-all" => alvr_xtask::unregister_all_drivers(),
                _ => return reply::with_status(reply(), StatusCode::BAD_REQUEST),
            });
            if res.is_ok() {
                reply::with_status(reply(), StatusCode::OK)
            } else {
                reply::with_status(reply(), StatusCode::INTERNAL_SERVER_ERROR)
            }
        });

    let firewall_rules_requests =
        warp::path!("firewall-rules" / String).map(|action_str: String| {
            let add = action_str == "add";
            let maybe_err = alvr_xtask::firewall_rules(&alvr_server_dir(), add).err();
            if let Some(e) = &maybe_err {
                error!("Setting firewall rules failed: code {}", e);
            }
            reply::json(&maybe_err.unwrap_or(0))
        });

    let audio_devices_request =
        warp::path("audio_devices").map(|| reply::json(&audio::output_audio_devices().ok()));

    let restart_steamvr_request = warp::path("restart_steamvr").map(move || {
        process::kill_steamvr();
        process::maybe_launch_steamvr();
        warp::reply()
    });

    let version_request = warp::path("version").map(|| ALVR_SERVER_VERSION);

    warp::serve(
        index_request
            .or(settings_schema_request)
            .or(session_requests)
            .or(log_subscription)
            .or(driver_registration_requests)
            .or(firewall_rules_requests)
            .or(audio_devices_request)
            .or(files_requests)
            .or(restart_steamvr_request)
            .or(version_request)
            .with(reply::with::header(
                "Cache-Control",
                "no-cache, no-store, must-revalidate",
            )),
    )
    .run((
        [0, 0, 0, 0],
        session_manager
            .lock()
            .unwrap()
            .get()
            .to_settings()
            .connection
            .web_server_port,
    ))
    .await;

    trace_err!(driver_log_redirect.await)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let mutex = single_instance::SingleInstance::new("alvr_web_server_mutex").unwrap();
    if mutex.is_single() {
        let log_senders = Arc::new(Mutex::new(vec![]));
        init_logging(log_senders.clone());

        show_err(run(log_senders).await).ok();
    }
}
