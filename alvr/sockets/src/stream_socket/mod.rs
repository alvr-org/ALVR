// Note: for StreamSocket, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate
//
// StreamSender and StreamReceiver endpoints allow for convenient conversion of the header to/from
// bytes while still handling the additional byte buffer with zero copies and extra allocations.

mod tcp;
mod udp;

use alvr_common::{anyhow::Result, con_bail, debug, error, info, AnyhowToCon, ConResult};
use alvr_session::{SocketBufferSize, SocketProtocol};
use bytes::{Buf, BufMut, BytesMut};
use futures::SinkExt;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
    net::IpAddr,
    ops::{Deref, DerefMut},
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};
use tcp::{TcpStreamReceiveSocket, TcpStreamSendSocket};
use tokio::{net, runtime::Runtime};
use udp::{UdpStreamReceiveSocket, UdpStreamSendSocket};

pub fn set_socket_buffers(
    socket: &socket2::Socket,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<()> {
    info!(
        "Initial socket buffer size: send: {}B, recv: {}B",
        socket.send_buffer_size()?,
        socket.recv_buffer_size()?
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
                    socket.send_buffer_size()?
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
                    socket.recv_buffer_size()?
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
    fn send_buffer(&self, runtime: &Runtime, buffer: BytesMut) -> Result<()> {
        match &self.socket {
            StreamSendSocket::Udp(socket) => Ok(runtime.block_on(
                socket
                    .inner
                    .lock()
                    .feed((buffer.freeze(), socket.peer_addr)),
            )?),
            StreamSendSocket::Tcp(socket) => {
                Ok(runtime.block_on(socket.lock().feed(buffer.freeze()))?)
            }
        }
    }

    pub fn send(&mut self, runtime: &Runtime, header: &T, payload_buffer: Vec<u8>) -> Result<()> {
        // packet layout:
        // [ 2B (stream ID) | 4B (packet index) | 4B (packet shard count) | 4B (shard index)]
        // this escluses length delimited coding, which is handled by the TCP backend
        const OFFSET: usize = 2 + 4 + 4 + 4;
        let max_shard_data_size = self.max_packet_size - OFFSET;

        let header_size = bincode::serialized_size(header).unwrap() as usize;
        self.header_buffer.clear();
        if self.header_buffer.capacity() < header_size {
            // If the buffer is empty, with this call we request a capacity of "header_size".
            self.header_buffer.reserve(header_size);
        }
        bincode::serialize_into(&mut self.header_buffer, header).unwrap();
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
            self.send_buffer(runtime, shards_buffer.split())?;
        }

        match &self.socket {
            StreamSendSocket::Udp(socket) => runtime.block_on(socket.inner.lock().flush())?,
            StreamSendSocket::Tcp(socket) => runtime.block_on(socket.lock().flush())?,
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
    pub fn get(&self) -> Result<(T, &[u8])> {
        let mut data: &[u8] = &self.inner;
        let header = bincode::deserialize_from(&mut data)?;

        Ok((header, data))
    }
}

pub struct StreamReceiver<T> {
    receiver: mpsc::Receiver<BytesMut>,
    last_reconstructed_packet_index: u32,
    packet_shards: BTreeMap<u32, HashMap<usize, BytesMut>>,
    empty_shard_maps: Vec<HashMap<usize, BytesMut>>,
    _phantom: PhantomData<T>,
}

/// Get next packet reconstructing from shards.
/// Returns true if a packet has been recontructed and copied into the buffer.
impl<T: DeserializeOwned> StreamReceiver<T> {
    pub fn recv_buffer(
        &mut self,
        timeout: Duration,
        buffer: &mut ReceiverBuffer<T>,
    ) -> ConResult<bool> {
        // Get shard
        let mut shard = match self.receiver.recv_timeout(timeout) {
            Ok(shard) => Ok(shard),
            Err(RecvTimeoutError::Timeout) => alvr_common::timeout(),
            Err(RecvTimeoutError::Disconnected) => con_bail!("Disconnected"),
        }?;
        let shard_packet_index = shard.get_u32();
        let shards_count = shard.get_u32() as usize;
        let shard_index = shard.get_u32() as usize;

        // Discard shard if too old
        if shard_packet_index <= self.last_reconstructed_packet_index {
            debug!("Received old shard!");
            return Ok(false);
        }

        // Insert shards into map
        let shard_map = self
            .packet_shards
            .entry(shard_packet_index)
            .or_insert_with(|| self.empty_shard_maps.pop().unwrap_or_default());
        shard_map.insert(shard_index, shard);

        // If the shard map is (probably) complete:
        if shard_map.len() == shards_count {
            buffer.inner.clear();

            // Copy shards into final buffer. Fail if there are missing shards. This is impossibly
            // rare (if the shards_count value got corrupted) but should be handled.
            for idx in 0..shards_count {
                if let Some(shard) = shard_map.get(&idx) {
                    buffer.inner.put_slice(shard);
                } else {
                    error!("Cannot find shard with given index!");
                    return Ok(false);
                }
            }

            // Check if current packet index is one up the last successful reconstucted packet.
            buffer.had_packet_loss = shard_packet_index != self.last_reconstructed_packet_index + 1;
            self.last_reconstructed_packet_index = shard_packet_index;

            // Pop old shards and recycle containers
            while let Some((packet_index, mut shards)) = self.packet_shards.pop_first() {
                shards.clear();
                self.empty_shard_maps.push(shards);

                if packet_index == shard_packet_index {
                    break;
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn recv_header_only(&mut self, timeout: Duration) -> ConResult<T> {
        let mut buffer = ReceiverBuffer::new();

        loop {
            if self.recv_buffer(timeout, &mut buffer)? {
                return Ok(buffer.get().to_con()?.0);
            }
        }
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
    ) -> Result<Self> {
        Ok(match stream_socket_config {
            SocketProtocol::Udp => StreamSocketBuilder::Udp(udp::bind(
                runtime,
                port,
                send_buffer_bytes,
                recv_buffer_bytes,
            )?),
            SocketProtocol::Tcp => StreamSocketBuilder::Tcp(tcp::bind(
                runtime,
                port,
                send_buffer_bytes,
                recv_buffer_bytes,
            )?),
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
                let (send_socket, receive_socket) = udp::connect(socket, server_ip, port);

                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            StreamSocketBuilder::Tcp(listener) => {
                let (send_socket, receive_socket) =
                    tcp::accept_from_server(runtime, timeout, listener, server_ip)?;

                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
        };

        Ok(StreamSocket {
            max_packet_size,
            send_socket,
            receive_socket,
            packet_queues: HashMap::new(),
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
                let socket =
                    udp::bind(runtime, port, send_buffer_bytes, recv_buffer_bytes).to_con()?;
                let (send_socket, receive_socket) = udp::connect(socket, client_ip, port);

                (
                    StreamSendSocket::Udp(send_socket),
                    StreamReceiveSocket::Udp(receive_socket),
                )
            }
            SocketProtocol::Tcp => {
                let (send_socket, receive_socket) = tcp::connect_to_client(
                    runtime,
                    timeout,
                    client_ip,
                    port,
                    send_buffer_bytes,
                    recv_buffer_bytes,
                )?;

                (
                    StreamSendSocket::Tcp(send_socket),
                    StreamReceiveSocket::Tcp(receive_socket),
                )
            }
        };

        Ok(StreamSocket {
            max_packet_size,
            send_socket,
            receive_socket,
            packet_queues: HashMap::new(),
        })
    }
}

pub struct StreamSocket {
    max_packet_size: usize,
    send_socket: StreamSendSocket,
    receive_socket: StreamReceiveSocket,
    packet_queues: HashMap<u16, mpsc::Sender<BytesMut>>,
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

    pub fn subscribe_to_stream<T>(&mut self, stream_id: u16) -> StreamReceiver<T> {
        let (sender, receiver) = mpsc::channel();

        self.packet_queues.insert(stream_id, sender);

        StreamReceiver {
            receiver,
            last_reconstructed_packet_index: 0,
            packet_shards: BTreeMap::new(),
            empty_shard_maps: vec![],
            _phantom: PhantomData,
        }
    }

    pub fn recv(&mut self, runtime: &Runtime, timeout: Duration) -> ConResult {
        match &mut self.receive_socket {
            StreamReceiveSocket::Udp(socket) => {
                udp::recv(runtime, timeout, socket, &mut self.packet_queues)
            }
            StreamReceiveSocket::Tcp(socket) => {
                tcp::recv(runtime, timeout, socket, &mut self.packet_queues)
            }
        }
    }
}
