use alvr_common::logging::*;
use fern::{log_file, Dispatch};
use log::LevelFilter;
use std::{
    fs::{self, OpenOptions},
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedSender;

// Define logging modes, create crash log and (re)create session log
pub fn init_logging(log_senders: Arc<Mutex<Vec<UnboundedSender<String>>>>) {
    // create driver log file or else the tail command will not work
    fs::OpenOptions::new()
        .create(true)
        .open(driver_log_path())
        .ok();

    if cfg!(debug_assertions) {
        Dispatch::new()
            .format(move |out, message, record| {
                let log_line = format!(
                    "{} [{}] At {}:{}: {}",
                    chrono::Local::now().format("%H:%M:%S.%f"),
                    record.level(),
                    record.file().unwrap(),
                    record.line().unwrap(),
                    message
                );
                for sender in &*log_senders.lock().unwrap() {
                    sender.send(log_line.clone()).ok();
                }
                out.finish(format_args!("{}", log_line));
            })
            .level(LevelFilter::Debug)
            .chain(std::io::stdout())
    } else {
        Dispatch::new()
            .format(move |out, message, record| {
                let log_line = format!(
                    "{} [{}] {}",
                    chrono::Local::now().format("%H:%M:%S.%f"),
                    record.level(),
                    message
                );
                for sender in &*log_senders.lock().unwrap() {
                    sender.send(log_line.clone()).ok();
                }
                out.finish(format_args!("{}", log_line));
            })
            .level(LevelFilter::Info)
    }
    .chain(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(SESSION_LOG_FNAME)
            .unwrap(),
    )
    .chain(
        Dispatch::new()
            .level(LevelFilter::Error)
            .chain(log_file(CRASH_LOG_FNAME).unwrap()),
    )
    .apply()
    .unwrap();

    crate::logging::set_show_error_fn_and_panic_hook(|message| {
        msgbox::create("ALVR crashed", &message, msgbox::IconType::Error)
    });
}
