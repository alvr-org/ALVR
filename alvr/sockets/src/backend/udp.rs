use crate::LOCAL_IP;

use super::{SocketReader, SocketWriter};
use alvr_common::{anyhow::Result, ConResult, HandleTryAgain};
use alvr_session::{DscpTos, SocketBufferSize};
use socket2::{MaybeUninitSlice, Socket};
use std::{
    ffi::c_int,
    mem,
    net::{IpAddr, UdpSocket},
    time::Duration,
};

// Create tokio socket, convert to socket2, apply settings, convert back to tokio. This is done to
// let tokio set all the internal parameters it needs from the start.
pub fn bind(
    port: u16,
    dscp: Option<DscpTos>,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<UdpSocket> {
    let socket = UdpSocket::bind((LOCAL_IP, port))?.into();

    crate::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();

    crate::set_dscp(&socket, dscp);

    Ok(socket.into())
}

pub fn connect(
    socket: &UdpSocket,
    peer_ip: IpAddr,
    port: u16,
    timeout: Duration,
) -> Result<(UdpSocket, Socket)> {
    socket.connect((peer_ip, port))?;
    socket.set_read_timeout(Some(timeout))?;

    Ok((socket.try_clone()?, socket.try_clone()?.into()))
}

impl SocketWriter for UdpSocket {
    fn send(&mut self, buffer: &[u8]) -> Result<()> {
        UdpSocket::send(self, buffer)?;

        Ok(())
    }
}

impl SocketReader for Socket {
    fn recv(&mut self, buffer: &mut [u8]) -> ConResult<usize> {
        Socket::recv(self, unsafe { mem::transmute(buffer) }).handle_try_again()
    }

    fn peek(&self, buffer: &mut [u8]) -> ConResult<usize> {
        #[cfg(windows)]
        const FLAGS: c_int = 0x02 | 0x8000; // MSG_PEEK | MSG_PARTIAL
        #[cfg(not(windows))]
        const FLAGS: c_int = 0x02 | 0x20; // MSG_PEEK | MSG_TRUNC

        let buffer = MaybeUninitSlice::new(unsafe { mem::transmute(buffer) });
        Ok(self
            .recv_vectored_with_flags(&mut [buffer], FLAGS)
            .handle_try_again()?
            .0)
    }
}
