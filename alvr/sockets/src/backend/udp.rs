use crate::LOCAL_IP;

use super::{SocketReader, SocketWriter};
use alvr_common::{anyhow::Result, ConResult, HandleTryAgain};
use alvr_session::SocketBufferSize;
use std::net::{IpAddr, UdpSocket};

// Create tokio socket, convert to socket2, apply settings, convert back to tokio. This is done to
// let tokio set all the internal parameters it needs from the start.
pub fn bind(
    port: u16,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<UdpSocket> {
    let socket = UdpSocket::bind((LOCAL_IP, port))?.into();

    crate::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();

    Ok(socket.into())
}

pub fn connect(socket: &UdpSocket, peer_ip: IpAddr, port: u16) -> Result<(UdpSocket, UdpSocket)> {
    socket.connect((peer_ip, port))?;

    Ok((socket.try_clone()?, socket.try_clone()?))
}

impl SocketWriter for UdpSocket {
    fn send(&mut self, buffer: &[u8]) -> Result<()> {
        UdpSocket::send(self, buffer)?;

        Ok(())
    }
}

impl SocketReader for UdpSocket {
    fn recv(&mut self, buffer: &mut [u8]) -> ConResult<usize> {
        UdpSocket::recv(self, buffer).handle_try_again()
    }

    fn peek(&self, buffer: &mut [u8]) -> ConResult<usize> {
        UdpSocket::peek(self, buffer).handle_try_again()
    }
}
