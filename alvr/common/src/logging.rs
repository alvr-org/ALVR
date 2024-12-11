use anyhow::Result;
use backtrace::Backtrace;
use serde::{Deserialize, Serialize};
use settings_schema::SettingsSchema;
use std::{error::Error, fmt::Display};

pub const SERVER_IMPL_DBG_LABEL: &str = "SERVER IMPL";
pub const CLIENT_IMPL_DBG_LABEL: &str = "CLIENT IMPL";
pub const SERVER_CORE_DBG_LABEL: &str = "SERVER CORE";
pub const CLIENT_CORE_DBG_LABEL: &str = "CLIENT CORE";
pub const CONNECTION_DBG_LABEL: &str = "CONNECTION";
pub const SOCKETS_DBG_LABEL: &str = "SOCKETS";
pub const SERVER_GFX_DBG_LABEL: &str = "SERVER GFX";
pub const CLIENT_GFX_DBG_LABEL: &str = "CLIENT GFX";
pub const ENCODER_DBG_LABEL: &str = "ENCODER";
pub const DECODER_DBG_LABEL: &str = "DECODER";

#[macro_export]
macro_rules! dbg_server_impl {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::SERVER_IMPL_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_client_impl {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::CLIENT_IMPL_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_server_core {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::SERVER_CORE_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_client_core {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::CLIENT_CORE_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_connection {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::CONNECTION_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_sockets {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::SOCKETS_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_server_gfx {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::SERVER_GFX_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_client_gfx {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::CLIENT_GFX_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_encoder {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::ENCODER_DBG_LABEL, $($args)*);
    };
}

#[macro_export]
macro_rules! dbg_decoder {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::debug!(target: $crate::DECODER_DBG_LABEL, $($args)*);
    };
}

#[derive(SettingsSchema, Serialize, Deserialize, Clone)]
pub struct DebugGroupsConfig {
    #[schema(flag = "steamvr-restart")]
    pub server_impl: bool,
    #[schema(flag = "steamvr-restart")]
    pub client_impl: bool,
    #[schema(flag = "steamvr-restart")]
    pub server_core: bool,
    #[schema(flag = "steamvr-restart")]
    pub client_core: bool,
    #[schema(flag = "steamvr-restart")]
    pub connection: bool,
    #[schema(flag = "steamvr-restart")]
    pub sockets: bool,
    #[schema(flag = "steamvr-restart")]
    pub server_gfx: bool,
    #[schema(flag = "steamvr-restart")]
    pub client_gfx: bool,
    #[schema(flag = "steamvr-restart")]
    pub encoder: bool,
    #[schema(flag = "steamvr-restart")]
    pub decoder: bool,
}

pub fn filter_debug_groups(target: &str, config: &DebugGroupsConfig) -> bool {
    match target {
        SERVER_IMPL_DBG_LABEL => config.server_impl,
        CLIENT_IMPL_DBG_LABEL => config.client_impl,
        SERVER_CORE_DBG_LABEL => config.server_core,
        CLIENT_CORE_DBG_LABEL => config.client_core,
        CONNECTION_DBG_LABEL => config.connection,
        SOCKETS_DBG_LABEL => config.sockets,
        SERVER_GFX_DBG_LABEL => config.server_gfx,
        CLIENT_GFX_DBG_LABEL => config.client_gfx,
        ENCODER_DBG_LABEL => config.encoder,
        DECODER_DBG_LABEL => config.decoder,
        _ => true,
    }
}

pub fn is_enabled_debug_group(target: &str, config: &DebugGroupsConfig) -> bool {
    (config.server_impl && target == SERVER_IMPL_DBG_LABEL)
        || (config.client_impl && target == CLIENT_IMPL_DBG_LABEL)
        || (config.server_core && target == SERVER_CORE_DBG_LABEL)
        || (config.client_core && target == CLIENT_CORE_DBG_LABEL)
        || (config.connection && target == CONNECTION_DBG_LABEL)
        || (config.sockets && target == SOCKETS_DBG_LABEL)
        || (config.server_gfx && target == SERVER_GFX_DBG_LABEL)
        || (config.client_gfx && target == CLIENT_GFX_DBG_LABEL)
        || (config.encoder && target == ENCODER_DBG_LABEL)
        || (config.decoder && target == DECODER_DBG_LABEL)
}

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

        log::error!("ALVR panicked: {err_str}");

        #[cfg(not(target_os = "android"))]
        std::thread::spawn({
            let panic_str = panic_info.to_string();
            move || {
                rfd::MessageDialog::new()
                    .set_title("ALVR panicked")
                    .set_description(&panic_str)
                    .set_level(rfd::MessageLevel::Error)
                    .show();
            }
        });
    }))
}

pub fn show_w<W: Display + Send + 'static>(w: W) {
    log::warn!("{w}");

    #[cfg(not(target_os = "android"))]
    std::thread::spawn(move || {
        rfd::MessageDialog::new()
            .set_title("ALVR warning")
            .set_description(w.to_string())
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

    #[cfg(not(target_os = "android"))]
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
