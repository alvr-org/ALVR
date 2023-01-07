// Note: for StreamSocket, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate
//
// StreamSender and StreamReceiver endpoints allow for convenient conversion of the header to/from
// bytes while still handling the additional byte buffer with zero copies and extra allocations.

mod tcp;
mod throttled_udp;
mod udp;

use alvr_common::prelude::*;
use alvr_session::{SocketBufferSize, SocketProtocol};
use bytes::{Buf, BufMut, BytesMut};
use futures::SinkExt;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    marker::PhantomData,
    net::IpAddr,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tcp::{TcpStreamReceiveSocket, TcpStreamSendSocket};
use throttled_udp::{ThrottledUdpStreamReceiveSocket, ThrottledUdpStreamSendSocket};
use tokio::net;
use tokio::sync::{mpsc, Mutex};
use udp::{UdpStreamReceiveSocket, UdpStreamSendSocket};

#[derive(Clone)]
enum StreamSendSocket {
    Udp(UdpStreamSendSocket),
    ThrottledUdp(ThrottledUdpStreamSendSocket),
    Tcp(TcpStreamSendSocket),
}

enum StreamReceiveSocket {
    Udp(UdpStreamReceiveSocket),
    ThrottledUdp(ThrottledUdpStreamReceiveSocket),
    Tcp(TcpStreamReceiveSocket),
}

pub struct SendBufferLock<'a> {
    header_bytes: &'a mut BytesMut,
    buffer_bytes: BytesMut,
}

impl Deref for SendBufferLock<'_> {
    type Target = BytesMut;
    fn deref(&self) -> &BytesMut {
        &self.buffer_bytes
    }
}

impl DerefMut for SendBufferLock<'_> {
    fn deref_mut(&mut self) -> &mut BytesMut {
        &mut self.buffer_bytes
    }
}

impl Drop for SendBufferLock<'_> {
    fn drop(&mut self) {
        // the extra split is to avoid moving buffer_bytes
        self.header_bytes.unsplit(self.buffer_bytes.split())
    }
}

/*#[derive(Serialize, Deserialize)]
struct PacketControlHeader {
    stream_id: u16,
    index: u32,
    size: u32,
    shard_index: u32,
}*/

#[derive(Clone)]
pub struct StreamSender<T> {
    stream_id: u16,
    socket: StreamSendSocket,
    // if the packet index overflows the worst that happens is a false positive packet loss
    next_packet_index: u32,
    full_packet_index: u32,
    _phantom: PhantomData<T>,
}

impl<T: Serialize> StreamSender<T> {
    async fn send_buffer(&self, buffer: BytesMut) {
        match &self.socket {
            StreamSendSocket::Udp(socket) => socket
                .inner
                .lock()
                .await
                .feed((buffer.freeze(), socket.peer_addr))
                .await
                .map_err(err!())
                .unwrap(),
            StreamSendSocket::Tcp(socket) => socket
                .lock()
                .await
                .feed(buffer.freeze())
                .await
                .map_err(err!())
                .unwrap(),
            StreamSendSocket::ThrottledUdp(socket) => {
                socket.send(buffer.freeze()).await.map_err(err!()).unwrap()
            }
        };
    }

    pub async fn send(&mut self, data_header: &T, payload: Vec<u8>) {
        // u16 + u32 + u32 + u32 + u32
        let offset = (2 + 4 * 4) as usize;

        let data_header_size = bincode::serialized_size(data_header)
            .map_err(err!())
            .unwrap() as usize;
        let payload_size = payload.len();
        let total_payload_size = data_header_size + payload_size;

        let max_payload_size = 1400 - offset; // TODO: Need to get max allowed buffer size
        let total_shards = // Total number of shards will be at least 1 because header is sent separately
            payload_size / max_payload_size + 1 + (payload_size % max_payload_size != 0) as usize;

        let mut data_remain = total_payload_size;

        let mut buffer = BytesMut::with_capacity(offset + max_payload_size);

        buffer.put_u16(self.stream_id);
        buffer.put_u32(self.next_packet_index);
        buffer.put_u32(total_shards as u32); // + 1 header shard
        buffer.put_u32(total_payload_size as u32);
        buffer.put_u32(self.full_packet_index);

        let mut buffer_writer = buffer.writer();
        bincode::serialize_into(&mut buffer_writer, data_header)
            .map_err(err!())
            .ok();
        self.send_buffer(buffer_writer.into_inner()).await;
        data_remain -= data_header_size;
        self.next_packet_index += 1;

        let mut last_max_index = 0;
        for _ in 0..total_shards - 1 {
            // If number of total shards is correct, the shard size will be always down to zero
            let shard_size = cmp::min(data_remain, max_payload_size);
            data_remain -= shard_size;
            let mut buffer = BytesMut::with_capacity(offset + shard_size);

            buffer.put_u16(self.stream_id);
            buffer.put_u32(self.next_packet_index);
            buffer.put_u32(total_shards as u32);
            buffer.put_u32(total_payload_size as u32);
            buffer.put_u32(self.full_packet_index);

            let offset = last_max_index;
            let max = offset + shard_size;
            //debug!("offset:{last_max_index}/max:{max}/len:{total_payload_size}");
            buffer.put_slice(&payload[offset..max]);
            last_max_index = max;

            self.send_buffer(buffer).await;
            self.next_packet_index += 1;
        }
        self.full_packet_index += 1;

        match &self.socket {
            StreamSendSocket::Udp(socket) => socket
                .inner
                .lock()
                .await
                .flush()
                .await
                .map_err(err!())
                .unwrap(),
            StreamSendSocket::Tcp(socket) => {
                socket.lock().await.flush().await.map_err(err!()).unwrap()
            }
            StreamSendSocket::ThrottledUdp(_) => {}
        };
    }
}

