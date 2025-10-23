use crate::{CONTROL_PORT, LOCAL_IP};
use alvr_common::{ConResult, HandleTryAgain, ToCon, anyhow::Result, con_bail};
use alvr_session::{DscpTos, SocketBufferSize};
use bincode::config;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    io::{Read, Write},
    marker::PhantomData,
    mem,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};

// This corresponds to the length of the payload
const FRAMED_PREFIX_LENGTH: usize = mem::size_of::<u32>();

pub fn bind(
    timeout: Duration,
    port: u16,
    dscp: Option<DscpTos>,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<TcpListener> {
    let socket = TcpListener::bind((LOCAL_IP, port))?.into();

    crate::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();

    crate::set_dscp(&socket, dscp);

    socket.set_read_timeout(Some(timeout))?;

    Ok(socket.into())
}

pub fn accept_from_server(
    listener: &TcpListener,
    server_ip: Option<IpAddr>,
    timeout: Duration,
) -> ConResult<(TcpStream, TcpStream)> {
    // Uses timeout set during bind()
    let (socket, server_address) = listener.accept().handle_try_again()?;

    if let Some(ip) = server_ip
        && server_address.ip() != ip
    {
        con_bail!(
            "Connected to wrong client: Expected: {ip}, Found {}",
            server_address.ip()
        );
    }

    socket.set_read_timeout(Some(timeout)).to_con()?;
    socket.set_nodelay(true).to_con()?;

    Ok((socket.try_clone().to_con()?, socket))
}

pub fn connect_to_client(
    timeout: Duration,
    client_ips: &[IpAddr],
    port: u16,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> ConResult<(TcpStream, TcpStream)> {
    let split_timeout = timeout / client_ips.len() as u32;

    let mut res = alvr_common::try_again();
    for ip in client_ips {
        res = TcpStream::connect_timeout(&SocketAddr::new(*ip, port), split_timeout)
            .handle_try_again();

        if res.is_ok() {
            break;
        }
    }
    let socket = res?.into();

    crate::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();
    socket.set_read_timeout(Some(timeout)).to_con()?;

    let socket = TcpStream::from(socket);

    socket.set_nodelay(true).to_con()?;

    Ok((socket.try_clone().to_con()?, socket))
}

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
            } else if Instant::now() > deadline {
                return alvr_common::try_again();
            }
        }

        let size = FRAMED_PREFIX_LENGTH + u32::from_le_bytes(payload_size_bytes) as usize;
        buffer.resize(size, 0);

        recv_cursor.insert(0)
    };

    loop {
        *recv_cursor_ref += socket
            .read(&mut buffer[*recv_cursor_ref..])
            .handle_try_again()?;

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
    let listener = bind(
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
                connect_to_client(
                    timeout,
                    &ips,
                    CONTROL_PORT,
                    SocketBufferSize::Default,
                    SocketBufferSize::Default,
                )?
                .0
            }
            PeerType::Server(listener) => accept_from_server(listener, None, timeout)?.0,
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
