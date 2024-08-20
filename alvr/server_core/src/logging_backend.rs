use crate::SESSION_MANAGER;
use alvr_common::{log::LevelFilter, once_cell::sync::Lazy, LogEntry, LogSeverity};
use alvr_events::{Event, EventType};
use chrono::Local;
use fern::Dispatch;
use std::{fs, path::PathBuf};
use tokio::sync::broadcast;

static CHANNEL_CAPACITY: usize = 256;
pub static LOGGING_EVENTS_SENDER: Lazy<broadcast::Sender<Event>> =
    Lazy::new(|| broadcast::channel(CHANNEL_CAPACITY).0);

pub fn init_logging(session_log_path: Option<PathBuf>, crash_log_path: Option<PathBuf>) {
    let debug_groups_config = SESSION_MANAGER
        .read()
        .settings()
        .extra
        .logging
        .debug_groups
        .clone();

    let mut log_dispatch = Dispatch::new()
        // Note: meta::target() is in the format <crate>::<module>
        .filter(|meta| !meta.target().starts_with("mdns_sd"))
        .format(move |out, message, record| {
            let maybe_event = format!("{message}");
            let event_type = if maybe_event.starts_with('{') && maybe_event.ends_with('}') {
                serde_json::from_str(&maybe_event).unwrap()
            } else {
                let log_level = record.level();
                if log_level <= LevelFilter::Info
                    || alvr_common::filter_debug_groups(&maybe_event, &debug_groups_config)
                {
                    EventType::Log(LogEntry {
                        severity: LogSeverity::from_log_level(record.level()),
                        content: message.to_string(),
                    })
                } else {
                    return;
                }
            };
            let event = Event {
                timestamp: Local::now().format("%H:%M:%S.%f").to_string(),
                event_type,
            };
            out.finish(format_args!("{}", serde_json::to_string(&event).unwrap()));

            LOGGING_EVENTS_SENDER.send(event).ok();
        });

    if cfg!(debug_assertions) {
        log_dispatch = log_dispatch.level(LevelFilter::Debug)
    } else {
        log_dispatch = log_dispatch.level(LevelFilter::Info);
    }

    log_dispatch = if let Some(path) = session_log_path {
        log_dispatch.chain(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .unwrap(),
        )
    } else if cfg!(target_os = "linux") {
        // this sink is required to make sure all log gets processed and forwarded to the websocket
        log_dispatch.chain(
            fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .unwrap(),
        )
    } else {
        log_dispatch.chain(std::io::stdout())
    };

    log_dispatch = if let Some(path) = crash_log_path {
        log_dispatch.chain(
            Dispatch::new()
                .level(LevelFilter::Error)
                .chain(fern::log_file(path).unwrap()),
        )
    } else if cfg!(target_os = "linux") {
        log_dispatch.chain(
            fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .unwrap(),
        )
    } else {
        log_dispatch.chain(std::io::stderr())
    };

    log_dispatch.apply().unwrap();

    alvr_common::set_panic_hook();
}
