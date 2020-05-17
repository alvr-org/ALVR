mod logging_backend;
mod tail;

use alvr_common::{data::*, logging::*, *};
use futures::SinkExt;
use logging_backend::*;
use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tail::tail_stream;
use tokio::{
    stream::StreamExt,
    sync::mpsc::{self, *},
};
use warp::{
    body, fs as wfs, reply,
    ws::{Message, WebSocket, Ws},
    Filter, Rejection, Reply,
};

const TRACE_CONTEXT: &str = "Web server main";

const WEB_GUI_DIR_STR: &str = "web_gui";

fn try_log_redirect(line: &str, level: log::Level) -> bool {
    let level_label = &format!("[{}]", level);
    if line.starts_with(level_label) {
        let untagged_line = &line[level_label.len() + 1..];
        if level == log::Level::Error {
            show_err!(Err::<(), &str>(untagged_line)).ok();
        } else {
            log::log!(level, "{}", untagged_line);
        }

        true
    } else {
        false
    }
}

async fn handle_session_not_found(_: Rejection) -> Result<impl Reply, Infallible> {
    Ok(reply::json(&SessionDesc::default()))
}

fn update_settings_and_session(session_desc: SessionDesc) -> StrResult {
    save_json(&session_desc, &Path::new(SESSION_FNAME))?;
    save_json(
        &session_to_settings(&session_desc),
        &Path::new(SETTINGS_FNAME),
    )
}

async fn subscribed_to_log(mut socket: WebSocket, mut log_receiver: UnboundedReceiver<String>) {
    while let Some(line) = log_receiver.next().await {
        if let Err(e) = socket.send(Message::text(line)).await {
            log::info!("Failed to send log with websocket: {}", e);
            break;
        }
    }
}

async fn run(log_senders: Arc<Mutex<Vec<UnboundedSender<String>>>>) -> StrResult {
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
            .map(|data| {
                show_err!(update_settings_and_session(data)).ok();
                warp::reply()
            })
            .or(wfs::file(SESSION_FNAME).recover(handle_session_not_found)),
    );

    let log_subscription = warp::path("log").and(warp::ws()).map(move |ws: Ws| {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        log_senders.lock().unwrap().push(log_sender);
        ws.on_upgrade(|socket| subscribed_to_log(socket, log_receiver))
    });

    warp::serve(
        index_request
            .or(settings_schema_request)
            .or(session_requests)
            .or(log_subscription)
            .or(files_requests),
    )
    .run(([127, 0, 0, 1], 8080))
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

        if let Err(e) = run(log_senders).await {
            log::error!("{}", e);
        }
    }
}
