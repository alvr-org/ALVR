use anyhow::{anyhow, Result};
use std::{
    error::Error,
    fmt::Display,
    io,
    sync::mpsc::{RecvTimeoutError, TryRecvError},
};

pub enum ConnectionError {
    TryAgain(anyhow::Error),
    Other(anyhow::Error),
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (ConnectionError::TryAgain(e) | ConnectionError::Other(e)) = self;
        write!(f, "{e:?}")
    }
}

pub type ConResult<T = ()> = Result<T, ConnectionError>;

pub fn try_again<T>() -> ConResult<T> {
    Err(ConnectionError::TryAgain(anyhow!("Try again")))
}

#[macro_export]
macro_rules! con_bail {
    ($($args:tt)+) => {
        return Err(alvr_common::ConnectionError::Other(alvr_common::anyhow::anyhow!($($args)+)))
    };
}

pub trait ToCon<T> {
    /// Convert result to ConResult. The error is always mapped to `Other()`
    fn to_con(self) -> ConResult<T>;
}

impl<T> ToCon<T> for Option<T> {
    fn to_con(self) -> ConResult<T> {
        self.ok_or_else(|| ConnectionError::Other(anyhow!("Unexpected None")))
    }
}

impl<T, E: Error + Send + Sync + 'static> ToCon<T> for Result<T, E> {
    fn to_con(self) -> ConResult<T> {
        self.map_err(|e| ConnectionError::Other(e.into()))
    }
}

pub trait AnyhowToCon<T> {
    fn to_con(self) -> ConResult<T>;
}

impl<T> AnyhowToCon<T> for Result<T, anyhow::Error> {
    fn to_con(self) -> ConResult<T> {
        self.map_err(ConnectionError::Other)
    }
}

pub trait HandleTryAgain<T> {
    fn handle_try_again(self) -> ConResult<T>;
}

impl<T> HandleTryAgain<T> for io::Result<T> {
    fn handle_try_again(self) -> ConResult<T> {
        self.map_err(|e| {
            if e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::WouldBlock {
                ConnectionError::TryAgain(e.into())
            } else {
                ConnectionError::Other(e.into())
            }
        })
    }
}

impl<T> HandleTryAgain<T> for std::result::Result<T, RecvTimeoutError> {
    fn handle_try_again(self) -> ConResult<T> {
        self.map_err(|e| match e {
            RecvTimeoutError::Timeout => ConnectionError::TryAgain(e.into()),
            RecvTimeoutError::Disconnected => ConnectionError::Other(e.into()),
        })
    }
}

impl<T> HandleTryAgain<T> for std::result::Result<T, TryRecvError> {
    fn handle_try_again(self) -> ConResult<T> {
        self.map_err(|e| match e {
            TryRecvError::Empty => ConnectionError::TryAgain(e.into()),
            TryRecvError::Disconnected => ConnectionError::Other(e.into()),
        })
    }
}
