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
        check_interrupt, enone, err, err_dbg, fmt_e, int_e, int_fmt_e, interrupt, logging::*,
        to_int_e, IntResult, InterruptibleError, StrResult,
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

pub enum InterruptibleError {
    Interrupted,
    Other(String),
}
impl Display for InterruptibleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterruptibleError::Interrupted => write!(f, "Action interrupted"),
            InterruptibleError::Other(s) => write!(f, "{}", s),
        }
    }
}
pub type IntResult<T = ()> = Result<T, InterruptibleError>;

pub fn interrupt<T>() -> IntResult<T> {
    Err(InterruptibleError::Interrupted)
}

/// Bail out if interrupted
#[macro_export]
macro_rules! check_interrupt {
    ($running:expr) => {
        if !$running {
            return interrupt();
        }
    };
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
