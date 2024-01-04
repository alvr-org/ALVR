use crate::LOCAL_IP;

use super::{SocketReader, SocketWriter};
use alvr_common::{anyhow::Result, con_bail, ConResult, HandleTryAgain, ToCon};
use alvr_session::{DscpTos, SocketBufferSize};
use std::{
    io::Read,
    io::Write,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

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

    if let Some(ip) = server_ip {
        if server_address.ip() != ip {
            con_bail!(
                "Connected to wrong client: Expected: {ip}, Found {}",
                server_address.ip()
            );
        }
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

impl SocketWriter for TcpStream {
    fn send(&mut self, buffer: &[u8]) -> Result<()> {
        self.write_all(buffer)?;

        Ok(())
    }
}

impl SocketReader for TcpStream {
    fn recv(&mut self, buffer: &mut [u8]) -> ConResult<usize> {
        Read::read(self, buffer).handle_try_again()
    }

    fn peek(&self, buffer: &mut [u8]) -> ConResult<usize> {
        TcpStream::peek(self, buffer).handle_try_again()
    }
}
