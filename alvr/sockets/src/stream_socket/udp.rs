use crate::{Ldc, LOCAL_IP};
use alvr_common::{anyhow::Result, con_bail, parking_lot::Mutex, ConResult, ToCon};
use alvr_session::SocketBufferSize;
use bytes::{Buf, Bytes, BytesMut};
use futures::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{mpsc, Arc},
    time::Duration,
};
use tokio::{net::UdpSocket, runtime::Runtime, time};
use tokio_util::udp::UdpFramed;

#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct UdpStreamSendSocket {
    pub peer_addr: SocketAddr,
    pub inner: Arc<Mutex<SplitSink<UdpFramed<Ldc>, (Bytes, SocketAddr)>>>,
}

// peer_addr is needed to check that the packet comes from the desired device. Connecting directly
// to the peer is not supported by UdpFramed.
pub struct UdpStreamReceiveSocket {
    pub peer_addr: SocketAddr,
    pub inner: SplitStream<UdpFramed<Ldc>>,
}

// Create tokio socket, convert to socket2, apply settings, convert back to tokio. This is done to
// let tokio set all the internal parameters it needs from the start.
pub fn bind(
    runtime: &Runtime,
    port: u16,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<UdpSocket> {
    let socket = runtime.block_on(UdpSocket::bind((LOCAL_IP, port)))?;
    let socket = socket2::Socket::from(socket.into_std()?);

    super::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();

    let _tokio_guard = runtime.enter();
    let socket = UdpSocket::from_std(socket.into())?;

    Ok(socket)
}

pub fn connect(
    socket: UdpSocket,
    peer_ip: IpAddr,
    port: u16,
) -> (UdpStreamSendSocket, UdpStreamReceiveSocket) {
    let peer_addr = (peer_ip, port).into();
    let socket = UdpFramed::new(socket, Ldc::new());
    let (send_socket, receive_socket) = socket.split();

    (
        UdpStreamSendSocket {
            peer_addr,
            inner: Arc::new(Mutex::new(send_socket)),
        },
        UdpStreamReceiveSocket {
            peer_addr,
            inner: receive_socket,
        },
    )
}

pub fn recv(
    runtime: &Runtime,
    timeout: Duration,
    socket: &mut UdpStreamReceiveSocket,
    packet_enqueuers: &mut HashMap<u16, mpsc::Sender<BytesMut>>,
) -> ConResult {
    if let Some(maybe_packet) = runtime.block_on(async {
        tokio::select! {
            res = socket.inner.next() => res.map(|p| p.to_con()),
            _ = time::sleep(timeout) => Some(alvr_common::timeout()),
        }
    }) {
        let (mut packet_bytes, address) = maybe_packet?;

        if address != socket.peer_addr {
            // Non fatal
            return Ok(());
        }

        let stream_id = packet_bytes.get_u16();
        if let Some(enqueuer) = packet_enqueuers.get_mut(&stream_id) {
            enqueuer.send(packet_bytes).to_con()?;
        }

        Ok(())
    } else {
        con_bail!("Socket closed")
    }
}
