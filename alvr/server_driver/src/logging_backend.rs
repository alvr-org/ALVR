use alvr_common::logging::*;
use fern::Dispatch;
use log::LevelFilter;
use std::{fs, thread};
use tokio::sync::broadcast::Sender;

// Define logging modes, create crash log and (re)create session log
pub fn init_logging(log_sender: Sender<String>) {
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
                log_sender.send(log_line.clone()).ok();
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
                log_sender.send(log_line.clone()).ok();
                out.finish(format_args!("{}", log_line));
            })
            .level(LevelFilter::Info)
    }
    .chain(
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(SESSION_LOG_FNAME)
            .unwrap(),
    )
    .chain(
        Dispatch::new()
            .level(LevelFilter::Error)
            .chain(fern::log_file(CRASH_LOG_FNAME).unwrap()),
    )
    .apply()
    .unwrap();

    crate::logging::set_show_error_fn_and_panic_hook(|message| {
        let message = message.to_owned();
        thread::spawn(move || {
            msgbox::create("ALVR crashed", &message, msgbox::IconType::Error);
        });
    });
}
