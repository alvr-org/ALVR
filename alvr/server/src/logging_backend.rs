use crate::{FILESYSTEM_LAYOUT, SERVER_DATA_MANAGER};
use alvr_common::log::{self, LevelFilter};
use alvr_events::{Event, EventSeverity, EventType, LogEvent};
use chrono::Local;
use fern::Dispatch;
use std::fs;
use tokio::sync::broadcast::Sender;

// todo: don't stringify events immediately, use Sender<Event>
pub fn init_logging(
    log_sender: Sender<String>,
    legacy_events_sender: Sender<String>,
    events_sender: Sender<String>,
) {
    let mut log_dispatch = Dispatch::new().format(move |out, message, record| {
        let maybe_event = format!("{message}");
        if maybe_event.starts_with('{') {
            legacy_events_sender.send(maybe_event.clone()).ok();
        } else {
            let severity = match record.level() {
                log::Level::Error => EventSeverity::Error,
                log::Level::Warn => EventSeverity::Warning,
                log::Level::Info => EventSeverity::Info,
                log::Level::Debug | log::Level::Trace => EventSeverity::Debug,
            };

            let event = EventType::Log(LogEvent {
                severity,
                content: message.to_string(),
            });

            legacy_events_sender
                .send(serde_json::to_string(&event).unwrap())
                .ok();
        }
        let log_message = if maybe_event.starts_with('{') {
            format!("#{}#", maybe_event)
        } else {
            maybe_event.clone()
        };
        log_sender
            .send(format!(
                "{} [{}] {log_message}",
                chrono::Local::now().format("%H:%M:%S.%f"),
                record.level()
            ))
            .ok();

        let event_type = if maybe_event.starts_with('{') {
            serde_json::from_str(&maybe_event).unwrap()
        } else {
            let severity = match record.level() {
                log::Level::Error => EventSeverity::Error,
                log::Level::Warn => EventSeverity::Warning,
                log::Level::Info => EventSeverity::Info,
                log::Level::Debug | log::Level::Trace => EventSeverity::Debug,
            };

            EventType::Log(LogEvent {
                severity,
                content: message.to_string(),
            })
        };
        let event = Event {
            timestamp: Local::now().format("%H:%M:%S.%f").to_string(),
            event_type,
        };
        out.finish(format_args!("{}", serde_json::to_string(&event).unwrap()));
        // todo: don't stringify event
        events_sender
            .send(serde_json::to_string(&event).unwrap())
            .ok();
    });

    if cfg!(debug_assertions) {
        log_dispatch = log_dispatch.level(LevelFilter::Debug)
    } else {
        log_dispatch = log_dispatch.level(LevelFilter::Info);
    }

    if SERVER_DATA_MANAGER.read().settings().extra.log_to_disk {
        log_dispatch = log_dispatch.chain(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(FILESYSTEM_LAYOUT.session_log())
                .unwrap(),
        );
    } else {
        // this sink is required to make sure all log gets processed and forwarded to the websocket
        if cfg!(target_os = "linux") {
            log_dispatch = log_dispatch.chain(
                fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/null")
                    .unwrap(),
            );
        } else {
            log_dispatch = log_dispatch.chain(std::io::stdout());
        }
    }

    log_dispatch
        .chain(
            Dispatch::new()
                .level(LevelFilter::Error)
                .chain(fern::log_file(FILESYSTEM_LAYOUT.crash_log()).unwrap()),
        )
        .apply()
        .unwrap();

    alvr_common::set_panic_hook();
}
