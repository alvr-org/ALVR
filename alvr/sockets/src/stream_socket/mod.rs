// Note: for StreamSocket, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate
//
// StreamSender and StreamReceiver endpoints allow for convenient conversion of the header to/from
// bytes while still handling the additional byte buffer with zero copies and extra allocations.

mod tcp;
mod udp;

use alvr_common::prelude::*;
use alvr_session::{SocketBufferSize, SocketProtocol};
use bytes::{Buf, BufMut, BytesMut};
use futures::SinkExt;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    marker::PhantomData,
    mem,
    net::IpAddr,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};
use tcp::{TcpStreamReceiveSocket, TcpStreamSendSocket};
use tokio::{net, runtime::Runtime};
use tokio::{
    sync::{mpsc, Mutex},
    time,
};
use udp::{UdpStreamReceiveSocket, UdpStreamSendSocket};

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

#[derive(Clone)]
pub struct StreamSender<T> {
    stream_id: u16,
    max_packet_size: usize,
    socket: StreamSendSocket,
    header_buffer: Vec<u8>,
    // if the packet index overflows the worst that happens is a false positive packet loss
    next_packet_index: u32,
    _phantom: PhantomData<T>,
}

impl<T: Serialize> StreamSender<T> {
    async fn send_buffer(&self, buffer: BytesMut) -> StrResult {
        match &self.socket {
            StreamSendSocket::Udp(socket) => socket
                .inner
                .lock()
                .await
                .feed((buffer.freeze(), socket.peer_addr))
                .await
                .map_err(err!()),
            StreamSendSocket::Tcp(socket) => socket
                .lock()
                .await
                .feed(buffer.freeze())
                .await
                .map_err(err!()),
        }
    }

    pub fn send(&mut self, runtime: &Runtime, header: &T, payload_buffer: Vec<u8>) -> StrResult {
        // packet layout:
        // [ 2B (stream ID) | 4B (packet index) | 4B (packet shard count) | 4B (shard index)]
        // this escluses length delimited coding, which is handled by the TCP backend
        const OFFSET: usize = 2 + 4 + 4 + 4;
        let max_shard_data_size = self.max_packet_size - OFFSET;

        let header_size = bincode::serialized_size(header).map_err(err!()).unwrap() as usize;
        self.header_buffer.clear();
        if self.header_buffer.capacity() < header_size {
            // If the buffer is empty, with this call we request a capacity of "header_size".
            self.header_buffer.reserve(header_size);
        }
        bincode::serialize_into(&mut self.header_buffer, header)
            .map_err(err!())
            .unwrap();
        let header_shards = self.header_buffer.chunks(max_shard_data_size);

        let payload_shards = payload_buffer.chunks(max_shard_data_size);

        let total_shards_count = payload_shards.len() + header_shards.len();
        let mut shards_buffer = BytesMut::with_capacity(
            header_size + payload_buffer.len() + total_shards_count * OFFSET,
        );

        for (shard_index, shard) in header_shards.chain(payload_shards).enumerate() {
            shards_buffer.put_u16(self.stream_id);
            shards_buffer.put_u32(self.next_packet_index);
            shards_buffer.put_u32(total_shards_count as _);
            shards_buffer.put_u32(shard_index as u32);
            shards_buffer.put_slice(shard);
            runtime.block_on(self.send_buffer(shards_buffer.split()))?;
        }

        match &self.socket {
            StreamSendSocket::Udp(socket) => runtime
                .block_on(async { socket.inner.lock().await.flush().await })
                .map_err(err!())?,

            StreamSendSocket::Tcp(socket) => runtime
                .block_on(async { socket.lock().await.flush().await })
                .map_err(err!())?,
        }

        self.next_packet_index += 1;

        Ok(())
    }
}

#[derive(Default)]
pub struct ReceiverBuffer<T> {
    inner: BytesMut,
    had_packet_loss: bool,
    _phantom: PhantomData<T>,
}

impl<T> ReceiverBuffer<T> {
    pub fn new() -> Self {
        Self {
            inner: BytesMut::new(),
            had_packet_loss: false,
            _phantom: PhantomData,
        }
    }

