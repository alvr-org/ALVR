mod packets;

pub use packets::*;

use alvr_common::prelude::*;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use serde::{de::DeserializeOwned, Serialize};
use std::{marker::PhantomData, thread, time::Duration};

pub struct IpcSender<S> {
    socket: LocalSocketStream,
    _phantom: PhantomData<S>,
}

impl<S: Serialize> IpcSender<S> {
    pub fn send(&mut self, message: &S) -> StrResult {
        trace_err!(bincode::serialize_into(&mut self.socket, message))
    }
}

pub struct IpcReceiver<R> {
    socket: LocalSocketStream,
    _phantom: PhantomData<R>,
}

impl<R: DeserializeOwned> IpcReceiver<R> {
    pub fn receive(&mut self) -> StrResult<R> {
        trace_err!(bincode::deserialize_from(&mut self.socket))
    }
}

pub fn ipc_connect<S, R>(name: &str) -> StrResult<(IpcSender<S>, IpcReceiver<R>)> {
    let sender = trace_err!(LocalSocketStream::connect(format!(
        "/tmp/alvr_{}_out.sock",
        name
    )))?;
    let receiver = trace_err!(trace_err!(LocalSocketListener::bind(format!(
        "/tmp/alvr_{}_in.sock",
        name
    )))?
    .accept())?;

    Ok((
        IpcSender {
            socket: sender,
            _phantom: PhantomData,
        },
        IpcReceiver {
            socket: receiver,
            _phantom: PhantomData,
        },
    ))
}

pub fn ipc_listen<S, R>(name: &str) -> StrResult<(IpcSender<S>, IpcReceiver<R>)> {
    let receiver = trace_err!(trace_err!(LocalSocketListener::bind(format!(
        "/tmp/alvr_{}_out.sock",
        name
    )))?
    .accept())?;

    // Wait for the client to setup the listener
    thread::sleep(Duration::from_millis(100));

    let sender = trace_err!(LocalSocketStream::connect(format!(
        "/tmp/alvr_{}_in.sock",
        name
    )))?;

    Ok((
        IpcSender {
            socket: sender,
            _phantom: PhantomData,
        },
        IpcReceiver {
            socket: receiver,
            _phantom: PhantomData,
        },
    ))
}
