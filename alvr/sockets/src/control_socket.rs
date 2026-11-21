use crate::handshake::{framed_recv, framed_send};
use alvr_common::{ConResult, anyhow::Result};
use serde::{Serialize, de::DeserializeOwned};
use std::{marker::PhantomData, net::TcpStream, time::Duration};

pub struct ControlSocketSender<T> {
    pub(crate) inner: TcpStream,
    pub(crate) buffer: Vec<u8>,
    pub(crate) _phantom: PhantomData<T>,
}

impl<S: Serialize> ControlSocketSender<S> {
    pub fn send(&mut self, packet: &S) -> Result<()> {
        framed_send(&mut self.inner, &mut self.buffer, packet)
    }
}

pub struct ControlSocketReceiver<T> {
    pub(crate) inner: TcpStream,
    pub(crate) buffer: Vec<u8>,
    pub(crate) recv_cursor: Option<usize>,
    pub(crate) _phantom: PhantomData<T>,
}

impl<R: DeserializeOwned> ControlSocketReceiver<R> {
    pub fn recv(&mut self, timeout: Duration) -> ConResult<R> {
        framed_recv(
            &mut self.inner,
            &mut self.buffer,
            &mut self.recv_cursor,
            timeout,
        )
    }
}
