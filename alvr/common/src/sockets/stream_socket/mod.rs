// Note: for StreamSocket, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate
//
// StreamSender and StreamReceiver endpoints allow for convenient conversion of the header to/from
// bytes while still handling the additional byte buffer with zero copies and extra allocations.

mod tcp;
mod udp;

use crate::{data::*, *};
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
use tokio::sync::{mpsc, Mutex};
use udp::{UdpStreamReceiveSocket, UdpStreamSendSocket};

// todo: when min_const_generics reaches stable, use this as a const generic parameter
// todo: when const_generics reaches stable, convert this to an enum
pub type StreamId = u8;
pub const AUDIO: StreamId = 0;
pub const LEGACY: StreamId = 1;
pub const RESERVED: StreamId = 2;

#[derive(Clone)]
enum StreamSendSocket {
    Udp(UdpStreamSendSocket),
    Tcp(TcpStreamSendSocket),
}

enum StreamReceiveSocket {
    Udp(UdpStreamReceiveSocket),
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

pub struct StreamSender<T> {
    socket: StreamSendSocket,
    stream_id: StreamId,
    _phantom: PhantomData<T>,
}

impl<T> StreamSender<T> {
    // The buffer is moved into the method. There is no way of reusing the same buffer twice without
    // extra copies/allocations
    pub async fn send_buffer(&self, buffer: SenderBuffer<T>) -> StrResult {
        match &self.socket {
            StreamSendSocket::Udp(socket) => trace_err!(
                socket
                    .inner
                    .lock()
                    .await
                    .send((buffer.inner.freeze(), socket.peer_addr))
                    .await
            ),
            StreamSendSocket::Tcp(socket) => {
                trace_err!(socket.lock().await.send(buffer.inner.freeze()).await)
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
        let header_size = trace_err!(bincode::serialized_size(header))?;
        // the first byte is for the stream ID
        let offset = 1 + header_size as usize;

        let mut buffer = BytesMut::with_capacity(offset + preferred_max_buffer_size);

        buffer.put_u8(self.stream_id);
        let mut buffer_writer = buffer.writer();
        trace_err!(bincode::serialize_into(&mut buffer_writer, header))?;
        let buffer = buffer_writer.into_inner();

        Ok(SenderBuffer {
            inner: buffer,
            offset,
            _phantom: PhantomData,
        })
    }

    pub async fn send(&self, packet: &T) -> StrResult {
        self.send_buffer(self.new_buffer(packet, 0)?).await
    }
}

enum StreamReceiverType {
    Queue(mpsc::UnboundedReceiver<BytesMut>),
    // QuickReliable(...)
}

pub struct StreamReceiver<T> {
    receiver: StreamReceiverType,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> StreamReceiver<T> {
    pub async fn recv_buffer(&mut self) -> StrResult<(T, BytesMut)> {
        let mut bytes = match &mut self.receiver {
            StreamReceiverType::Queue(receiver) => trace_none!(receiver.recv().await)?,
        };

        // pop the stream ID
        bytes.get_u8();

        let mut bytes_reader = bytes.reader();
        let header = trace_err!(bincode::deserialize_from(&mut bytes_reader))?;
        let bytes = bytes_reader.into_inner();

        // At this point, bytes does not include the header anymore
        Ok((header, bytes))
    }

    pub async fn recv(&mut self) -> StrResult<T> {
        Ok(self.recv_buffer().await?.0)
    }
}

pub struct StreamSocket {
    send_socket: StreamSendSocket,
    receive_socket: StreamReceiveSocket,
    packet_queues: Arc<Mutex<HashMap<StreamId, mpsc::UnboundedSender<BytesMut>>>>,
}

impl StreamSocket {
    pub async fn request_stream<T>(&self, stream_id: StreamId) -> StrResult<StreamSender<T>> {
        Ok(StreamSender {
            socket: self.send_socket.clone(),
            stream_id,
            _phantom: PhantomData,
        })
    }

    pub async fn subscribe_to_stream<T>(
        &mut self,
        stream_id: StreamId,
    ) -> StrResult<StreamReceiver<T>> {
        let receiver_type = match &mut self.send_socket {
            StreamSendSocket::Udp { .. } | StreamSendSocket::Tcp(_) => {
                let (enqueuer, dequeuer) = mpsc::unbounded_channel();
                self.packet_queues.lock().await.insert(stream_id, enqueuer);

                StreamReceiverType::Queue(dequeuer)
            }
        };

        Ok(StreamReceiver {
            receiver: receiver_type,
            _phantom: PhantomData,
        })
    }

    pub async fn connect_to_server(
        server_ip: IpAddr,
        port: u16,
        stream_socket_config: SocketConfig,
    ) -> StrResult<Self> {
        let (send_socket, receive_socket) = match stream_socket_config {
            SocketConfig::Udp => {
                let (send_socket, receive_socket) = udp::connect(server_ip, port).await?;
                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            SocketConfig::Tcp => {
                let (send_socket, receive_socket) = tcp::connect_to_server(server_ip, port).await?;
                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
        };

        Ok(Self {
            send_socket,
            receive_socket,
            packet_queues: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn connect_to_client(
        client_ip: IpAddr,
        port: u16,
        stream_socket_config: SocketConfig,
    ) -> StrResult<Self> {
        let (send_socket, receive_socket) = match stream_socket_config {
            SocketConfig::Udp => {
                let (send_socket, receive_socket) = udp::connect(client_ip, port).await?;
                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            SocketConfig::Tcp => {
                let (send_socket, receive_socket) = tcp::connect_to_client(client_ip, port).await?;
                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
        };

        Ok(Self {
            send_socket,
            receive_socket,
            packet_queues: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn receive_loop(self) -> StrResult {
        match self.receive_socket {
            StreamReceiveSocket::Udp(socket) => udp::receive_loop(socket, self.packet_queues).await,
            StreamReceiveSocket::Tcp(socket) => tcp::receive_loop(socket, self.packet_queues).await,
        }
    }
}
