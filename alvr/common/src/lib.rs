mod average;
mod logging;
mod paths;
mod primitives;
mod version;

use std::{
    fmt::Display,
    sync::atomic::{AtomicBool, Ordering},
};

pub mod prelude {
    pub use crate::{
        con_e, con_fmt_e, enone, err, err_dbg, fmt_e, logging::*, timeout, to_con_e, ConResult,
        ConnectionError, StrResult,
    };
    pub use log::{debug, error, info, warn};
}

pub use log;
pub use once_cell;
pub use parking_lot;
pub use semver;
pub use settings_schema;

pub use average::*;
pub use logging::*;
pub use paths::*;
pub use primitives::*;
pub use version::*;

pub const ALVR_NAME: &str = "ALVR";

pub type StrResult<T = ()> = Result<T, String>;

pub enum ConnectionError {
    Timeout,
    Other(String),
}
impl Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::Timeout => write!(f, "Timeout"),
            ConnectionError::Other(s) => write!(f, "{}", s),
        }
    }
}
pub type ConResult<T = ()> = Result<T, ConnectionError>;

pub fn timeout<T>() -> ConResult<T> {
    Err(ConnectionError::Timeout)
}

// Simple wrapper for AtomicBool when using Ordering::Relaxed. Deref cannot be implemented (cannot
// return local reference)
pub struct RelaxedAtomic(AtomicBool);

impl RelaxedAtomic {
    pub const fn new(initial_value: bool) -> Self {
        Self(AtomicBool::new(initial_value))
    }

    pub fn value(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: bool) {
        self.0.store(value, Ordering::Relaxed);
    }
}
