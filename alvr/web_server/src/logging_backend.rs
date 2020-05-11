use alvr_common::logging::*;
use fern::{log_file, Dispatch};
use log::LevelFilter;

// Define logging modes, create crash log and (re)create session log
pub fn init_logging() {
    std::fs::remove_file(SESSION_LOG_FNAME).ok();

    if cfg!(debug_assertions) {
        Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} [{}] At {}:{}:\n{}",
                    chrono::Local::now().format("%H:%M:%S.%f"),
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
                out.finish(format_args!(
                    "{} [{}] {}",
                    chrono::Local::now().format("%H:%M:%S.%f"),
                    record.level(),
                    message
                ))
            })
            .level(LevelFilter::Info)
    }
    .chain(log_file(SESSION_LOG_FNAME).unwrap())
    .chain(
        Dispatch::new()
            .level(LevelFilter::Error)
            .chain(log_file(CRASH_LOG_FNAME).unwrap()),
    )
    .apply()
    .unwrap();

    crate::logging::set_show_error_fn(|message| {
        msgbox::create("ALVR crashed", &message, msgbox::IconType::Error)
    });
}
