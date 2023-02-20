use crate::data_sources::ServerEvent;
use alvr_common::{log::LevelFilter, parking_lot::Mutex, LogSeverity};
use alvr_events::{Event, EventType, LogEvent};
use std::{
    io::Write,
    sync::{mpsc, Arc},
};

pub fn init_logging(event_sender: mpsc::Sender<ServerEvent>) {
    let event_sender = Arc::new(Mutex::new(event_sender));

    env_logger::Builder::new()
        .filter(Some("alvr_events"), LevelFilter::Off)
        .filter(Some("naga"), LevelFilter::Off)
        .filter(Some("ureq"), LevelFilter::Off)
        .filter(Some("wgpu_core"), LevelFilter::Off)
        .filter(Some("wgpu_hal"), LevelFilter::Off)
        .filter_level(if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .format(move |f, record| {
            let timestamp = chrono::Local::now().format("%H:%M:%S.%f").to_string();

            event_sender
                .lock()
                .send(ServerEvent::Event(Event {
                    timestamp: timestamp.clone(),
                    event_type: EventType::Log(LogEvent {
                        severity: LogSeverity::from_log_level(record.level()),
                        content: format!("{}", record.args()),
                    }),
                }))
                .ok();

            writeln!(
                f,
                "[{} {} {}] {}",
                timestamp,
                record.level(),
                record.module_path().unwrap_or_default(),
                record.args()
            )
        })
        .init();
}
