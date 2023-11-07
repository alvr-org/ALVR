mod average;
mod connection_result;
mod inputs;
mod logging;
mod primitives;
mod version;

use once_cell::sync::Lazy;
use parking_lot::{Condvar, Mutex, RwLockWriteGuard};
use serde::{Deserialize, Serialize};
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

pub type OptLazy<T> = Lazy<Mutex<Option<T>>>;

pub const fn lazy_mut_none<T>() -> OptLazy<T> {
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

#[derive(PartialEq, Eq, Debug)]
pub enum LifecycleState {
    StartingUp,
    Idle,
    Resumed,
    ShuttingDown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Streaming,
    Disconnecting,
}

pub fn wait_rwlock<T>(condvar: &Condvar, guard: &mut RwLockWriteGuard<'_, T>) {
    let staging_mutex = Mutex::<()>::new(());
    let mut inner_guard = staging_mutex.lock();
    RwLockWriteGuard::unlocked(guard, move || {
        condvar.wait(&mut inner_guard);
    });
}
