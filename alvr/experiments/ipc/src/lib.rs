mod packets;

pub use packets::*;

use alvr_common::prelude::*;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use serde::{de::DeserializeOwned, Serialize};
use std::{io::ErrorKind, marker::PhantomData, thread, time::Duration};

fn deserialize_non_blocking<R: DeserializeOwned>(
    mut socket: &mut LocalSocketStream,
) -> StrResult<Option<R>> {
    match bincode::deserialize_from(&mut socket) {
        Ok(message) => Ok(Some(message)),
        Err(e) => match *e {
            bincode::ErrorKind::Io(e) if e.kind() == ErrorKind::WouldBlock => Ok(None),
            _ => fmt_e!("IPC Error"),
        },
    }
}

pub struct IpcClient<S, R> {
    socket: LocalSocketStream,
    _phantom: PhantomData<(S, R)>,
}

impl<S: Serialize, R: DeserializeOwned> IpcClient<S, R> {
    pub fn request(&mut self, message: &S) -> StrResult<R> {
        trace_err!(bincode::serialize_into(&mut self.socket, message))?;
        trace_err!(bincode::deserialize_from(&mut self.socket))
    }
}

pub struct IpcSseReceiver<R> {
    socket: LocalSocketStream,
    _phantom: PhantomData<R>,
}

impl<R: DeserializeOwned> IpcSseReceiver<R> {
    pub fn receive_non_blocking(&mut self) -> StrResult<Option<R>> {
        deserialize_non_blocking(&mut self.socket)
    }
}

pub fn ipc_connect<CS, CR, SR>(name: &str) -> StrResult<(IpcClient<CS, CR>, IpcSseReceiver<SR>)> {
    let request_socket = trace_err!(LocalSocketStream::connect(format!(
        "/tmp/alvr_{}_request.sock",
        name
    )))?;
    let sse_socket = trace_err!(trace_err!(LocalSocketListener::bind(format!(
        "/tmp/alvr_{}_sse.sock",
        name
    )))?
    .accept())?;

    sse_socket.set_nonblocking(true).unwrap();

    Ok((
        IpcClient {
            socket: request_socket,
            _phantom: PhantomData,
        },
        IpcSseReceiver {
            socket: sse_socket,
            _phantom: PhantomData,
        },
    ))
}

pub struct IpcServer<S, R> {
    socket: LocalSocketStream,
    _phantom: PhantomData<(S, R)>,
}
impl<S: Serialize, R: DeserializeOwned> IpcServer<S, R> {
    // Ok: try again, Err: connection closed
    pub fn serve_non_blocking(&mut self, mut request_callback: impl FnMut(R) -> S) -> StrResult {
        while let Some(request) = deserialize_non_blocking(&mut self.socket)? {
            let response = request_callback(request);

            // Note: the socket is shared, so even the sending part is non blocking. Despite
            // this, WouldBlock should never happen and this call should never fail.
            trace_err!(bincode::serialize_into(&mut self.socket, &response))?;
        }

        Ok(())
    }
}

pub struct IpcSseSender<S> {
    socket: LocalSocketStream,
    _phantom: PhantomData<S>,
}

impl<S: Serialize> IpcSseSender<S> {
    pub fn send(&mut self, message: &S) -> StrResult {
        trace_err!(bincode::serialize_into(&mut self.socket, message))
    }
}

pub fn ipc_listen<CS, CR, SR>(name: &str) -> StrResult<(IpcServer<CS, CR>, IpcSseSender<SR>)> {
    let request_socket = trace_err!(trace_err!(LocalSocketListener::bind(format!(
        "/tmp/alvr_{}_request.sock",
        name
    )))?
    .accept())?;

    request_socket.set_nonblocking(true).unwrap();

    // Wait for the client to setup the sse socket listener
    thread::sleep(Duration::from_millis(100));

    let sse_sender = trace_err!(LocalSocketStream::connect(format!(
        "/tmp/alvr_{}_sse.sock",
        name
    )))?;

    Ok((
        IpcServer {
            socket: request_socket,
            _phantom: PhantomData,
        },
        IpcSseSender {
            socket: sse_sender,
            _phantom: PhantomData,
        },
    ))
}
