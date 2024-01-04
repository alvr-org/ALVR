use crate::backend::{tcp, SocketReader, SocketWriter};

use super::CONTROL_PORT;
use alvr_common::{anyhow::Result, ConResult, HandleTryAgain, ToCon};
use alvr_session::SocketBufferSize;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    marker::PhantomData,
    mem,
    net::{IpAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};

// This corresponds to the length of the payload
const FRAMED_PREFIX_LENGTH: usize = mem::size_of::<u32>();

struct RecvState {
    packet_length: usize, // contains length prefix
    packet_cursor: usize, // counts also the length prefix bytes
}

fn framed_send<S: Serialize>(
    socket: &mut TcpStream,
    buffer: &mut Vec<u8>,
    packet: &S,
) -> Result<()> {
    let serialized_size = bincode::serialized_size(&packet)? as usize;
    let packet_size = serialized_size + FRAMED_PREFIX_LENGTH;

    if buffer.len() < packet_size {
        buffer.resize(packet_size, 0);
    }

    buffer[0..FRAMED_PREFIX_LENGTH].copy_from_slice(&(serialized_size as u32).to_be_bytes());
    bincode::serialize_into(&mut buffer[FRAMED_PREFIX_LENGTH..packet_size], &packet)?;

    socket.send(&buffer[0..packet_size])?;

    Ok(())
}

fn framed_recv<R: DeserializeOwned>(
    socket: &mut TcpStream,
    buffer: &mut Vec<u8>,
    maybe_recv_state: &mut Option<RecvState>,
    timeout: Duration,
) -> ConResult<R> {
    let deadline = Instant::now() + timeout;

    let recv_state_mut = if let Some(state) = maybe_recv_state {
        state
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
            FRAMED_PREFIX_LENGTH + u32::from_be_bytes(payload_length_bytes) as usize;

        if buffer.len() < packet_length {
            buffer.resize(packet_length, 0);
        }

        maybe_recv_state.insert(RecvState {
            packet_length,
            packet_cursor: 0,
        })
    };

    loop {
        recv_state_mut.packet_cursor +=
            socket.recv(&mut buffer[recv_state_mut.packet_cursor..recv_state_mut.packet_length])?;

        if recv_state_mut.packet_cursor == recv_state_mut.packet_length {
            break;
        } else if Instant::now() > deadline {
            return alvr_common::try_again();
        }
    }

    let packet = bincode::deserialize(&buffer[FRAMED_PREFIX_LENGTH..recv_state_mut.packet_length])
        .to_con()?;

    *maybe_recv_state = None;

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
    recv_state: Option<RecvState>,
    _phantom: PhantomData<T>,
}

impl<R: DeserializeOwned> ControlSocketReceiver<R> {
    pub fn recv(&mut self, timeout: Duration) -> ConResult<R> {
        framed_recv(
            &mut self.inner,
            &mut self.buffer,
            &mut self.recv_state,
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
                buffer: vec![],
                _phantom: PhantomData,
            },
            ControlSocketReceiver {
                inner: self.inner,
                buffer: vec![],
                recv_state: None,
                _phantom: PhantomData,
            },
        ))
    }
}
