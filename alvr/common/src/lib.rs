pub mod data;
pub mod logging;
pub mod sockets;

#[cfg(not(target_os = "android"))]
pub mod audio;

#[cfg(not(target_os = "android"))]
pub mod commands;

#[cfg(not(target_os = "android"))]
pub mod graphics;

pub use log::{debug, error, info, warn};
pub use logging::StrResult;
