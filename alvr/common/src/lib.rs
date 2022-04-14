mod logging;

use once_cell::sync::Lazy;
use semver::Version;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicBool, Ordering},
};

pub mod prelude {
    pub use crate::{enone, err, err_dbg, fmt_e, logging::*, StrResult};
    pub use log::{debug, error, info, warn};
}

pub use glam;
pub use log;
pub use logging::*;
pub use once_cell;
pub use parking_lot;
pub use semver;

pub type StrResult<T = ()> = Result<T, String>;

pub const ALVR_NAME: &str = "ALVR";
pub static ALVR_VERSION: Lazy<Version> =
    Lazy::new(|| Version::parse(env!("CARGO_PKG_VERSION")).unwrap());

// Consistent across architectures, might not be consistent across different compiler versions.
pub fn hash_string(string: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    string.hash(&mut hasher);
    hasher.finish()
}

pub fn is_nightly() -> bool {
    ALVR_VERSION.build.contains("nightly")
}

pub fn is_stable() -> bool {
    ALVR_VERSION.pre.is_empty() && !is_nightly()
}

// Semver compatible versions will produce the same protocol ID. Protocol IDs are not ordered
// As a convention, encode/decode the protocol ID bytes as little endian.
// Only makor and
pub fn protocol_id() -> u64 {
    let protocol_string = if ALVR_VERSION.pre.is_empty() {
        ALVR_VERSION.major.to_string()
    } else {
        format!("{}-{}", ALVR_VERSION.major, ALVR_VERSION.pre)
    };

    hash_string(&protocol_string)
}

// deprecated
pub fn is_version_compatible(other_version: &Version) -> bool {
    let protocol_string = if other_version.pre.is_empty() {
        other_version.major.to_string()
    } else {
        format!("{}-{}", other_version.major, other_version.pre)
    };

    protocol_id() == hash_string(&protocol_string)
}

pub const HEAD_PATH: &str = "/user/head";
pub const LEFT_HAND_PATH: &str = "/user/hand/left";
pub const RIGHT_HAND_PATH: &str = "/user/hand/right";
pub const LEFT_CONTROLLER_HAPTIC_PATH: &str = "/user/hand/left/output/haptic";
pub const RIGHT_CONTROLLER_HAPTIC_PATH: &str = "/user/hand/right/output/haptic";

pub static HEAD_ID: Lazy<u64> = Lazy::new(|| hash_string(HEAD_PATH));
pub static LEFT_HAND_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_HAND_PATH));
pub static RIGHT_HAND_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_HAND_PATH));
pub static LEFT_CONTROLLER_HAPTIC_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_CONTROLLER_HAPTIC_PATH));
pub static RIGHT_CONTROLLER_HAPTIC_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_CONTROLLER_HAPTIC_PATH));

// Simple wrapper for AtomicBool when using Ordering::Relaxed. Deref cannot be implemented (cannot
// return local reference)
pub struct RelaxedAtomic(AtomicBool);

impl RelaxedAtomic {
    pub fn new(initial_value: bool) -> Self {
        Self(AtomicBool::new(initial_value))
    }

    pub fn value(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: bool) {
        self.0.store(value, Ordering::Relaxed);
    }
}
