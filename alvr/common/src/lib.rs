pub mod data;
pub mod logging;

#[cfg(not(target_os = "android"))]
pub mod commands;

pub mod prelude {
    pub use crate::{
        fmt_e,
        logging::{log_event, Event, StrResult},
        trace_err, trace_err_dbg, trace_none, trace_str,
    };
    pub use log::{debug, error, info, warn};
}
