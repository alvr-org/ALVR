use crate::{
    CLIENT_PORT,
    tcp::{accept_from_server, bind, connect_to_client},
};
use alvr_common::{ConResult, HandleTryAgain, ToCon, anyhow::Result, con_bail};
use bincode::config;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    io::{Read, Write},
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

    socket.write_all(buffer)?;

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
        let mut payload_size_bytes = [0; FRAMED_PREFIX_LENGTH];

        loop {
            let count = socket.peek(&mut payload_size_bytes).handle_try_again()?;
            if count == FRAMED_PREFIX_LENGTH {
                break;
            } else if count == 0 {
                con_bail!("Connection closed by peer");
            } else if Instant::now() > deadline {
                return alvr_common::try_again();
            }
        }

        let size = FRAMED_PREFIX_LENGTH + u32::from_le_bytes(payload_size_bytes) as usize;
        buffer.resize(size, 0);

        recv_cursor.insert(0)
    };

    loop {
        let count = socket
            .read(&mut buffer[*recv_cursor_ref..])
            .handle_try_again()?;
        if count == 0 {
            con_bail!("Connection closed by peer");
        }

        *recv_cursor_ref += count;

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

pub fn get_server_listener(timeout: Duration) -> Result<TcpListener> {
    let listener = bind(timeout, CLIENT_PORT)?;

    Ok(listener)
}

pub struct HandshakeConnection {
    pub(crate) inner: TcpStream,
}

pub enum PeerType<'a> {
    AnyClient(Vec<IpAddr>),
    Server(&'a TcpListener),
}

impl HandshakeConnection {
    pub fn connect_to(timeout: Duration, peer: PeerType<'_>) -> ConResult<(Self, IpAddr)> {
        let socket = match peer {
            PeerType::AnyClient(ips) => connect_to_client(timeout, &ips, CLIENT_PORT)?,
            PeerType::Server(listener) => accept_from_server(listener, None, timeout)?,
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
}
