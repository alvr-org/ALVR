mod logging;

use once_cell::sync::Lazy;
use semver::{Prerelease, Version};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub use glam;
pub use log;
pub use logging::*;
pub use once_cell;
pub use parking_lot;
pub use semver;

pub type StrResult<T = ()> = Result<T, String>;

pub const ALVR_NAME: &str = "ALVR";

pub mod prelude {
    pub use crate::{enone, err, err_dbg, fmt_e, logging::*, StrResult};
    pub use log::{debug, error, info, warn};
}

pub static ALVR_VERSION: Lazy<Version> =
    Lazy::new(|| Version::parse(env!("CARGO_PKG_VERSION")).unwrap());

// accept semver-compatible versions
// Note: by not having to set the requirement manually, the major version is constrained to be
// bumped when the packet layouts or some critical behaviour has changed.
pub fn is_version_compatible(other_version: &Version) -> bool {
    if other_version.pre != Prerelease::EMPTY || ALVR_VERSION.pre != Prerelease::EMPTY {
        other_version.major == ALVR_VERSION.major
            && other_version.minor == ALVR_VERSION.minor
            && other_version.patch == ALVR_VERSION.patch
            && other_version.pre == ALVR_VERSION.pre
        // Note: metadata (+) is always ignored in the version check
    } else {
        other_version.major == ALVR_VERSION.major
    }
}

pub fn is_nightly() -> bool {
    ALVR_VERSION.build.contains("nightly")
}

pub fn is_stable() -> bool {
    ALVR_VERSION.pre == Prerelease::EMPTY && !is_nightly()
}

// Consistent across architectures, might not be consistent across different compiler versions.
pub fn hash_string(string: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    string.hash(&mut hasher);
    hasher.finish()
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