enum StreamReceiverType {
    Queue(mpsc::UnboundedReceiver<BytesMut>),
    // QuicReliable(...)
}

pub struct ReceivedPacket<T> {
    pub header: T,
    pub buffer: BytesMut,
    pub had_packet_loss: bool,
}

pub struct StreamReceiver<T> {
    receiver: StreamReceiverType,
    next_packet_index: u32,
    previous_packet_index: u32,
    full_packet_index: u32,
    had_packet_loss: bool,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> StreamReceiver<T> {
    fn check_packet_loss(&mut self, packet_index: u32) {
        if packet_index != self.next_packet_index && self.previous_packet_index != packet_index {
            let previous_packet = self.previous_packet_index;
            let next_packet = self.next_packet_index;
            info!(
                "Lost packet: {next_packet}/Received: {packet_index}/Previous: {previous_packet}"
            );
            if self.had_packet_loss == false {
                self.had_packet_loss = true;
            }
        }
    }

    pub async fn recv(&mut self) -> StrResult<ReceivedPacket<T>> {
        let mut received_shards = 0;
        let mut buffer = BytesMut::new();
        let mut last_max_index = 0;
        loop {
            let mut bytes = match &mut self.receiver {
                StreamReceiverType::Queue(receiver) => {
                    receiver.recv().await.ok_or_else(enone!())?
                }
            };

            let packet_index = bytes.get_u32();
            let total_shards = bytes.get_u32() as usize;
            let total_payload_size = bytes.get_u32();
            let full_packet_index = bytes.get_u32();

            //debug!("idx:{packet_index}/p_idx:{full_packet_index}/shrd:{received_shards}/t_shrd:{total_shards}/len:{total_payload_size}");

            self.check_packet_loss(packet_index);
            if self.previous_packet_index == packet_index {
                debug!("Packet {packet_index} retransmitted");
                continue;
            }
            self.previous_packet_index = packet_index;
            self.next_packet_index = packet_index + 1;

            if self.full_packet_index != full_packet_index {
                self.full_packet_index = full_packet_index;

                self.had_packet_loss = false;

                last_max_index = 0;
                received_shards = 0;

                buffer.resize(total_payload_size as _, 0);
            }

            let len = bytes.len();
            let max = (last_max_index + len) as usize;
            //debug!("offset:{last_max_index}/max:{max}/len:{total_payload_size}");
            buffer[last_max_index..max].copy_from_slice(&bytes);
            last_max_index = max;

            received_shards += 1;
            if received_shards == total_shards || self.had_packet_loss {
                break;
            }
        }

        let mut bytes_reader = buffer.reader();
        let header = bincode::deserialize_from(&mut bytes_reader).map_err(err!())?;
        buffer = bytes_reader.into_inner();

        Ok(ReceivedPacket {
            header,
            buffer,
            // TODO: Ideally, receiver should avoid delegating packet loss handling to further code
            // and implement strategy pattern instead.
            had_packet_loss: self.had_packet_loss,
        })
    }
}

pub enum StreamSocketBuilder {
    Tcp(net::TcpListener),
    Udp(net::UdpSocket),
    ThrottledUdp(net::UdpSocket),
}

impl StreamSocketBuilder {
    pub async fn listen_for_server(
        port: u16,
        stream_socket_config: SocketProtocol,
        send_buffer_bytes: SocketBufferSize,
        recv_buffer_bytes: SocketBufferSize,
    ) -> StrResult<Self> {
        Ok(match stream_socket_config {
            SocketProtocol::Udp => StreamSocketBuilder::Udp(
                udp::bind(port, send_buffer_bytes, recv_buffer_bytes).await?,
            ),
            SocketProtocol::Tcp => StreamSocketBuilder::Tcp(
                tcp::bind(port, send_buffer_bytes, recv_buffer_bytes).await?,
            ),
            SocketProtocol::ThrottledUdp { .. } => StreamSocketBuilder::ThrottledUdp(
                udp::bind(port, send_buffer_bytes, recv_buffer_bytes).await?,
            ),
        })
    }

