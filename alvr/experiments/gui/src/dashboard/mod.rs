#[cfg(feature = "gpl")]
mod sixtyfps;
#[cfg(not(feature = "gpl"))]
mod tui;

#[cfg(feature = "gpl")]
pub use self::sixtyfps::*;
#[cfg(not(feature = "gpl"))]
pub use self::tui::*;
