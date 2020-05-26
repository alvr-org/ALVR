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
#[repr(u8)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "id", content = "data")]
pub enum LogId {
    ClientFoundOk,
    ClientFoundInvalid,
    ClientFoundWrongIp,
    // Note: this should be a string but rust strings are not C compatible
    ClientFoundWrongVersion([u8; 32]),
}

#[macro_export]
macro_rules! format_id {
    ($id:expr) => {
        format!("#{}#", serde_json::to_string(&$id).unwrap())
    };
}

#[macro_export]
macro_rules! _format_err {
    (@ $($($args:tt)+)?) => {
        format!("At {}:{}", file!(), line!()) $(+ ", " + &format!($($args)+))?
    };
    (id: $id:expr $(, $($args_rest:tt)+)?) => {
        format_id!($id) + " " + &_format_err!(@ $($($args_rest)+)?)
    };
    ($($args:tt)*) => {
        _format_err!(@ $($args)*)
    };
}

#[macro_export]
macro_rules! trace_str {
    ($($args:tt)*) => {
        Err(_format_err!($($args)*))
    };
}

#[macro_export]
macro_rules! trace_err {
    ($res:expr $(, $($args_rest:tt)+)?) => {
        $res.map_err(|e| _format_err!($($($args_rest)+)?) + &format!(": {}", e))
    };
}

// trace_err variant for errors that do not implement fmt::Display
#[macro_export]
macro_rules! trace_err_dbg {
    ($res:expr $(, $($args_rest:tt)+)?) => {
        $res.map_err(|e| _format_err!($($($args_rest)+)?) + &format!(": {:?}", e))
    };
}

#[macro_export]
macro_rules! trace_none {
    ($res:expr $(, $($args_rest:tt)+)?) => {
        $res.ok_or_else(|| _format_err!($($($args_rest)+)?))
    };
}

#[macro_export]
macro_rules! _log {
    (@ $level:expr, $($args:tt)+) => {
        log::log!($level, $($args)+)
    };
    ($level:expr, id: $id:expr $(, $($args_rest:tt)+)?) => {
        _log!(@ $level, "{}", format_id!($id) $(+ " " + &format!($($args_rest)+))?)
    };
    ($level:expr, $($args:tt)+) => {
        _log!(@ $level, $($args)+)
    };
}

#[macro_export]
macro_rules! error {
    ($($args:tt)*) => {
        _log!(log::Level::Error, $($args)*)
    };
}

#[macro_export]
macro_rules! warn {
    ($($args:tt)*) => {
        _log!(log::Level::Warn, $($args)*)
    };
}

#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        _log!(log::Level::Info, $($args)*)
    };
}

#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => {
        _log!(log::Level::Debug, $($args)*)
    };
}
