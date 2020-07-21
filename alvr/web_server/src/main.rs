mod logging_backend;
mod sockets;
mod tail;

use alvr_common::{data::*, logging::*, *};
use futures::SinkExt;
use logging_backend::*;
use settings_schema::Switch;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::SystemTime,
};
use tail::tail_stream;
use tokio::{
    stream::StreamExt,
    sync::mpsc::{self, *},
};
use warp::{
    body, fs as wfs,
    http::StatusCode,
    reply,
    ws::{Message, WebSocket, Ws},
    Filter,
};

const WEB_GUI_DIR_STR: &str = "web_gui";
const WEB_SERVER_PORT: u16 = 8082;

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

async fn client_discovery(session_manager: Arc<Mutex<SessionManager>>) {
    let res = sockets::search_client(None, |address, client_handshake_packet| {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        {
            let session_manager_ref = &mut session_manager.lock().unwrap();
            let session_desc_ref = &mut session_manager_ref
                .get_mut(SERVER_SESSION_UPDATE_ID, SessionUpdateType::ClientList);

            let maybe_known_client_ref =
                session_desc_ref
                    .last_clients
                    .iter_mut()
                    .find(|connection_desc| {
                        connection_desc.address == address.to_string()
                            && connection_desc.handshake_packet == client_handshake_packet
                    });

            if let Some(known_client_ref) = maybe_known_client_ref {
                known_client_ref.last_update_ms_since_epoch = now_ms as _;

                if matches!(
                    known_client_ref.state,
                    ClientConnectionState::AvailableUntrusted
                ) {
                    return None;
                } else {
                    known_client_ref.state = ClientConnectionState::AvailableTrusted;
                }
            } else {
                session_desc_ref.last_clients.push(ClientConnectionDesc {
                    state: ClientConnectionState::AvailableUntrusted,
                    last_update_ms_since_epoch: now_ms as _,
                    address: address.to_string(),
                    handshake_packet: client_handshake_packet,
                });

                return None;
            }
        }

        let settings = session_manager.lock().unwrap().get().to_settings();

        let video_width;
        let video_height;
        match settings.video.render_resolution {
            FrameSize::Scale(scale) => {
                video_width = align32(client_handshake_packet.render_width as f32 * scale);
                video_height = align32(client_handshake_packet.render_height as f32 * scale);
            }
            FrameSize::Absolute { width, height } => {
                video_width = width;
                video_height = height;
            }
        }

        let foveation_mode;
        let foveation_strength;
        let foveation_shape;
        let foveation_vertical_offset;
        if let Switch::Enabled(foveation_data) = settings.video.foveated_rendering {
            foveation_mode = true as u8;
            foveation_strength = foveation_data.strength;
            foveation_shape = foveation_data.shape;
            foveation_vertical_offset = foveation_data.vertical_offset;
        } else {
            foveation_mode = false as u8;
            foveation_strength = 0.;
            foveation_shape = 0.;
            foveation_vertical_offset = 0.;
        }

        let mut server_handshake_packet = ServerHandshakePacket {
            packet_type: 2,
            codec: settings.video.codec as _,
            video_width,
            video_height,
            buffer_size_bytes: settings.connection.client_recv_buffer_size as _,
            frame_queue_size: settings.connection.frame_queue_size as _,
            refresh_rate: settings.video.refresh_rate as _,
            stream_mic: settings.audio.microphone,
            foveation_mode,
            foveation_strength,
            foveation_shape,
            foveation_vertical_offset,
            web_gui_url: [0; 32],
        };

        // show_err::<(), _>(trace_str!("{:#?}", server_handshake_packet));

        let mut maybe_host_address = None;

        // todo: get the host address using another handshake round instead
        for adapter in ipconfig::get_adapters().expect("PC network adapters") {
            for host_address in adapter.ip_addresses() {
                let address_string = host_address.to_string();
                if address_string.starts_with("192.168") {
                    maybe_host_address = Some(*host_address);
                }
            }
        }
        if let Some(host_address) = maybe_host_address {
            server_handshake_packet.web_gui_url = [0; 32];
            let url_string = format!("http://{}:{}/", host_address, WEB_SERVER_PORT);
            let url_c_string = std::ffi::CString::new(url_string).unwrap();
            let url_bytes = url_c_string.as_bytes_with_nul();
            server_handshake_packet.web_gui_url[0..url_bytes.len()].copy_from_slice(url_bytes);

            process::maybe_launch_steamvr();

            Some(server_handshake_packet)
        } else {
            None
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

    tokio::spawn(client_discovery(session_manager.clone()));

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

    let settings_schema_request = warp::path("settings-schema").map(|| env!("SETTINGS_SCHEMA"));

    let session_requests = warp::path("session").and(
        warp::get()
            .map({
                let session_manager = session_manager.clone();
                move || reply::json(session_manager.lock().unwrap().get())
            })
            .or(warp::path!(String).and(body::json()).map({
                let session_manager = session_manager.clone();
                move |meta_str: String, value: serde_json::Value| {
                    let meta_list = meta_str.split('?').collect::<Vec<_>>();
                    if meta_list.len() == 2 {
                        let update_author_id = meta_list[0];
                        if let Ok(update_type) = serde_json::from_str(meta_list[1]) {
                            let res = session_manager
                                .lock()
                                .unwrap()
                                .get_mut(update_author_id, update_type)
                                .merge_from_json(value);
                            if let Err(e) = res {
                                warn!("{}", e);
                                // HTTP Code: WARNING
                                return reply::with_status(
                                    reply(),
                                    StatusCode::from_u16(199).unwrap(),
                                );
                            } else {
                                return reply::with_status(reply(), StatusCode::OK);
                            }
                        }
                    }
                    reply::with_status(reply(), StatusCode::BAD_REQUEST)
                }
            }))
            // stopgap. todo: remove
            .or(warp::post().and(body::json().map(move |value| {
                let res = session_manager
                    .lock()
                    .unwrap()
                    .get_mut("", SessionUpdateType::Settings)
                    .merge_from_json(value);
                if let Err(e) = res {
                    warn!("{}", e);
                    // HTTP Code: WARNING
                    reply::with_status(reply(), StatusCode::from_u16(199).unwrap())
                } else {
                    reply::with_status(reply(), StatusCode::OK)
                }
            }))),
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
    .run(([0, 0, 0, 0], WEB_SERVER_PORT))
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