    pub fn had_packet_loss(&self) -> bool {
        self.had_packet_loss
    }
}

impl<T: DeserializeOwned> ReceiverBuffer<T> {
    pub fn get(&self) -> StrResult<(T, &[u8])> {
        let mut data: &[u8] = &self.inner;
        let header = bincode::deserialize_from(&mut data).map_err(err!())?;

        Ok((header, data))
    }
}

pub struct StreamReceiver<T> {
    receiver: mpsc::UnboundedReceiver<BytesMut>,
    next_packet_shards: HashMap<usize, BytesMut>,
    next_packet_shards_count: Option<usize>,
    next_packet_index: u32,
    _phantom: PhantomData<T>,
}

/// Get next packet reconstructing from shards. It can store at max shards from two packets; if the
/// reordering entropy is too high, packets will never be successfully reconstructed.
impl<T: DeserializeOwned> StreamReceiver<T> {
    pub async fn recv_buffer(&mut self, buffer: &mut ReceiverBuffer<T>) -> StrResult {
        buffer.had_packet_loss = false;

        loop {
            let current_packet_index = self.next_packet_index;
            self.next_packet_index += 1;

            let mut current_packet_shards =
                HashMap::with_capacity(self.next_packet_shards.capacity());
            mem::swap(&mut current_packet_shards, &mut self.next_packet_shards);

            let mut current_packet_shards_count = self.next_packet_shards_count.take();

            loop {
                if let Some(shards_count) = current_packet_shards_count {
                    if current_packet_shards.len() >= shards_count {
                        buffer.inner.clear();

                        for i in 0..shards_count {
                            if let Some(shard) = current_packet_shards.get(&i) {
                                buffer.inner.put_slice(shard);
                            } else {
                                error!("Cannot find shard with given index!");
                                buffer.had_packet_loss = true;

                                self.next_packet_shards.clear();

                                break;
                            }
                        }

                        return Ok(());
                    }
                }

                let mut shard = self.receiver.recv().await.ok_or_else(enone!())?;

                let shard_packet_index = shard.get_u32();
                let shards_count = shard.get_u32() as usize;
                let shard_index = shard.get_u32() as usize;

                if shard_packet_index == current_packet_index {
                    current_packet_shards.insert(shard_index, shard);
                    current_packet_shards_count = Some(shards_count);
                } else if shard_packet_index >= self.next_packet_index {
                    if shard_packet_index > self.next_packet_index {
                        self.next_packet_shards.clear();
                    }

                    self.next_packet_shards.insert(shard_index, shard);
                    self.next_packet_shards_count = Some(shards_count);
                    self.next_packet_index = shard_packet_index;

                    if shard_packet_index > self.next_packet_index
                        || self.next_packet_shards.len() == shards_count
                    {
                        debug!("Skipping to next packet. Signaling packet loss.");
                        buffer.had_packet_loss = true;
                        break;
                    }
                }
                // else: ignore old shard
            }
        }
    }

    pub async fn recv_header_only(&mut self) -> StrResult<T> {
        let mut buffer = ReceiverBuffer::new();
        self.recv_buffer(&mut buffer).await?;

        Ok(buffer.get()?.0)
    }
}

pub enum StreamSocketBuilder {
    Tcp(net::TcpListener),
    Udp(net::UdpSocket),
}

impl StreamSocketBuilder {
    pub fn listen_for_server(
        runtime: &Runtime,
        port: u16,
        stream_socket_config: SocketProtocol,
        send_buffer_bytes: SocketBufferSize,
        recv_buffer_bytes: SocketBufferSize,
    ) -> StrResult<Self> {
        Ok(match stream_socket_config {
            SocketProtocol::Udp => StreamSocketBuilder::Udp(runtime.block_on(udp::bind(
                port,
                send_buffer_bytes,
                recv_buffer_bytes,
            ))?),
            SocketProtocol::Tcp => StreamSocketBuilder::Tcp(runtime.block_on(tcp::bind(
                port,
                send_buffer_bytes,
                recv_buffer_bytes,
            ))?),
        })
    }

