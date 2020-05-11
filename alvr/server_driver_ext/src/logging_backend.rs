use alvr_common::logging::DRIVER_LOG_FNAME;
use fern::{log_file, Dispatch};
use log::LevelFilter;
use std::sync::Once;

pub fn init_logging() {
    static INIT_LOGGING_ENTRY_POINT: Once = Once::new();

    INIT_LOGGING_ENTRY_POINT.call_once(|| {
        std::fs::remove_file(DRIVER_LOG_FNAME).ok();

        if cfg!(debug_assertions) {
            Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{}] At {}:{}:\n{}",
                        record.level(),
                        record.file().unwrap(),
                        record.line().unwrap(),
                        message
                    ))
                })
                .level(LevelFilter::Trace)
                .chain(std::io::stdout())
        } else {
            Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!("[{}] {}", record.level(), message))
                })
                .level(LevelFilter::Info)
        }
        .chain(log_file(DRIVER_LOG_FNAME).unwrap())
        .apply()
        .unwrap();
    });
}
