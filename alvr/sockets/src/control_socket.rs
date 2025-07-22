use crate::backend::{SocketReader, SocketWriter, tcp};

use super::CONTROL_PORT;
use alvr_common::{
    ConResult, HandleTryAgain, ToCon,
    anyhow::{Result, bail},
};
use alvr_session::SocketBufferSize;
use bincode::{config, enc::write::SizeWriter, error::EncodeError};
use serde::{Serialize, de::DeserializeOwned};
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
    let maybe_size = bincode::serde::encode_into_slice(
        packet,
        &mut buffer[FRAMED_PREFIX_LENGTH..],
        config::standard(),
    );

    let encoded_size = match maybe_size {
        Ok(size) => size,
        Err(EncodeError::UnexpectedEnd) => {
            // Obtaining the size of the encoded data is expensive, as the data would need to be
            // encoded twice. If the buffer is not large enough, we will pay for 3 encoding times,
            // but this should happen rarely at steady state.

            let mut size_writer = SizeWriter::default();
            bincode::serde::encode_into_writer(packet, &mut size_writer, config::standard())?;

            let packet_size = FRAMED_PREFIX_LENGTH + size_writer.bytes_written;
            if buffer.len() < packet_size {
                buffer.resize(packet_size, 0);
            }

            bincode::serde::encode_into_slice(
                packet,
                &mut buffer[FRAMED_PREFIX_LENGTH..],
                config::standard(),
            )?
        }
        Err(e) => bail!("Failed to encode packet: {}", e),
    };

    buffer[0..FRAMED_PREFIX_LENGTH].copy_from_slice(&(encoded_size as u32).to_le_bytes());

    socket.send(&buffer[0..FRAMED_PREFIX_LENGTH + encoded_size])?;

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
            FRAMED_PREFIX_LENGTH + u32::from_le_bytes(payload_length_bytes) as usize;

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

    let (packet, _) = bincode::serde::decode_from_slice(
        &buffer[FRAMED_PREFIX_LENGTH..recv_state_mut.packet_length],
        config::standard(),
    )
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
        framed_send(&mut self.inner, &mut vec![0; FRAMED_PREFIX_LENGTH], packet)
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
                recv_state: None,
                _phantom: PhantomData,
            },
        ))
    }
}
