use anyhow::Result;
use std::{error::Error, fmt::Display};

pub enum ConnectionError {
    Timeout,
    Other(anyhow::Error),
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::Timeout => write!(f, "Timeout"),
            ConnectionError::Other(s) => write!(f, "{}", s),
        }
    }
}

pub type ConResult<T = ()> = Result<T, ConnectionError>;

pub fn timeout<T>() -> ConResult<T> {
    Err(ConnectionError::Timeout)
}

#[macro_export]
macro_rules! con_bail {
    ($($args:tt)+) => {
        return Err(alvr_common::ConnectionError::Other(alvr_common::anyhow::anyhow!($($args)+)))
    };
}

pub trait ToCon<T> {
    fn to_con(self) -> ConResult<T>;
}

impl<T> ToCon<T> for Option<T> {
    fn to_con(self) -> ConResult<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(ConnectionError::Other(anyhow::anyhow!("Unexpected None"))),
        }
    }
}

impl<T, E: Error + Send + Sync + 'static> ToCon<T> for Result<T, E> {
    fn to_con(self) -> ConResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => Err(ConnectionError::Other(e.into())),
        }
    }
}

pub trait AnyhowToCon<T> {
    fn to_con(self) -> ConResult<T>;
}

impl<T> AnyhowToCon<T> for Result<T, anyhow::Error> {
    fn to_con(self) -> ConResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => Err(ConnectionError::Other(e)),
        }
    }
}
