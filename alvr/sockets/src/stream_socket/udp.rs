use super::StreamId;
use crate::{Ldc, LOCAL_IP};
use alvr_common::prelude::*;
use bytes::{Buf, Bytes, BytesMut};
use futures::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, Mutex},
};
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

pub async fn bind(port: u16) -> StrResult<UdpSocket> {
    trace_err!(UdpSocket::bind((LOCAL_IP, port)).await)
}

pub async fn connect(
    socket: UdpSocket,
    peer_ip: IpAddr,
    port: u16,
) -> StrResult<(UdpStreamSendSocket, UdpStreamReceiveSocket)> {
    let peer_addr = (peer_ip, port).into();
    let socket = UdpFramed::new(socket, Ldc::new());
    let (send_socket, receive_socket) = socket.split();

    Ok((
        UdpStreamSendSocket {
            peer_addr,
            inner: Arc::new(Mutex::new(send_socket)),
        },
        UdpStreamReceiveSocket {
            peer_addr,
            inner: receive_socket,
        },
    ))
}

pub async fn receive_loop(
    mut socket: UdpStreamReceiveSocket,
    packet_enqueuers: Arc<Mutex<HashMap<StreamId, mpsc::UnboundedSender<BytesMut>>>>,
) -> StrResult {
    while let Some(maybe_packet) = socket.inner.next().await {
        let (mut packet_bytes, address) = trace_err!(maybe_packet)?;

        if address != socket.peer_addr {
            continue;
        }

        let stream_id = packet_bytes.get_u16();
        if let Some(enqueuer) = packet_enqueuers.lock().await.get_mut(&stream_id) {
            trace_err!(enqueuer.send(packet_bytes))?;
        }
    }

    Ok(())
}
