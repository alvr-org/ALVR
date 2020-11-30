#[cfg(windows)]
pub mod audio;

#[cfg(windows)]
pub mod commands;

pub mod data;
pub mod graphics;
pub mod logging;
pub mod sockets;

pub use logging::StrResult;
