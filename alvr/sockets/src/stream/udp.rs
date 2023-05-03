use crate::{Ldc, LOCAL_IP};
use alvr_common::prelude::*;
use alvr_session::SocketBufferSize;
use async_std::net;
use bytes::BytesMut;
use futures::{channel::mpsc, TryFutureExt};
use libp2p::gossipsub::TopicHash;
use socket2::Socket;
use std::{collections::HashMap, net::IpAddr, sync::Arc};


pub fn set_socket_buffers(
    socket: &socket2::Socket,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> StrResult {
    info!(
        "Initial socket buffer size: send: {}B, recv: {}B",
        socket.send_buffer_size().map_err(err!())?,
        socket.recv_buffer_size().map_err(err!())?
    );

    {
        let maybe_size = match send_buffer_bytes {
            SocketBufferSize::Default => None,
            SocketBufferSize::Maximum => Some(u32::MAX),
            SocketBufferSize::Custom(size) => Some(size),
        };

        if let Some(size) = maybe_size {
            if let Err(e) = socket.set_send_buffer_size(size as usize) {
                info!("Error setting socket send buffer: {e}");
            } else {
                info!(
                    "Set socket send buffer succeeded: {}",
                    socket.send_buffer_size().map_err(err!())?
                );
            }
        }
    }

    {
        let maybe_size = match recv_buffer_bytes {
            SocketBufferSize::Default => None,
            SocketBufferSize::Maximum => Some(u32::MAX),
            SocketBufferSize::Custom(size) => Some(size),
        };

        if let Some(size) = maybe_size {
            if let Err(e) = socket.set_recv_buffer_size(size as usize) {
                info!("Error setting socket recv buffer: {e}");
            } else {
                info!(
                    "Set socket recv buffer succeeded: {}",
                    socket.recv_buffer_size().map_err(err!())?
                );
            }
        }
    }

    Ok(())
}

pub struct UdpSender {
    max_packet_size: usize,
    socket: Arc<UdpSocket>,
    next_packet_index: u32,
}

pub struct UdpReceiver {
    receiver: mpsc::UnboundedReceiver<BytesMut>,
    next_packet_shards: HashMap<usize, BytesMut>,
    next_packet_shards_count: Option<usize>,
    next_packet_index: u32,
}

struct RecvBuffer {
    buffer: Vec<u8>, // NB: first byte is the topic
    content_size: usize, // counting first byte for the topic
}

pub struct UdpSocket {
    socket: Arc<net::UdpSocket>,
    max_packet_size: usize,
    recv_channel_senders: HashMap<u8, mpsc::UnboundedSender<RecvBuffer>>,
    recv_free_buffers: Vec<Vec<u8>>,
}

impl UdpSocket {
    // returns: socket, control receiver
    pub fn new(
        port: u16,
        send_buffer_bytes: SocketBufferSize,
        recv_buffer_bytes: SocketBufferSize,
        max_packet_size: usize,
    ) -> StrResult<(Self, UdpReceiver)> {
        let socket = UdpSocket::bind((LOCAL_IP, port)).map_err(err!())?;
        let socket = socket2::Socket::from(socket);

        set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();

        let this = Self {
            socket: socket.into(),
            max_packet_size,
            inbound_senders: todo!(),
        };
        let conrol_receiver = this.subscribe_to_stream(CONTROL_HASH);

        Ok((this, conrol_receiver))
    }

    pub fn connect_peer(&mut self, addr: IpAddr) {
        self.socket.connect(addr);
    }

    pub fn request_stream(&self) -> UdpSender {
        UdpSender {
            max_packet_size: self.max_packet_size,
            socket: Arc::clone(&self.socket),
            next_packet_index: 0,
        }
    }

    pub fn subscribe_to_stream(&mut self, topic: TopicHash) -> UdpReceiver {
        let (sender, receiver) = mpsc::unbounded();
        self.recv_channel_senders.insert(topic.as_str().as_bytes()[0], sender);

        UdpReceiver {
            receiver,
            next_packet_shards: HashMap::new(),
            next_packet_shards_count: None,
            next_packet_index: 0,
        }
    }

    pub async fn recv_loop(&mut self) -> StrResult {
        loop {
            let mut buffer = self
                .recv_free_buffers
                .pop()
                .unwrap_or(vec![0; self.max_packet_size]);
            let received_size = self.socket.recv(&mut buffer).map_err(err!());

            let topic = 
        }
    }
}
