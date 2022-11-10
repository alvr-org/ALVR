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

pub struct SenderBuffer<T> {
    inner: BytesMut,
    offset: usize,
    _phantom: PhantomData<T>,
}

impl<T> SenderBuffer<T> {
    // Get the editable part of the buffer (the header part is excluded). The returned buffer can
    // be grown at zero-cost until `preferred_max_buffer_size` (set with send_buffer()) is reached.
    // After that a reallocation will be needed but there will be no other side effects.
    pub fn get_mut(&mut self) -> SendBufferLock {
        let buffer_bytes = self.inner.split_off(self.offset);
        SendBufferLock {
            header_bytes: &mut self.inner,
            buffer_bytes,
        }
    }
}
#[derive(Clone)]
pub struct StreamSender<T> {
    stream_id: u16,
    socket: StreamSendSocket,
    // if the packet index overflows the worst that happens is a false positive packet loss
    next_packet_index: u32,
    _phantom: PhantomData<T>,
}

impl<T> StreamSender<T> {
    // The buffer is moved into the method. There is no way of reusing the same buffer twice without
    // extra copies/allocations
    pub async fn send_buffer(&mut self, mut buffer: SenderBuffer<T>) -> StrResult {
        buffer.inner[2..6].copy_from_slice(&self.next_packet_index.to_be_bytes());
        self.next_packet_index += 1;

        match &self.socket {
            StreamSendSocket::Udp(socket) => socket
                .inner
                .lock()
                .await
                .send((buffer.inner.freeze(), socket.peer_addr))
                .await
                .map_err(err!()),
            StreamSendSocket::Tcp(socket) => socket
                .lock()
                .await
                .send(buffer.inner.freeze())
                .await
                .map_err(err!()),
            StreamSendSocket::ThrottledUdp(socket) => {
                socket.send(buffer.inner.freeze()).await.map_err(err!())
            }
        }
    }
}

impl<T: Serialize> StreamSender<T> {
    pub fn new_buffer(
        &self,
        header: &T,
        preferred_max_buffer_size: usize,
    ) -> StrResult<SenderBuffer<T>> {
        let header_size = bincode::serialized_size(header).map_err(err!())?;
        // the first two bytes are for the stream ID
        let offset = 2 + 4 + header_size as usize;

        let mut buffer = BytesMut::with_capacity(offset + preferred_max_buffer_size);

        buffer.put_u16(self.stream_id);

        // make space for the packet index
        buffer.put_u32(0);

        let mut buffer_writer = buffer.writer();
        bincode::serialize_into(&mut buffer_writer, header).map_err(err!())?;
        let buffer = buffer_writer.into_inner();

        Ok(SenderBuffer {
            inner: buffer,
            offset,
            _phantom: PhantomData,
        })
    }

    pub async fn send(&mut self, packet: &T) -> StrResult {
        self.send_buffer(self.new_buffer(packet, 0)?).await
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
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> StreamReceiver<T> {
    pub async fn recv(&mut self) -> StrResult<ReceivedPacket<T>> {
        let mut bytes = match &mut self.receiver {
            StreamReceiverType::Queue(receiver) => receiver.recv().await.ok_or_else(enone!())?,
        };

        let packet_index = bytes.get_u32();
        let had_packet_loss = packet_index != self.next_packet_index;
        self.next_packet_index = packet_index + 1;

        let mut bytes_reader = bytes.reader();
        let header = bincode::deserialize_from(&mut bytes_reader).map_err(err!())?;
        let buffer = bytes_reader.into_inner();

        // At this point, "buffer" does not include the header anymore
        Ok(ReceivedPacket {
            header,
            buffer,
            had_packet_loss,
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
            _phantom: PhantomData,
        })
    }

    pub async fn subscribe_to_stream<T>(&self, stream_id: u16) -> StrResult<StreamReceiver<T>> {
        let (enqueuer, dequeuer) = mpsc::unbounded_channel();
        self.packet_queues.lock().await.insert(stream_id, enqueuer);

        Ok(StreamReceiver {
            receiver: StreamReceiverType::Queue(dequeuer),
            next_packet_index: 0,
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