    pub fn accept_from_server(
        self,
        runtime: &Runtime,
        timeout: Duration,
        server_ip: IpAddr,
        port: u16,
        max_packet_size: usize,
    ) -> ConResult<StreamSocket> {
        let (send_socket, receive_socket) = match self {
            StreamSocketBuilder::Udp(socket) => {
                let (send_socket, receive_socket) =
                    udp::connect(socket, server_ip, port).map_err(to_con_e!())?;

                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            StreamSocketBuilder::Tcp(listener) => {
                let (send_socket, receive_socket) = runtime.block_on(async {
                    tokio::select! {
                        res = tcp::accept_from_server(listener, server_ip) => {
                            res.map_err(to_con_e!())
                        },
                        _ = time::sleep(timeout) => alvr_common::timeout(),
                    }
                })?;

                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
        };

        Ok(StreamSocket {
            max_packet_size,
            send_socket,
            receive_socket: Arc::new(Mutex::new(Some(receive_socket))),
            packet_queues: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn connect_to_client(
        runtime: &Runtime,
        timeout: Duration,
        client_ip: IpAddr,
        port: u16,
        protocol: SocketProtocol,
        send_buffer_bytes: SocketBufferSize,
        recv_buffer_bytes: SocketBufferSize,
        max_packet_size: usize,
    ) -> ConResult<StreamSocket> {
        let (send_socket, receive_socket) = match protocol {
            SocketProtocol::Udp => {
                let socket = runtime
                    .block_on(udp::bind(port, send_buffer_bytes, recv_buffer_bytes))
                    .map_err(to_con_e!())?;
                let (send_socket, receive_socket) =
                    udp::connect(socket, client_ip, port).map_err(to_con_e!())?;
                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            SocketProtocol::Tcp => {
                let (send_socket, receive_socket) = runtime.block_on(async {
                    tokio::select! {
                        res = tcp::connect_to_client(client_ip, port, send_buffer_bytes, recv_buffer_bytes) => {
                            res.map_err(to_con_e!())
                        },
                        _ = time::sleep(timeout) => alvr_common::timeout(),
                    }
                })?;

                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
        };

        Ok(StreamSocket {
            max_packet_size,
            send_socket,
            receive_socket: Arc::new(Mutex::new(Some(receive_socket))),
            packet_queues: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

pub struct StreamSocket {
    max_packet_size: usize,
    send_socket: StreamSendSocket,
    receive_socket: Arc<Mutex<Option<StreamReceiveSocket>>>,
    packet_queues: Arc<Mutex<HashMap<u16, mpsc::UnboundedSender<BytesMut>>>>,
}

impl StreamSocket {
    pub fn request_stream<T>(&self, stream_id: u16) -> StreamSender<T> {
        StreamSender {
            stream_id,
            max_packet_size: self.max_packet_size,
            socket: self.send_socket.clone(),
            header_buffer: vec![],
            next_packet_index: 0,
            _phantom: PhantomData,
        }
    }

    pub fn subscribe_to_stream<T>(&self, runtime: &Runtime, stream_id: u16) -> StreamReceiver<T> {
        let (sender, receiver) = mpsc::unbounded_channel();
        runtime
            .block_on(self.packet_queues.lock())
            .insert(stream_id, sender);

        StreamReceiver {
            receiver,
            next_packet_shards: HashMap::new(),
            next_packet_shards_count: None,
            next_packet_index: 0,
            _phantom: PhantomData,
        }
    }

    pub fn recv(&self, runtime: &Runtime, timeout: Duration) -> ConResult {
        match runtime
            .block_on(self.receive_socket.lock())
            .as_mut()
            .unwrap()
        {
            StreamReceiveSocket::Udp(socket) => runtime.block_on(async {
                tokio::select! {
                    res = udp::recv(socket, &self.packet_queues) => res.map_err(to_con_e!()),
                    _ = time::sleep(timeout) => alvr_common::timeout(),
                }
            }),
            StreamReceiveSocket::Tcp(socket) => runtime.block_on(async {
                tokio::select! {
                    res = tcp::recv(socket, Arc::clone(&self.packet_queues)) => res.map_err(to_con_e!()),
                    _ = time::sleep(timeout) => alvr_common::timeout(),
                }
            }),
        }
    }
}
