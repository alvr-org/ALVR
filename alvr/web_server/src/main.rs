mod logging_backend;
mod tail;

use alvr_common::{data::*, logging::*, *};
use logging_backend::*;
use std::{convert::Infallible, path::PathBuf};
use tail::tail_stream;
use tokio::stream::StreamExt;
use warp::{body, fs as wfs, reply, sse, Filter, Rejection, Reply};

const TRACE_CONTEXT: &str = "Web server main";

const WEB_GUI_DIR_STR: &str = "web_gui";

fn try_log_redirect(line: &str, level: log::Level) -> bool {
    let level_label = &format!("[{}]", level);
    if line.starts_with(level_label) {
        let line_without_label = &line[level_label.len() + 1..];

        if level == log::Level::Error {
            show_err!(Err::<(), &str>(line_without_label)).ok();
        } else {
            log::log!(level, "{}", &line[level_label.len() + 1..]);
        }

        true
    } else {
        false
    }
}

async fn handle_session_not_found(_: Rejection) -> Result<impl Reply, Infallible> {
    Ok(reply::json(&SessionDesc::default()))
}

fn update_settings(session_desc: SessionDesc) {
    // todo
}

async fn run() -> StrResult {
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

    let session_requests = warp::path("session").and(
        body::json()
            .map(|data| {
                update_settings(data);
                warp::reply()
            })
            .or(wfs::file(SESSION_FNAME).recover(handle_session_not_found)),
    );

    // todo: find a way to clone the tail stream to avoid creating multiple log reader processes.
    //       This would also remove the need for unwrap() for failing without crashing.
    let log_subscription = warp::path("log").map(|| {
        sse::reply(
            sse::keep_alive().stream(
                show_err!(tail_stream(SESSION_LOG_FNAME))
                    .unwrap()
                    .map(|maybe_line| maybe_line.map(sse::data)),
            ),
        )
    });

    warp::serve(
        index_request
            .or(session_requests)
            .or(log_subscription)
            .or(files_requests),
    )
    .run(([127, 0, 0, 1], 80))
    .await;

    trace_err!(driver_log_redirect.await)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    init_logging();
    show_err!(run().await).ok();
}
