use serde::{Deserialize, Serialize};
use std::{fmt::Display, future::Future};

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .unwrap_or(&"Unavailable");
        let err_str = format!(
            "Message: {:?}\nBacktrace:\n{:?}",
            message,
            backtrace::Backtrace::new()
        );

        log::error!("{}", err_str);

        #[cfg(windows)]
        std::thread::spawn(move || {
            msgbox::create("ALVR panicked", &err_str, msgbox::IconType::Error).ok();
        });
    }))
}

pub fn show_w<W: Display>(w: W) {
    log::warn!("{}", w);

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
    log::error!("{}", e);

    // GDK crashes because of initialization in multiple thread
    #[cfg(windows)]
    {
        // Store the last error shown in a message box. Do not open a new message box if the content
        // of the error has not changed
        use parking_lot::Mutex;

        lazy_static::lazy_static! {
            static ref LAST_MESSAGEBOX_ERROR: Mutex<String> = Mutex::new("".into());
        }

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
    show_e_block(format!("{:?}", e), false);
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventSeverity {
    Error,
    Warning,
    Info,
    Debug,
}

// todo: remove some unused statistics
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")] // todo: remove casing conversion
pub struct Statistics {
    pub total_packets: u64,
    pub packet_rate: u64,
    pub packets_lost_total: u64,
    pub packets_lost_per_second: u64,
    pub total_sent: u64,
    pub sent_rate: f32,
    pub total_latency: f32,
    pub encode_latency: f32,
    pub encode_latency_max: f32,
    pub transport_latency: f32,
    pub decode_latency: f32,
    pub fec_percentage: u32,
    pub fec_failure_total: u64,
    pub fec_failure_in_second: u64,
    pub client_f_p_s: u32, // the name will be fixed after the old dashboard is removed
    pub server_f_p_s: u32,
}

// This struct is temporary, until we switch to the new event system
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Raw {
    pub timestamp: String,
    pub severity: EventSeverity,
    pub content: String,
}

// Event is serialized as #{ "id": "..." [, "data": ...] }#
// Pound signs are used to identify start and finish of json
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "id", content = "data")]
pub enum ServerEvent {
    SessionUpdated,
    SessionSettingsExtrapolationFailed,
    ClientFoundOk,
    ClientFoundInvalid,
    ClientFoundWrongVersion(String),
    ClientConnected,
    ClientDisconnected,
    UpdateDownloadedBytesCount(usize),
    UpdateDownloadError,
    Statistics(Statistics),
    Raw(Raw),
}

pub fn log_event(id: ServerEvent) {
    log::info!("#{}#", serde_json::to_string(&id).unwrap());
}

#[macro_export]
macro_rules! fmt_e {
    ($($args:tt)+) => {
        Err(format!($($args)+))
    };
}

#[macro_export]
macro_rules! trace_str {
    () => {
        format!("At {}:{}", file!(), line!())
    };
}

#[macro_export]
macro_rules! trace_err {
    ($res:expr) => {
        $res.map_err(|e| format!("{}: {}", trace_str!(), e))
    };
}

// trace_err variant for errors that do not implement fmt::Display
#[macro_export]
macro_rules! trace_err_dbg {
    ($res:expr) => {
        $res.map_err(|e| format!("{}: {:?}", trace_str!(), e))
    };
}

#[macro_export]
macro_rules! trace_none {
    ($res:expr) => {
        $res.ok_or_else(|| trace_str!())
    };
}
