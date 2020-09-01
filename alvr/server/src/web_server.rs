use crate::*;
use alvr_common::{commands::*, data::*, logging::*, *};
use futures::{future::BoxFuture, SinkExt};
use std::{
    convert::Infallible,
    net::IpAddr,
    path::PathBuf,
    sync::{atomic::*, Arc},
};
use tokio::{
    stream::StreamExt,
    sync::broadcast::{self, RecvError},
};
use warp::{
    body, fs as wfs,
    http::{HeaderValue, StatusCode},
    hyper::{header::*, HeaderMap},
    reply,
    ws::{Message, WebSocket, Ws},
    Filter, Reply,
};

pub const LOG_BROADCAST_CAPACITY: usize = 256;
const WEB_GUI_DIR_STR: &str = "web_gui";

async fn subscribed_to_log(mut socket: WebSocket, mut log_receiver: broadcast::Receiver<String>) {
    while let Some(maybe_line) = log_receiver.next().await {
        match maybe_line {
            Ok(line) => {
                if let Err(e) = socket.send(Message::text(line)).await {
                    info!("Failed to send log with websocket: {}", e);
                    break;
                }
            }
            Err(RecvError::Lagged(_)) => {
                warn!("Some log lines have been lost because the buffer is full");
            }
            Err(RecvError::Closed) => break,
        }
    }
}

async fn set_session_handler(
    session_manager: Arc<AMutex<SessionManager>>,
    update_type: String,
    update_author_id: String,
    value: serde_json::Value,
) -> Result<impl Reply, Infallible> {
    if let Ok(update_type) = serde_json::from_str(&format!("\"{}\"", update_type)) {
        let res = session_manager
            .lock()
            .await
            .get_mut(&update_author_id, update_type)
            .merge_from_json(value);

        if let Err(e) = res {
            warn!("{}", e);
            // HTTP Code: WARNING
            Ok(reply::with_status(
                reply(),
                StatusCode::from_u16(199).unwrap(),
            ))
        } else {
            Ok(reply::with_status(reply(), StatusCode::OK))
        }
    } else {
        Ok(reply::with_status(reply(), StatusCode::BAD_REQUEST))
    }
}

pub async fn web_server(
    session_manager: Arc<AMutex<SessionManager>>,
    log_sender: broadcast::Sender<String>,
    update_client_listeners_notifier: broadcast::Sender<()>,
) -> StrResult {
    let settings_changed = Arc::new(AtomicBool::new(false));

    let web_gui_dir = ALVR_DIR.join(WEB_GUI_DIR_STR);
    let index_request = warp::path::end().and(wfs::file(web_gui_dir.join("index.html")));
    let files_requests = wfs::dir(web_gui_dir);

    let settings_schema_request = warp::path("settings-schema")
        .map(|| reply::json(&settings_schema(session_settings_default())));

    let get_session_request = warp::get().and(warp::path("session")).and_then({
        let session_manager = session_manager.clone();
        move || {
            let session_manager = session_manager.clone();
            async move { Ok::<_, Infallible>(reply::json(session_manager.lock().await.get())) }
        }
    });
    let post_session_request = warp::post()
        .and(warp::path!("session" / String / String))
        .and(body::json())
        .and_then({
            let session_manager = session_manager.clone();
            move |update_type: String, update_author_id: String, value: serde_json::Value| {
                settings_changed.store(true, Ordering::Relaxed);
                set_session_handler(
                    session_manager.clone(),
                    update_type,
                    update_author_id,
                    value,
                )
            }
        });

    let log_subscription = warp::path("log").and(warp::ws()).map(move |ws: Ws| {
        let log_receiver = log_sender.subscribe();
        ws.on_upgrade(|socket| subscribed_to_log(socket, log_receiver))
    });

    let register_driver_request = warp::path!("driver" / "register").map(|| {
        if driver_registration(&[ALVR_DIR.clone()], true).is_ok() {
            reply::with_status(reply(), StatusCode::OK)
        } else {
            reply::with_status(reply(), StatusCode::INTERNAL_SERVER_ERROR)
        }
    });
    let unregister_driver_request =
        warp::path!("driver" / "unregister")
            .and(body::json())
            .map(|path: PathBuf| {
                if driver_registration(&[path], false).is_ok() {
                    reply::with_status(reply(), StatusCode::OK)
                } else {
                    reply::with_status(reply(), StatusCode::INTERNAL_SERVER_ERROR)
                }
            });
    let list_drivers_request = warp::path!("driver" / "list").map(|| {
        if let Ok(list) = get_registered_drivers() {
            reply::json(&list)
        } else {
            reply::json(&Vec::<PathBuf>::new())
        }
    });

    let firewall_rules_requests =
        warp::path!("firewall-rules" / String).map(|action_str: String| {
            let add = action_str == "add";
            let maybe_err = firewall_rules(add).err();
            if let Some(e) = &maybe_err {
                error!("Setting firewall rules failed: code {}", e);
            }
            reply::json(&maybe_err.unwrap_or(0))
        });

    let audio_devices_request =
        warp::path("audio-devices").map(|| reply::json(&audio::output_audio_devices().ok()));

    let restart_steamvr_request = warp::path("restart_steamvr").map(move || {
        restart_steamvr();
        warp::reply()
    });

    let client_list_action_request = warp::path!("clients" / String).and(body::json()).and_then({
        let session_manager = session_manager.clone();
        let update_client_listeners_notifier = update_client_listeners_notifier.clone();
        move |action: String, (hostname, maybe_ip): (String, Option<IpAddr>)| {
            let session_manager = session_manager.clone();
            let update_client_listeners_notifier = update_client_listeners_notifier.clone();
            async move {
                let action = match action.as_str() {
                    "trust" => ClientListAction::TrustAndMaybeAddIp(maybe_ip),
                    "remove" => ClientListAction::RemoveIpOrEntry(maybe_ip),
                    _ => {
                        return Ok::<_, Infallible>(reply::with_status(
                            reply(),
                            StatusCode::BAD_REQUEST,
                        ))
                    }
                };
                update_client_list(
                    session_manager,
                    hostname,
                    action,
                    update_client_listeners_notifier,
                )
                .await;

                Ok::<_, Infallible>(reply::with_status(reply(), StatusCode::OK))
            }
        }
    });

    let version_request = warp::path("version").map(|| ALVR_SERVER_VERSION.to_string());

    let graphics_devices_request =
        warp::path("graphics-devices").map(|| reply::json(&graphics::get_gpus_info()));

    let web_server_port = session_manager
        .lock()
        .await
        .get()
        .to_settings()
        .connection
        .web_server_port;

    let mut headers = HeaderMap::new();
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );

    // BoxFuture is needed to avoid error: "reached the type-length limit while instantiating ..."
    // todo: switch to Rocket
    let web_server: BoxFuture<()> = Box::pin(
        warp::serve(
            version_request
                .or(index_request)
                .or(files_requests)
                .or(settings_schema_request)
                .or(audio_devices_request)
                .or(graphics_devices_request)
                .or(restart_steamvr_request)
                .or(log_subscription)
                .or(register_driver_request)
                .or(unregister_driver_request)
                .or(list_drivers_request)
                .or(firewall_rules_requests)
                .or(get_session_request)
                .or(post_session_request)
                .or(client_list_action_request)
                .with(reply::with::headers(headers)),
        )
        .run(([0, 0, 0, 0], web_server_port)),
    );

    web_server.await;

    trace_str!("Web server closed unexpectedly")
}
