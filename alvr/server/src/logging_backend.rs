use crate::{FILESYSTEM_LAYOUT, SESSION_MANAGER};
use alvr_common::{Event, EventSeverity, Raw};
use fern::Dispatch;
use log::LevelFilter;
use std::fs;
use tokio::sync::broadcast::Sender;

pub fn init_logging(log_sender: Sender<String>, events_sender: Sender<String>) {
    let mut log_dispatch = Dispatch::new().format(move |out, message, record| {
        let maybe_event = format!("{}", message);
        if maybe_event.contains("#{") {
            let event_data = maybe_event.replace("#{", "{").replace("}#", "}");
            events_sender.send(event_data).ok();
        } else {
            let severity = match record.level() {
                log::Level::Error => EventSeverity::Error,
                log::Level::Warn => EventSeverity::Warning,
                log::Level::Info => EventSeverity::Info,
                log::Level::Debug | log::Level::Trace => EventSeverity::Debug,
            };

            let event = Event::Raw(Raw {
                timestamp: chrono::Local::now().format("%H:%M:%S.%f").to_string(),
                severity,
                content: message.to_string(),
            });

            events_sender
                .send(serde_json::to_string(&event).unwrap())
                .ok();
        }
        let log_line = format!(
            "{} [{}] {}",
            chrono::Local::now().format("%H:%M:%S.%f"),
            record.level(),
            message
        );
        log_sender.send(log_line.clone()).ok();
        out.finish(format_args!("{}", log_line));
    });

    if cfg!(debug_assertions) {
        log_dispatch = log_dispatch.level(LevelFilter::Debug)
    } else {
        log_dispatch = log_dispatch.level(LevelFilter::Info);
    }

    if SESSION_MANAGER.lock().get().to_settings().extra.log_to_disk {
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
        log_dispatch = log_dispatch.chain(std::io::stdout());
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
