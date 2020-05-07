pub type StrResult<T = ()> = Result<T, String>;

fn default_show_error_fn(_: &str) {}
pub static mut _SHOW_ERROR_CB: fn(&str) = default_show_error_fn;

pub fn set_show_error_fn(cb: fn(&str)) {
    unsafe { _SHOW_ERROR_CB = cb };
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
        unsafe { _SHOW_ERROR_CB(&err_str) };
    }))
}

#[macro_export]
macro_rules! trace_str {
    ($expect_fmt:expr $(, $args:expr)*) => {
        Err(format!("[{}] At {}:{}:\n", TRACE_CONTEXT, file!(), line!())
            + &format!($expect_fmt $(, $args)*))
    };
}

#[macro_export]
macro_rules! trace_err {
    ($res:expr $(, $expect_fmt:expr $(, $args:expr)*)?) => {
        $res.map_err(|e|
            format!("[{}] At {}:{}:\n", TRACE_CONTEXT, file!(), line!())
                $(+ &format!($expect_fmt $(, $args)*) + ":\n")?
                + &format!("{}", e)
        )
    };
}

// trace_err variant for errors that do not implement fmt::Display
#[macro_export]
macro_rules! trace_err_dbg {
    ($res:expr $(, $expect_fmt:expr $(, $args:expr)*)?) => {
        $res.map_err(|e|
            format!("[{}] At {}:{}:\n", TRACE_CONTEXT, file!(), line!())
                $(+ &format!($expect_fmt $(, $args)*) + ":\n")?
                + &format!("{:?}", e)
        )
    };
}

#[macro_export]
macro_rules! trace_none {
    ($res:expr $(, $none_message_fmt:expr $(, $args:expr)*)?) => {
        $res.ok_or_else(||
            format!("[{}] At {}:{}", TRACE_CONTEXT, file!(), line!())
                $(+ ", " + &format!($none_message_fmt $(, $args)*))?
        )
    };
}

#[macro_export]
macro_rules! show_err {
    ($res:expr) => {
        $res.map_err(|e| {
            log::error!("{}", e);
            unsafe { $crate::logging::_SHOW_ERROR_CB(&format!("{}", e)) };
        })
    };
}
