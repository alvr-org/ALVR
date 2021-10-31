#[cfg(feature = "pretty")]
mod pretty;
#[cfg(feature = "terminal")]
mod terminal;

#[cfg(feature = "pretty")]
pub use self::pretty::*;
#[cfg(feature = "terminal")]
pub use self::terminal::*;

type RequestHandler = dyn FnMut(String) -> serde_json::Value;
