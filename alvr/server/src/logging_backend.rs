use crate::{ALVR_DIR, SESSION_MANAGER};
use alvr_common::{logging::*, *};
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
                .open(ALVR_DIR.join(SESSION_LOG_FNAME))
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
                .chain(fern::log_file(ALVR_DIR.join(CRASH_LOG_FNAME)).unwrap()),
        )
        .apply()
        .unwrap();

    logging::set_panic_hook();
}
