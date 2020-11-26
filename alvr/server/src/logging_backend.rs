use crate::ALVR_DIR;
use alvr_common::{logging::*, *};
use fern::Dispatch;
use log::LevelFilter;
use std::fs;
use tokio::sync::broadcast::Sender;

pub fn init_logging(log_sender: Sender<String>) {
    let mut log_dispatch = Dispatch::new().format(move |out, message, record| {
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
        log_dispatch = log_dispatch
            .level(LevelFilter::Debug)
            .chain(std::io::stdout());
    } else {
        log_dispatch = log_dispatch.level(LevelFilter::Info);
    }

    log_dispatch
        .chain(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(ALVR_DIR.join(SESSION_LOG_FNAME))
                .unwrap(),
        )
        .chain(
            Dispatch::new()
                .level(LevelFilter::Error)
                .chain(fern::log_file(ALVR_DIR.join(CRASH_LOG_FNAME)).unwrap()),
        )
        .apply()
        .unwrap();

    // if cfg!(debug_assertions) {
    //     Dispatch::new()
    //         .format(|out, message, record| {
    //             out.finish(format_args!(
    //                 "[{}] At {}:{}: {}",
    //                 record.level(),
    //                 record.file().unwrap(),
    //                 record.line().unwrap(),
    //                 message
    //             ))
    //         })
    //         .level(LevelFilter::Trace)
    //         .chain(std::io::stdout())
    // } else {
    //     Dispatch::new()
    //         .format(|out, message, record| {
    //             out.finish(format_args!("[{}] {}", record.level(), message))
    //         })
    //         .level(LevelFilter::Info)
    // }
    // .chain(
    //     match fs::OpenOptions::new()
    //         .write(true)
    //         .create(true)
    //         .truncate(true)
    //         .open(logging::driver_log_path())
    //     {
    //         Ok(file) => fern::Output::from(file),
    //         // This doubles output in debug builds when we fail to open the log file
    //         // but at least messages go somewhere on release builds
    //         Err(_) => fern::Output::from(std::io::stdout()),
    //     },
    // )
    // .apply()
    // .unwrap();

    logging::set_panic_hook();
}
