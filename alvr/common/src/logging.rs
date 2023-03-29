use serde::{Deserialize, Serialize};
use settings_schema::SettingsSchema;
use std::{backtrace::Backtrace, fmt::Display, future::Future};

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

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let err_str = format!(
            "What happened:\n{panic_info}\n\nBacktrace:\n{}",
            Backtrace::force_capture()
        );

        log::error!("{err_str}");

        #[cfg(windows)]
        std::thread::spawn(move || {
            msgbox::create("ALVR panicked", &err_str, msgbox::IconType::Error).ok();
        });
    }))
}

pub fn show_w<W: Display>(w: W) {
    log::warn!("{w}");

    // GDK crashes because of initialization in multiple thread
    #[cfg(windows)]
    std::thread::spawn({
        let warn_string = w.to_string();
        move || {
            msgbox::create(
                "ALVR encountered a non-fatal error",
                &warn_string,
                msgbox::IconType::Info,
            )
            .ok();
        }
    });
}

pub fn show_warn<T, E: Display>(res: Result<T, E>) -> Option<T> {
    res.map_err(show_w).ok()
}

#[allow(unused_variables)]
fn show_e_block<E: Display>(e: E, blocking: bool) {
    log::error!("{e}");

    // GDK crashes because of initialization in multiple thread
    #[cfg(windows)]
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
                    msgbox::create(
                        "ALVR encountered an error",
                        &err_string,
                        msgbox::IconType::Error,
                    )
                    .ok();
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

pub async fn show_err_async<T, E: Display>(
    future_res: impl Future<Output = Result<T, E>>,
) -> Option<T> {
    show_err(future_res.await)
}

#[macro_export]
macro_rules! fmt_e {
    ($($args:tt)+) => {
        Err(format!($($args)+))
    };
}

#[macro_export]
macro_rules! err {
    () => {
        |e| format!("At {}:{}: {e}", file!(), line!())
    };
}

// trace_err variant for errors that do not implement fmt::Display
#[macro_export]
macro_rules! err_dbg {
    () => {
        |e| format!("At {}:{}: {e:?}", file!(), line!())
    };
}

#[macro_export]
macro_rules! enone {
    () => {
        || format!("At {}:{}", file!(), line!())
    };
}

#[macro_export]
macro_rules! int_fmt_e {
    ($($args:tt)+) => {
        Err(InterruptibleError::Other(format!($($args)+)))
    };
}

#[macro_export]
macro_rules! int_e {
    () => {
        |e| match e {
            InterruptibleError::Interrupted => InterruptibleError::Interrupted,
            InterruptibleError::Other(e) => {
                InterruptibleError::Other(format!("At {}:{}: {e}", file!(), line!()))
            }
        }
    };
}

#[macro_export]
macro_rules! to_int_e {
    () => {
        |e| InterruptibleError::Other(format!("At {}:{}: {e}", file!(), line!()))
    };
}
