use crate::backend::{SocketReader, SocketWriter, tcp};

use super::CONTROL_PORT;
use alvr_common::{ConResult, HandleTryAgain, ToCon, anyhow::Result};
use alvr_session::SocketBufferSize;
use bincode::config;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    marker::PhantomData,
    mem,
    net::{IpAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};

// This corresponds to the length of the payload
const FRAMED_PREFIX_LENGTH: usize = mem::size_of::<u32>();

fn framed_send<S: Serialize>(
    socket: &mut TcpStream,
    buffer: &mut Vec<u8>,
    packet: &S,
) -> Result<()> {
    buffer.resize(FRAMED_PREFIX_LENGTH, 0);

    let encoded_size = bincode::serde::encode_into_std_write(packet, buffer, config::standard())?;

    buffer[0..FRAMED_PREFIX_LENGTH].copy_from_slice(&(encoded_size as u32).to_le_bytes());

    socket.send(&buffer[0..FRAMED_PREFIX_LENGTH + encoded_size])?;

    Ok(())
}

fn framed_recv<R: DeserializeOwned>(
    socket: &mut TcpStream,
    buffer: &mut Vec<u8>,
    recv_cursor: &mut Option<usize>,
    timeout: Duration,
) -> ConResult<R> {
    let deadline = Instant::now() + timeout;

    let recv_cursor_ref = if let Some(cursor) = recv_cursor {
        cursor
    } else {
        let mut payload_length_bytes = [0; FRAMED_PREFIX_LENGTH];

        loop {
            let count = socket.peek(&mut payload_length_bytes).handle_try_again()?;
            if count == FRAMED_PREFIX_LENGTH {
                break;
            } else if Instant::now() > deadline {
                return alvr_common::try_again();
            }
        }

        let packet_length =
            FRAMED_PREFIX_LENGTH + u32::from_le_bytes(payload_length_bytes) as usize;

        buffer.resize(packet_length, 0);

        recv_cursor.insert(0)
    };

    loop {
        *recv_cursor_ref += socket.recv(&mut buffer[*recv_cursor_ref..])?;

        if *recv_cursor_ref == buffer.len() {
            break;
        } else if Instant::now() > deadline {
            return alvr_common::try_again();
        }
    }

    let (packet, _) =
        bincode::serde::decode_from_slice(&buffer[FRAMED_PREFIX_LENGTH..], config::standard())
            .to_con()?;

    *recv_cursor = None;

    Ok(packet)
}

pub struct ControlSocketSender<T> {
    inner: TcpStream,
    buffer: Vec<u8>,
    _phantom: PhantomData<T>,
}

impl<S: Serialize> ControlSocketSender<S> {
    pub fn send(&mut self, packet: &S) -> Result<()> {
        framed_send(&mut self.inner, &mut self.buffer, packet)
    }
}

pub struct ControlSocketReceiver<T> {
    inner: TcpStream,
    buffer: Vec<u8>,
    recv_cursor: Option<usize>,
    _phantom: PhantomData<T>,
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

pub fn get_server_listener(timeout: Duration) -> Result<TcpListener> {
    let listener = tcp::bind(
        timeout,
        CONTROL_PORT,
        None,
        SocketBufferSize::Default,
        SocketBufferSize::Default,
    )?;

    Ok(listener)
}

// Proto-control-socket that can send and receive any packet. After the split, only the packets of
// the specified types can be exchanged
pub struct ProtoControlSocket {
    inner: TcpStream,
}

pub enum PeerType<'a> {
    AnyClient(Vec<IpAddr>),
    Server(&'a TcpListener),
}

impl ProtoControlSocket {
    pub fn connect_to(timeout: Duration, peer: PeerType<'_>) -> ConResult<(Self, IpAddr)> {
        let socket = match peer {
            PeerType::AnyClient(ips) => {
                tcp::connect_to_client(
                    timeout,
                    &ips,
                    CONTROL_PORT,
                    SocketBufferSize::Default,
                    SocketBufferSize::Default,
                )?
                .0
            }
            PeerType::Server(listener) => tcp::accept_from_server(listener, None, timeout)?.0,
        };

        let peer_ip = socket.peer_addr().to_con()?.ip();

        Ok((Self { inner: socket }, peer_ip))
    }

    pub fn send<S: Serialize>(&mut self, packet: &S) -> Result<()> {
        framed_send(&mut self.inner, &mut vec![], packet)
    }

    pub fn recv<R: DeserializeOwned>(&mut self, timeout: Duration) -> ConResult<R> {
        framed_recv(&mut self.inner, &mut vec![], &mut None, timeout)
    }

    pub fn split<S: Serialize, R: DeserializeOwned>(
        self,
        timeout: Duration,
    ) -> Result<(ControlSocketSender<S>, ControlSocketReceiver<R>)> {
        self.inner.set_read_timeout(Some(timeout))?;

        Ok((
            ControlSocketSender {
                inner: self.inner.try_clone()?,
                buffer: vec![0; FRAMED_PREFIX_LENGTH],
                _phantom: PhantomData,
            },
            ControlSocketReceiver {
                inner: self.inner,
                buffer: vec![0; FRAMED_PREFIX_LENGTH],
                recv_cursor: None,
                _phantom: PhantomData,
            },
        ))
    }
}
