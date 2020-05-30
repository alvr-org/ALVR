use alvr_common::logging::*;
use fern::{log_file, Dispatch};
use log::LevelFilter;
use std::sync::Once;

pub fn init_logging() {
    static INIT_LOGGING_ENTRY_POINT: Once = Once::new();

    INIT_LOGGING_ENTRY_POINT.call_once(|| {
        std::fs::remove_file(driver_log_path()).ok();

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
        .chain(log_file(driver_log_path()).unwrap())
        .apply()
        .unwrap();
    });

    set_show_error_fn_and_panic_hook(|_| ());
}
