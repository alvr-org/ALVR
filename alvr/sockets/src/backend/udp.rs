use crate::LOCAL_IP;

use super::SocketReader;
use alvr_common::{anyhow::Result, ConResult, IOToCon};
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

impl SocketReader for UdpSocket {
    fn recv(&mut self, buffer: &mut [u8]) -> ConResult<usize> {
        let bytes = UdpSocket::recv(self, buffer).io_to_con()?;

        Ok(bytes)
    }

    fn peek(&self, buffer: &mut [u8]) -> ConResult<usize> {
        let bytes = UdpSocket::peek(self, buffer).io_to_con()?;

        Ok(bytes)
    }
}
