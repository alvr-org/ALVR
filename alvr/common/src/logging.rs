use anyhow::Result;
use backtrace::Backtrace;
use serde::{Deserialize, Serialize};
use settings_schema::SettingsSchema;
use std::{error::Error, fmt::Display};

#[derive(
    SettingsSchema, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum LogSeverity {
    Error = 3,
    Warning = 2,
    Info = 1,
    Debug = 0,
}

impl LogSeverity {
    pub fn from_log_level(level: log::Level) -> Self {
        match level {
            log::Level::Error => LogSeverity::Error,
            log::Level::Warn => LogSeverity::Warning,
            log::Level::Info => LogSeverity::Info,
            log::Level::Debug | log::Level::Trace => LogSeverity::Debug,
        }
    }

    pub fn into_log_level(self) -> log::Level {
        match self {
            LogSeverity::Error => log::Level::Error,
            LogSeverity::Warning => log::Level::Warn,
            LogSeverity::Info => log::Level::Info,
            LogSeverity::Debug => log::Level::Debug,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub severity: LogSeverity,
    pub content: String,
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let err_str = format!(
            "What happened:\n{panic_info}\n\nBacktrace:\n{:?}",
            Backtrace::new()
        );

        log::error!("{err_str}");

        #[cfg(all(not(target_os = "android"), feature = "enable-messagebox"))]
        std::thread::spawn(move || {
            rfd::MessageDialog::new()
                .set_title("ALVR panicked")
                .set_description(&err_str)
                .set_level(rfd::MessageLevel::Error)
                .show();
        });
    }))
}

pub fn show_w<W: Display + Send + 'static>(w: W) {
    log::warn!("{w}");

    #[cfg(all(not(target_os = "android"), feature = "enable-messagebox"))]
    std::thread::spawn(move || {
        rfd::MessageDialog::new()
            .set_title("ALVR warning")
            .set_description(&w.to_string())
            .set_level(rfd::MessageLevel::Warning)
            .show()
    });
}

pub fn show_warn<T, E: Display + Send + 'static>(res: Result<T, E>) -> Option<T> {
    res.map_err(show_w).ok()
}

#[allow(unused_variables)]
fn show_e_block<E: Display>(e: E, blocking: bool) {
    log::error!("{e}");

    #[cfg(all(not(target_os = "android"), feature = "enable-messagebox"))]
    {
        // Store the last error shown in a message box. Do not open a new message box if the content
        // of the error has not changed
        use once_cell::sync::Lazy;
        use parking_lot::Mutex;

        static LAST_MESSAGEBOX_ERROR: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("".into()));

        let err_string = e.to_string();
        let last_messagebox_error_ref = &mut *LAST_MESSAGEBOX_ERROR.lock();
        if *last_messagebox_error_ref != err_string {
            let show_msgbox = {
                let err_string = err_string.clone();
                move || {
                    rfd::MessageDialog::new()
                        .set_title("ALVR error")
                        .set_description(&err_string)
                        .set_level(rfd::MessageLevel::Error)
                        .show()
                }
            };

            if blocking {
                show_msgbox();
            } else {
                std::thread::spawn(show_msgbox);
            }

            *last_messagebox_error_ref = err_string;
        }
    }
}

pub fn show_e<E: Display>(e: E) {
    show_e_block(e, false);
}

pub fn show_e_dbg<E: std::fmt::Debug>(e: E) {
    show_e_block(format!("{e:?}"), false);
}

pub fn show_e_blocking<E: Display>(e: E) {
    show_e_block(e, true);
}

pub fn show_err<T, E: Display>(res: Result<T, E>) -> Option<T> {
    res.map_err(|e| show_e_block(e, false)).ok()
}

pub fn show_err_blocking<T, E: Display>(res: Result<T, E>) -> Option<T> {
    res.map_err(|e| show_e_block(e, true)).ok()
}

pub trait ToAny<T> {
    fn to_any(self) -> Result<T>;
}

impl<T> ToAny<T> for Option<T> {
    fn to_any(self) -> Result<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(anyhow::anyhow!("Unexpected None")),
        }
    }
}

impl<T, E: Error + Send + Sync + 'static> ToAny<T> for Result<T, E> {
    fn to_any(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => Err(e.into()),
        }
    }
}
