mod logging_backend;
mod sockets;
mod tail;

use alvr_common::{data::*, logging::*, *};
use futures::SinkExt;
use logging_backend::*;
use std::{
    path::{Path, PathBuf},
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
    reply,
    ws::{Message, WebSocket, Ws},
    Filter,
};

const WEB_GUI_DIR_STR: &str = "web_gui";

fn alvr_server_dir() -> PathBuf {
    std::env::current_exe().unwrap().parent().unwrap().to_owned()
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
    loop {
        if let Ok((address, handshake_packet)) = sockets::search_client(None).await {
            let now_ms = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let session_manager_ref = &mut session_manager.lock().unwrap();
            let session_desc_ref = &mut session_manager_ref.get_mut();

            let maybe_known_client_ref =
                session_desc_ref
                    .last_clients
                    .iter_mut()
                    .find(|connection_desc| {
                        connection_desc.address == address.to_string()
                            && connection_desc.handshake_packet == handshake_packet
                    });

            if let Some(known_client_ref) = maybe_known_client_ref {
                known_client_ref.available = true;
                known_client_ref.last_update_ms_since_epoch = now_ms as _;
            } else {
                session_desc_ref.last_clients.push(ClientConnectionDesc {
                    available: true,
                    connect_automatically: false,
                    last_update_ms_since_epoch: now_ms as _,
                    address: address.to_string(),
                    handshake_packet,
                })
            }
        }
    }
}

async fn run(log_senders: Arc<Mutex<Vec<UnboundedSender<String>>>>) -> StrResult {
    let session_manager = Arc::new(Mutex::new(SessionManager::new(&Path::new("./"))));

    tokio::spawn(client_discovery(session_manager.clone()));

    let driver_log_redirect = tokio::spawn(
        tail_stream(DRIVER_LOG_FNAME)?
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
        body::json()
            .map({
                let session_manager = session_manager.clone();
                move |data| {
                    *session_manager.lock().unwrap().get_mut() = data;
                    warp::reply()
                }
            })
            .or(warp::any().map(move || reply::json(&*session_manager.lock().unwrap().get_mut()))),
    );

    let log_subscription = warp::path("log").and(warp::ws()).map(move |ws: Ws| {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        log_senders.lock().unwrap().push(log_sender);
        ws.on_upgrade(|socket| subscribed_to_log(socket, log_receiver))
    });

    let driver_registration_requests = warp::path!("driver" / String).map(|action_str: String| {
        let register = action_str == "register";
        show_err(alvr_xtask::driver_registration(&alvr_server_dir(), register)).ok();
        warp::reply()
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

    warp::serve(
        index_request
            .or(settings_schema_request)
            .or(session_requests)
            .or(log_subscription)
            .or(driver_registration_requests)
            .or(firewall_rules_requests)
            .or(files_requests)
            .with(reply::with::header(
                "Cache-Control",
                "no-cache, no-store, must-revalidate",
            )),
    )
    .run(([127, 0, 0, 1], 8082))
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
