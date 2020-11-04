use alvr_common::logging::*;
use fern::Dispatch;
use log::LevelFilter;
use std::{fs::OpenOptions, sync::Once};

pub fn init_logging() {
    static INIT_LOGGING_ENTRY_POINT: Once = Once::new();

    INIT_LOGGING_ENTRY_POINT.call_once(|| {
        if cfg!(debug_assertions) {
            Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{}] At {}:{}: {}",
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
        .chain(
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(driver_log_path())
            {
                Ok(file) => fern::Output::from(file),
                // This doubles output in debug builds when we fail to open the log file
                // but at least messages go somewhere on release builds
                Err(_) => fern::Output::from(std::io::stdout()),
            },
        )
        .apply()
        .unwrap();
    });

    set_panic_hook();
}
