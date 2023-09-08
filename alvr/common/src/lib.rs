mod average;
mod connection_result;
mod inputs;
mod logging;
mod primitives;
mod version;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

pub use anyhow;
pub use log;
pub use once_cell;
pub use parking_lot;
pub use semver;
pub use settings_schema;

pub use average::*;
pub use connection_result::*;
pub use inputs::*;
pub use log::{debug, error, info, warn};
pub use logging::*;
pub use primitives::*;
pub use version::*;

pub const ALVR_NAME: &str = "ALVR";

pub type LazyMutOpt<T> = Lazy<Mutex<Option<T>>>;

pub const fn lazy_mut_none<T>() -> LazyMutOpt<T> {
    Lazy::new(|| Mutex::new(None))
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
