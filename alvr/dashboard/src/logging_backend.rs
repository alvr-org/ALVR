use crate::data_sources::PolledEvent;
use alvr_common::{log::LevelFilter, parking_lot::Mutex, LogEntry, LogSeverity};
use alvr_events::{Event, EventType};
use std::{
    io::Write,
    sync::{mpsc, Arc},
};

pub fn init_logging(event_sender: mpsc::Sender<PolledEvent>) {
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
                .send(PolledEvent {
                    inner: Event {
                        timestamp: timestamp.clone(),
                        event_type: EventType::Log(LogEntry {
                            severity: LogSeverity::from_log_level(record.level()),
                            content: format!("{}", record.args()),
                        }),
                    },
                    from_dashboard: true,
                })
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
