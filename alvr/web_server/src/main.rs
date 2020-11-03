mod logging_backend;
mod sockets;
mod tail;

use alvr_common::{commands::*, data::*, logging::*, *};
use bytes::buf::BufExt;
use futures::{stream::StreamExt, SinkExt};
use headers::{self, HeaderMapExt};
use hyper::{
    header,
    header::HeaderValue,
    header::CACHE_CONTROL,
    http::request::Parts,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, StatusCode,
};
use logging_backend::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json as json;
use settings_schema::Switch;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::SystemTime,
};
use tail::tail_stream;
use tokio::sync::mpsc::{self, *};
use tokio_tungstenite::{tungstenite::protocol, WebSocketStream};
use tokio_util::codec::{BytesCodec, FramedRead};

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

async fn client_discovery(session_manager: Arc<Mutex<SessionManager>>) {
    let res = sockets::search_client(None, |address, client_handshake_packet| {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        {
            let session_manager_ref = &mut session_manager.lock().unwrap();
            let session_desc_ref =
                &mut session_manager_ref.get_mut(None, SessionUpdateType::ClientList);

            let maybe_known_client_ref =
                session_desc_ref
                    .last_clients
                    .iter_mut()
                    .find(|connection_desc| {
                        connection_desc.address == address.to_string()
                            && connection_desc.handshake_packet.device_name
                                == client_handshake_packet.device_name
                            && connection_desc.handshake_packet.version
                                == client_handshake_packet.version
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

        // patch for Oculus Quest 2
        {
            let session_manager_ref = &mut session_manager.lock().unwrap();
            let session_desc_ref =
                &mut session_manager_ref.get_mut(None, SessionUpdateType::Settings);

            session_desc_ref.session_settings.video.refresh_rate =
                client_handshake_packet.client_refresh_rate as _;
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
            stream_mic: matches!(settings.audio.microphone, Switch::Enabled(_)),
            foveation_mode,
            foveation_strength,
            foveation_shape,
            foveation_vertical_offset,
            web_gui_url: [0; 32],
        };

        let mut maybe_host_address = None;

        // todo: get the host address using another handshake round instead
        for adapter in ipconfig::get_adapters().expect("PC network adapters") {
            for host_address in adapter.ip_addresses() {
                let address_string = host_address.to_string();
                if address_string.starts_with("192.168.")
                    || address_string.starts_with("10.")
                    || address_string.starts_with("172.")
                {
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

            maybe_launch_steamvr();

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

fn reply(code: StatusCode) -> StrResult<Response<Body>> {
    trace_err!(Response::builder().status(code).body(Body::empty()))
}

fn reply_json<T: Serialize>(obj: &T) -> StrResult<Response<Body>> {
    trace_err!(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(trace_err!(json::to_string(obj))?.into()))
}

async fn from_body<T: DeserializeOwned>(body: Body) -> StrResult<T> {
    trace_err!(json::from_reader(
        trace_err!(hyper::body::aggregate(body).await)?.reader()
    ))
}

async fn http_api(
    request: Request<Body>,
    session_manager: Arc<Mutex<SessionManager>>,
    log_senders: Arc<Mutex<Vec<UnboundedSender<String>>>>,
) -> StrResult<Response<Body>> {
    let (
        Parts {
            uri,
            method,
            headers,
            ..
        },
        body,
    ) = request.into_parts();
    let uri = uri.path();

    let mut response = match uri {
        "/settings-schema" => reply_json(&settings_schema(session_settings_default()))?,
        "/session" => {
            if matches!(method, Method::GET) {
                reply_json(session_manager.lock().unwrap().get())?
            } else if let Ok(data) = from_body::<json::Value>(body).await {
                // POST
                if let (Some(update_type), Some(update_author_id), Some(value)) = (
                    data.get("updateType"),
                    data.get("webClientId"),
                    data.get("session"),
                ) {
                    if let (Ok(update_type), Ok(update_author_id)) = (
                        json::from_value(update_type.clone()),
                        json::from_value::<String>(update_author_id.clone()),
                    ) {
                        let res = session_manager
                            .lock()
                            .unwrap()
                            .get_mut(Some(update_author_id), update_type)
                            .merge_from_json(value);
                        if let Err(e) = res {
                            warn!("{}", e);
                            // HTTP Code: WARNING
                            reply(trace_err!(StatusCode::from_u16(199))?)?
                        } else {
                            reply(StatusCode::OK)?
                        }
                    } else {
                        reply(StatusCode::BAD_REQUEST)?
                    }
                } else {
                    reply(StatusCode::BAD_REQUEST)?
                }
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/log" => {
            if let Some(key) = headers.typed_get::<headers::SecWebsocketKey>() {
                tokio::spawn(async move {
                    match body.on_upgrade().await {
                        Ok(upgraded) => {
                            let (log_sender, mut log_receiver) = mpsc::unbounded_channel();
                            log_senders.lock().unwrap().push(log_sender);

                            let mut ws = WebSocketStream::from_raw_socket(
                                upgraded,
                                protocol::Role::Server,
                                None,
                            )
                            .await;

                            while let Some(line) = log_receiver.next().await {
                                if let Err(e) = ws.send(protocol::Message::text(line)).await {
                                    info!("Failed to send log with websocket: {}", e);
                                    break;
                                }
                            }

                            ws.close(None).await.ok();
                        }
                        Err(e) => error!("{}", e),
                    }
                });

                let mut response = trace_err!(Response::builder()
                    .status(StatusCode::SWITCHING_PROTOCOLS)
                    .body(Body::empty()))?;

                let h = response.headers_mut();
                h.typed_insert(headers::Upgrade::websocket());
                h.typed_insert(headers::SecWebsocketAccept::from(key));
                h.typed_insert(headers::Connection::upgrade());

                response
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/driver/register" => {
            if driver_registration(&alvr_server_dir(), true).is_ok() {
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::INTERNAL_SERVER_ERROR)?
            }
        }
        "/driver/unregister" => {
            if let Ok(path) = from_body::<PathBuf>(body).await {
                if driver_registration(&path, false).is_ok() {
                    reply(StatusCode::OK)?
                } else {
                    reply(StatusCode::INTERNAL_SERVER_ERROR)?
                }
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/driver/list" => reply_json(&get_registered_drivers().unwrap_or_default())?,
        uri @ "/firewall-rules/add" | uri @ "/firewall-rules/remove" => {
            let add = uri.ends_with("add");
            let maybe_err = firewall_rules(&alvr_server_dir(), add).err();
            if let Some(e) = &maybe_err {
                error!("Setting firewall rules failed: code {}", e);
            }
            reply_json(&maybe_err.unwrap_or(0))?
        }
        "/graphics-devices" => reply_json(&graphics::get_gpu_names())?,
        "/audio-devices" => reply_json(&audio::output_audio_devices().ok())?,
        "/restart-steamvr" => {
            kill_steamvr();
            maybe_launch_steamvr();
            reply(StatusCode::OK)?
        }
        "/version" => Response::new(ALVR_SERVER_VERSION.to_string().into()),
        "/open" => {
            if let Ok(url) = from_body::<String>(body).await {
                webbrowser::open(&url).ok();
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        other_uri => {
            if other_uri.contains("..") {
                // Attempted tree traversal
                reply(StatusCode::FORBIDDEN)?
            } else {
                let path_branch = match other_uri {
                    "/" => "/index.html",
                    other_path => other_path,
                };

                if let Ok(file) =
                    tokio::fs::File::open(format!("{}{}", WEB_GUI_DIR_STR, path_branch)).await
                {
                    Response::new(Body::wrap_stream(FramedRead::new(file, BytesCodec::new())))
                } else {
                    reply(StatusCode::NOT_FOUND)?
                }
            }
        }
    };

    response.headers_mut().insert(
        CACHE_CONTROL,
        trace_err!(HeaderValue::from_str("no-cache, no-store, must-revalidate"))?,
    );

    Ok(response)
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

    let addr = "0.0.0.0:8082".parse().unwrap();

    let service = make_service_fn(|_| {
        let session_manager = session_manager.clone();
        let log_senders = log_senders.clone();
        async move {
            StrResult::Ok(service_fn(move |request| {
                http_api(request, session_manager.clone(), log_senders.clone())
            }))
        }
    });
    trace_err!(hyper::Server::bind(&addr).serve(service).await)?;

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
