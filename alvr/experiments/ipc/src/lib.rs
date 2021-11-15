mod packets;

pub use packets::*;

use alvr_common::prelude::*;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use serde::de::DeserializeOwned;
use std::{io::ErrorKind, thread, time::Duration};

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

pub struct IpcClient {
    socket: LocalSocketStream,
}

impl IpcClient {
    pub fn request(&mut self, message: &DriverRequest) -> StrResult<ResponseForDriver> {
        trace_err!(bincode::serialize_into(&mut self.socket, message))?;
        trace_err!(bincode::deserialize_from(&mut self.socket))
    }
}

pub struct IpcSseReceiver {
    socket: LocalSocketStream,
}

impl IpcSseReceiver {
    pub fn receive_non_blocking(&mut self) -> StrResult<Option<SsePacket>> {
        deserialize_non_blocking(&mut self.socket)
    }
}

pub fn ipc_connect(
    request_pipe_name: &str,
    sse_pipe_name: &str,
) -> StrResult<(IpcClient, IpcSseReceiver)> {
    let request_socket = trace_err!(LocalSocketStream::connect(request_pipe_name))?;
    let sse_socket = trace_err!(trace_err!(LocalSocketListener::bind(sse_pipe_name))?.accept())?;

    sse_socket.set_nonblocking(true).unwrap();

    Ok((
        IpcClient {
            socket: request_socket,
        },
        IpcSseReceiver { socket: sse_socket },
    ))
}

pub struct IpcServer {
    socket: LocalSocketStream,
}
impl IpcServer {
    // Ok: try again, Err: connection closed
    pub fn serve_non_blocking(
        &mut self,
        mut request_callback: impl FnMut(DriverRequest) -> ResponseForDriver,
    ) -> StrResult {
        while let Some(request) = deserialize_non_blocking(&mut self.socket)? {
            let response = request_callback(request);

            // Note: the socket is shared, so even the sending part is non blocking. Despite
            // this, WouldBlock should never happen and this call should never fail.
            trace_err!(bincode::serialize_into(&mut self.socket, &response))?;
        }

        Ok(())
    }
}

pub struct IpcSseSender {
    socket: LocalSocketStream,
}

impl IpcSseSender {
    pub fn send(&mut self, message: &SsePacket) -> StrResult {
        trace_err!(bincode::serialize_into(&mut self.socket, message))
    }
}

pub fn ipc_listen(
    request_pipe_name: &str,
    sse_pipe_name: &str,
) -> StrResult<(IpcServer, IpcSseSender)> {
    let listener = trace_err!(LocalSocketListener::bind(request_pipe_name))?;
    listener.set_nonblocking(true).unwrap();

    let request_socket = trace_err!(listener.accept())?;
    request_socket.set_nonblocking(true).unwrap();

    // Wait for the client to setup the sse socket listener
    thread::sleep(Duration::from_millis(100));

    let sse_sender = trace_err!(LocalSocketStream::connect(sse_pipe_name))?;

    Ok((
        IpcServer {
            socket: request_socket,
        },
        IpcSseSender { socket: sse_sender },
    ))
}
