pub mod logging;
pub mod version;

#[cfg(not(target_os = "android"))]
pub mod commands;

pub use logging::*;
pub use version::*;

pub type StrResult<T = ()> = Result<T, String>;

pub mod prelude {
    pub use crate::{
        fmt_e,
        logging::{log_event, Event},
        trace_err, trace_err_dbg, trace_none, trace_str, StrResult,
    };
    pub use log::{debug, error, info, warn};
}
