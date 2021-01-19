pub mod data;
pub mod logging;
pub mod sockets;

#[cfg(windows)]
pub mod audio;
#[cfg(windows)]
pub mod commands;
#[cfg(windows)]
pub mod graphics;

pub use log::{debug, error, info, warn};
pub use logging::StrResult;
