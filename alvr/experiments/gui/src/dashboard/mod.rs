#[cfg(feature = "pretty")]
mod pretty;
#[cfg(feature = "terminal")]
mod terminal;

#[cfg(feature = "pretty")]
pub use self::pretty::*;
#[cfg(feature = "terminal")]
pub use self::terminal::*;

use alvr_common::StrResult;

type RequestHandler = dyn FnMut(String) -> StrResult<String>;
