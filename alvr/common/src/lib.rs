mod logging;

use semver::{Prerelease, Version};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub use glam;
pub use lazy_static::lazy_static;
pub use log;
pub use logging::*;
pub use semver;

pub type StrResult<T = ()> = Result<T, String>;

pub const ALVR_NAME: &str = "ALVR";

pub mod prelude {
    pub use crate::{
        fmt_e, logging::*, trace_err, trace_err_dbg, trace_none, trace_str, StrResult,
    };
    pub use log::{debug, error, info, warn};
}

lazy_static! {
    pub static ref ALVR_VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

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

lazy_static! {
    pub static ref HEAD_ID: u64 = hash_string(HEAD_PATH);
    pub static ref LEFT_HAND_ID: u64 = hash_string(LEFT_HAND_PATH);
    pub static ref RIGHT_HAND_ID: u64 = hash_string(RIGHT_HAND_PATH);
    pub static ref LEFT_CONTROLLER_HAPTIC_ID: u64 = hash_string(LEFT_CONTROLLER_HAPTIC_PATH);
    pub static ref RIGHT_CONTROLLER_HAPTIC_ID: u64 = hash_string(RIGHT_CONTROLLER_HAPTIC_PATH);
}
