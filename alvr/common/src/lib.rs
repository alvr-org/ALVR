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

pub const HEAD_ENTER_CLICK_PATH: &str = "/user/head/input/enter/click";
pub const MENU_CLICK_PATH: &str = "/user/hand/left/input/menu/click";
pub const A_CLICK_PATH: &str = "/user/hand/right/input/a/click";
pub const A_TOUCH_PATH: &str = "/user/hand/right/input/a/touch";
pub const B_CLICK_PATH: &str = "/user/hand/right/input/b/click";
pub const B_TOUCH_PATH: &str = "/user/hand/right/input/b/touch";
pub const X_CLICK_PATH: &str = "/user/hand/left/input/x/click";
pub const X_TOUCH_PATH: &str = "/user/hand/left/input/x/touch";
pub const Y_CLICK_PATH: &str = "/user/hand/left/input/y/click";
pub const Y_TOUCH_PATH: &str = "/user/hand/left/input/y/touch";
pub const LEFT_SQUEEZE_CLICK_PATH: &str = "/user/hand/left/input/squeeze/click";
pub const LEFT_SQUEEZE_VALUE_PATH: &str = "/user/hand/left/input/squeeze/value";
pub const LEFT_TRIGGER_CLICK_PATH: &str = "/user/hand/left/input/trigger/click";
pub const LEFT_TRIGGER_VALUE_PATH: &str = "/user/hand/left/input/trigger/value";
pub const LEFT_TRIGGER_TOUCH_PATH: &str = "/user/hand/left/input/trigger/touch";
pub const LEFT_THUMBSTICK_X_PATH: &str = "/user/hand/left/input/thumbstick/x";
pub const LEFT_THUMBSTICK_Y_PATH: &str = "/user/hand/left/input/thumbstick/y";
pub const LEFT_THUMBSTICK_CLICK_PATH: &str = "/user/hand/left/input/thumbstick/click";
pub const LEFT_THUMBSTICK_TOUCH_PATH: &str = "/user/hand/left/input/thumbstick/touch";
pub const LEFT_THUMBREST_TOUCH_PATH: &str = "/user/hand/left/input/thumbrest/touch";
pub const RIGHT_SQUEEZE_CLICK_PATH: &str = "/user/hand/right/input/squeeze/click";
pub const RIGHT_SQUEEZE_VALUE_PATH: &str = "/user/hand/right/input/squeeze/value";
pub const RIGHT_TRIGGER_CLICK_PATH: &str = "/user/hand/right/input/trigger/click";
pub const RIGHT_TRIGGER_VALUE_PATH: &str = "/user/hand/right/input/trigger/value";
pub const RIGHT_TRIGGER_TOUCH_PATH: &str = "/user/hand/right/input/trigger/touch";
pub const RIGHT_THUMBSTICK_X_PATH: &str = "/user/hand/right/input/thumbstick/x";
pub const RIGHT_THUMBSTICK_Y_PATH: &str = "/user/hand/right/input/thumbstick/y";
pub const RIGHT_THUMBSTICK_CLICK_PATH: &str = "/user/hand/right/input/thumbstick/click";
pub const RIGHT_THUMBSTICK_TOUCH_PATH: &str = "/user/hand/right/input/thumbstick/touch";
pub const RIGHT_THUMBREST_TOUCH_PATH: &str = "/user/hand/right/input/thumbrest/touch";

pub static HEAD_ENTER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(HEAD_ENTER_CLICK_PATH));
pub static MENU_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(MENU_CLICK_PATH));
pub static A_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(A_CLICK_PATH));
pub static A_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(A_TOUCH_PATH));
pub static B_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(B_CLICK_PATH));
pub static B_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(B_TOUCH_PATH));
pub static X_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(X_CLICK_PATH));
pub static X_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(X_TOUCH_PATH));
pub static Y_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(Y_CLICK_PATH));
pub static Y_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(Y_TOUCH_PATH));
pub static LEFT_SQUEEZE_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_CLICK_PATH));
pub static LEFT_SQUEEZE_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_SQUEEZE_VALUE_PATH));
pub static LEFT_TRIGGER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_CLICK_PATH));
pub static LEFT_TRIGGER_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_VALUE_PATH));
pub static LEFT_TRIGGER_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_TRIGGER_TOUCH_PATH));
pub static LEFT_THUMBSTICK_X_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_THUMBSTICK_X_PATH));
pub static LEFT_THUMBSTICK_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(LEFT_THUMBSTICK_Y_PATH));
pub static LEFT_THUMBSTICK_CLICK_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBSTICK_CLICK_PATH));
pub static LEFT_THUMBSTICK_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBSTICK_TOUCH_PATH));
pub static LEFT_THUMBREST_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(LEFT_THUMBREST_TOUCH_PATH));
pub static RIGHT_SQUEEZE_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_CLICK_PATH));
pub static RIGHT_SQUEEZE_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_SQUEEZE_VALUE_PATH));
pub static RIGHT_TRIGGER_CLICK_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_CLICK_PATH));
pub static RIGHT_TRIGGER_VALUE_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_VALUE_PATH));
pub static RIGHT_TRIGGER_TOUCH_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_TRIGGER_TOUCH_PATH));
pub static RIGHT_THUMBSTICK_X_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_THUMBSTICK_X_PATH));
pub static RIGHT_THUMBSTICK_Y_ID: Lazy<u64> = Lazy::new(|| hash_string(RIGHT_THUMBSTICK_Y_PATH));
pub static RIGHT_THUMBSTICK_CLICK_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBSTICK_CLICK_PATH));
pub static RIGHT_THUMBSTICK_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBSTICK_TOUCH_PATH));
pub static RIGHT_THUMBREST_TOUCH_ID: Lazy<u64> =
    Lazy::new(|| hash_string(RIGHT_THUMBREST_TOUCH_PATH));

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