    pub async fn accept_from_server(self, server_ip: IpAddr, port: u16) -> StrResult<StreamSocket> {
        let (send_socket, receive_socket) = match self {
            StreamSocketBuilder::Udp(socket) => {
                let (send_socket, receive_socket) = udp::connect(socket, server_ip, port).await?;
                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            StreamSocketBuilder::Tcp(listener) => {
                let (send_socket, receive_socket) =
                    tcp::accept_from_server(listener, server_ip).await?;
                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
            StreamSocketBuilder::ThrottledUdp(socket) => {
                let (send_socket, receive_socket) =
                    throttled_udp::accept_from_server(socket, server_ip, port).await?;
                (
                    StreamSendSocket::ThrottledUdp(send_socket),
                    StreamReceiveSocket::ThrottledUdp(receive_socket),
                )
            }
        };

        Ok(StreamSocket {
            send_socket,
            receive_socket: Arc::new(Mutex::new(Some(receive_socket))),
            packet_queues: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn connect_to_client(
        client_ip: IpAddr,
        port: u16,
        protocol: SocketProtocol,
        video_byterate: u32,
        send_buffer_bytes: SocketBufferSize,
        recv_buffer_bytes: SocketBufferSize,
    ) -> StrResult<StreamSocket> {
        let (send_socket, receive_socket) = match protocol {
            SocketProtocol::Udp => {
                let socket = udp::bind(port, send_buffer_bytes, recv_buffer_bytes).await?;
                let (send_socket, receive_socket) = udp::connect(socket, client_ip, port).await?;
                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            SocketProtocol::Tcp => {
                let (send_socket, receive_socket) =
                    tcp::connect_to_client(client_ip, port, send_buffer_bytes, recv_buffer_bytes)
                        .await?;
                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
            SocketProtocol::ThrottledUdp { bitrate_multiplier } => {
                let socket = udp::bind(port, send_buffer_bytes, recv_buffer_bytes).await?;

                let (send_socket, receive_socket) = throttled_udp::connect_to_client(
                    socket,
                    client_ip,
                    port,
                    video_byterate,
                    bitrate_multiplier,
                )
                .await?;
                (
                    StreamSendSocket::ThrottledUdp(send_socket),
                    StreamReceiveSocket::ThrottledUdp(receive_socket),
                )
            }
        };

        Ok(StreamSocket {
            send_socket,
            receive_socket: Arc::new(Mutex::new(Some(receive_socket))),
            packet_queues: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

pub struct StreamSocket {
    send_socket: StreamSendSocket,
    receive_socket: Arc<Mutex<Option<StreamReceiveSocket>>>,
    packet_queues: Arc<Mutex<HashMap<u16, mpsc::UnboundedSender<BytesMut>>>>,
}

impl StreamSocket {
    pub async fn request_stream<T>(&self, stream_id: u16) -> StrResult<StreamSender<T>> {
        Ok(StreamSender {
            stream_id,
            socket: self.send_socket.clone(),
            next_packet_index: 0,
            full_packet_index: 0,
            _phantom: PhantomData,
        })
    }

    pub async fn subscribe_to_stream<T>(&self, stream_id: u16) -> StrResult<StreamReceiver<T>> {
        let (enqueuer, dequeuer) = mpsc::unbounded_channel();
        self.packet_queues.lock().await.insert(stream_id, enqueuer);

        Ok(StreamReceiver {
            receiver: StreamReceiverType::Queue(dequeuer),
            next_packet_index: 0,
            previous_packet_index: u32::MAX,
            full_packet_index: u32::MAX,
            had_packet_loss: false,
            _phantom: PhantomData,
        })
    }

    pub async fn receive_loop(&self) -> StrResult {
        match self.receive_socket.lock().await.take().unwrap() {
            StreamReceiveSocket::Udp(socket) => {
                udp::receive_loop(socket, Arc::clone(&self.packet_queues)).await
            }
            StreamReceiveSocket::Tcp(socket) => {
                tcp::receive_loop(socket, Arc::clone(&self.packet_queues)).await
            }
            StreamReceiveSocket::ThrottledUdp(socket) => {
                throttled_udp::receive_loop(socket, Arc::clone(&self.packet_queues)).await
            }
        }
    }
}
