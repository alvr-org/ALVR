use serde::{Deserialize, Serialize};

pub type StrResult<T = ()> = Result<T, String>;

pub const DRIVER_LOG_FNAME: &str = "driver_log.txt";
pub const SESSION_LOG_FNAME: &str = "session_log.txt";
pub const CRASH_LOG_FNAME: &str = "crash_log.txt";

fn default_show_error_fn(_: &str) {}

// todo: consider using atomics or lazy_static
static mut SHOW_ERROR_CB: fn(&str) = default_show_error_fn;

pub fn set_show_error_fn_and_panic_hook(cb: fn(&str)) {
    unsafe { SHOW_ERROR_CB = cb };
    std::panic::set_hook(Box::new(|panic_info| {
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .unwrap_or(&"Unavailable");

        let err_str = format!(
            "ALVR panicked.\nMessage: {:?}\nBacktrace:\n{:?}",
            message,
            backtrace::Backtrace::new()
        );
        log::error!("{}", err_str);
        unsafe { SHOW_ERROR_CB(&err_str) };
    }))
}

pub fn show_err<T, E: std::fmt::Display>(res: Result<T, E>) -> Result<T, ()> {
    res.map_err(|e| {
        log::error!("{}", e);
        unsafe { SHOW_ERROR_CB(&format!("{}", e)) };
    })
}

// Log id is serialized as #{ "id": "...", "data": [...|null] }#
// Pound signs are used to identify start and finish of json
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "id", content = "data")]
pub enum LogId {
    None,
    ClientFoundOk,
    ClientFoundInvalid,
    ClientFoundWrongIp,
    ClientFoundWrongVersion(String),
}

#[macro_export]
macro_rules! _format_id {
    ($id:expr) => {
        &format!("#{}#", serde_json::to_string(&$id).unwrap())
    };
}

#[macro_export]
macro_rules! _format_id_address_message_impl {
    ($id:expr, $($message_fmt:expr $(, $args:expr)*)?) => {
        String::new()
            + _format_id!($id)
            + &format!(" At {}:{}", file!(), line!())
            $(+ ", " + &format!($message_fmt $(, $args)*))?
    };
}

#[macro_export]
macro_rules! _format_id_address_message {
    (id: $id:expr $(, $($args_rest:tt)+)?) => {
        _format_id_address_message_impl!($id, $($($args_rest)+)?)
    };
    ($($args:tt)*) => {
        _format_id_address_message_impl!($crate::logging::LogId::None, $($args)*)
    };
}

#[macro_export]
macro_rules! trace_str {
    ($($args:tt)*) => {
        Err(_format_id_address_message!($($args)*))
    };
}

#[macro_export]
macro_rules! trace_err {
    ($res:expr $(, $($args_rest:tt)+)?) => {
        $res.map_err(|e| _format_id_address_message!($($($args_rest)+)?) + &format!(": {}", e))
    };
}

// trace_err variant for errors that do not implement fmt::Display
#[macro_export]
macro_rules! trace_err_dbg {
    ($res:expr $(, $($args_rest:tt)+)?) => {
        $res.map_err(|e| _format_id_address_message!($($($args_rest)+)?) + &format!(": {:?}", e))
    };
}

#[macro_export]
macro_rules! trace_none {
    ($res:expr $(, $($args_rest:tt)+)?) => {
        $res.ok_or_else(|| _format_id_address_message!($($($args_rest)+)?))
    };
}

#[macro_export]
macro_rules! _log_impl {
    ($level_ident:ident, $id:expr, $($message_fmt:expr $(, $args:expr)*)?) => {
        log::log!(
            log::Level::$level_ident,
            "{}",
            String::new() + _format_id!($id) + " " $(+ &format!($message_fmt $(, $args)*))?
        )
    };
}

#[macro_export]
macro_rules! _log {
    ($level_ident:ident, id: $id:expr $(, $($args_rest:tt)+)?) => {
        _log_impl!($level_ident, $id, $($($args_rest)+)?)
    };
    ($level_ident:ident $(, $($args_rest:tt)+)?) => {
        _log_impl!($level_ident, $crate::logging::LogId::None, $($($args_rest)+)?)
    };
}

#[macro_export]
macro_rules! error {
    ($($args:tt)*) => {
        _log!(Error, $($args)*)
    };
}

#[macro_export]
macro_rules! warn {
    ($($args:tt)*) => {
        _log!(Warn, $($args)*)
    };
}

#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        _log!(Info, $($args)*)
    };
}

#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => {
        _log!(Debug, $($args)*)
    };
}
