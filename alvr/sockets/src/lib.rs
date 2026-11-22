// Note: for SocketConnection, the client uses a server socket, the server uses a client socket.
// This is because the client doesn not have a UI for trusting a server.

// Performance analysis:
// We want to minimize the transmission time for various sizes of packets.
// The current code locks the write socket *per shard* and not *per packet*. This leds to the best
// performance outcome given that the possible packets can be either very small (one shard) or very
// large (hundreds/thousands of shards, for video). if we don't allow interleaving shards, a very
// small packet will need to wait a long time before getting received if there was an ongoing
// transmission of a big packet before. If we allow interleaving shards, small packets can be
// transmitted quicker, with only minimal latency increase for the ongoing transmission of the big
// packet.
// Note: We can't clone the underlying socket for each StreamSender and the mutex around the socket
// cannot be removed. This is because we need to make sure at least shards are written whole.

mod handshake;
mod tcp;
mod udp;

use alvr_common::{
    AnyhowToCon, ConResult, ConnectionError, HandleTryAgain, ToCon, anyhow::Result, con_bail, info,
    warn,
};
use alvr_packets::{ClientControlPacket, ServerControlPacket};
use alvr_session::{DscpTos, SocketBufferConfig, SocketBufferSize, SocketTransport};
use bincode::config;
use serde::{Serialize, de::DeserializeOwned};
use socket2::Socket;
use std::{
    cmp::Ordering,
    collections::HashMap,
    marker::PhantomData,
    mem,
    net::{IpAddr, Ipv4Addr, TcpListener, TcpStream, UdpSocket},
    ops::{Deref, DerefMut},
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

pub use handshake::*;

pub const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
pub const SERVER_PORT: u16 = 9943;
pub const CLIENT_PORT: u16 = 9944;
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(500);
pub const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(2);

pub const MDNS_SERVICE_TYPE: &str = "_alvr._tcp.local.";
pub const MDNS_PROTOCOL_KEY: &str = "protocol";
pub const MDNS_DEVICE_ID_KEY: &str = "device_id";

pub const WIRED_CLIENT_HOSTNAME: &str = "client.wired";

const SHARD_COMMON_PREFIX_SIZE: usize = mem::size_of::<u16>() // stream ID
    + mem::size_of::<u32>() // packet index
    + mem::size_of::<u32>() // shards count
    + mem::size_of::<u32>(); // shards index

fn configure_socket(socket: &Socket, config: &SocketConnectionConfig) -> Result<()> {
    info!(
        "Initial socket buffer size: send: {}B, recv: {}B",
        socket.send_buffer_size()?,
        socket.recv_buffer_size()?
    );

    {
        let maybe_size = match config.buffer_config.send_size_bytes {
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
        let maybe_size = match config.buffer_config.recv_size_bytes {
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

    // https://en.wikipedia.org/wiki/Differentiated_services
    if let Some(dscp) = config.dscp {
        let tos = match dscp {
            DscpTos::BestEffort => 0,
            DscpTos::ClassSelector(precedence) => precedence << 3,
            DscpTos::AssuredForwarding {
                class,
                drop_probability,
            } => (class << 3) | drop_probability as u8,
            DscpTos::ExpeditedForwarding => 0b101110,
        };

        socket.set_tos_v4((tos << 2) as u32).ok();
    }

    socket.set_read_timeout(Some(config.timeout))?;

    Ok(())
}

// connect_to_client should be used on the server side.
// At the moment, the TcpListener is implemened on the client side so the API for this function
// is non-standard
// todo: convert to class when storing a TcpListener
pub fn connect_to_client<T: DeserializeOwned>(
    client_ips: Vec<IpAddr>,
    timeout: Duration,
) -> ConResult<(HandshakeConnection, IpAddr, T)> {
    let (mut handshake_connection, client_ip) =
        HandshakeConnection::connect_to(timeout, PeerType::AnyClient(client_ips))?;

    let res = handshake_connection.recv(timeout)?;

    Ok((handshake_connection, client_ip, res))
}

pub fn listen_to_server<T: DeserializeOwned>(
    listener_socket: &TcpListener,
    timeout: Duration,
    client_info: &impl Serialize,
) -> ConResult<(HandshakeConnection, T)> {
    let (mut handshake_connection, _) =
        HandshakeConnection::connect_to(timeout, PeerType::Server(listener_socket))?;

    handshake_connection.send(client_info).to_con()?;

    let config_packet = handshake_connection.recv(timeout)?;

    Ok((handshake_connection, config_packet))
}

pub fn send_restart_signal(
    mut handshake_connection: HandshakeConnection,
    stream_config_packet: impl Serialize,
) -> ConResult<()> {
    // We must send the config packet before, which will be unused
    handshake_connection.send(&stream_config_packet).to_con()?;

    handshake_connection
        .send(&ServerControlPacket::Restarting)
        .to_con()
}

struct ShardPrefix {
    stream_id: u16,
    packet_index: u32,
    shards_count: usize,
    shard_index: usize,
}

impl ShardPrefix {
    fn read(bytes: &[u8]) -> Self {
        Self {
            stream_id: u16::from_le_bytes(bytes[0..2].try_into().unwrap()),
            packet_index: u32::from_le_bytes(bytes[2..6].try_into().unwrap()),
            shards_count: u32::from_le_bytes(bytes[6..10].try_into().unwrap()) as usize,
            shard_index: u32::from_le_bytes(bytes[10..14].try_into().unwrap()) as usize,
        }
    }

    fn write(&self, bytes: &mut [u8]) {
        bytes[0..2].copy_from_slice(&self.stream_id.to_le_bytes());
        bytes[2..6].copy_from_slice(&self.packet_index.to_le_bytes());
        bytes[6..10].copy_from_slice(&(self.shards_count as u32).to_le_bytes());
        bytes[10..14].copy_from_slice(&(self.shard_index as u32).to_le_bytes());
    }
}

trait MultiplexedSocketWriter {
    // Note: consts are not trait-safe, we require a method
    fn payload_offset(&self) -> usize;

    fn send(&self, stream_id: u16, packet_index: u32, buffer: &mut Vec<u8>) -> Result<()>;
}

struct ReconstructedPacket {
    index: u32,
    buffer: Vec<u8>,
}

struct StreamRecvQueues {
    used_buffer_sender: mpsc::Sender<Vec<u8>>,
    used_buffer_receiver: mpsc::Receiver<Vec<u8>>,
    packet_queue: mpsc::Sender<ReconstructedPacket>,
}

trait MultiplexedSocketReader {
    fn payload_offset(&self) -> usize;

    fn recv(&mut self, stream_queues: &HashMap<u16, StreamRecvQueues>) -> ConResult;
}

/// Memory buffer that contains a hidden prefix
#[derive(Default)]
pub struct Buffer<H = ()> {
    inner: Vec<u8>,
    raw_payload_offset: usize, // this corresponds to prefix + header
    _phantom: PhantomData<H>,
}

impl<H> Deref for Buffer<H> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.inner[self.raw_payload_offset..]
    }
}

impl<H> DerefMut for Buffer<H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner[self.raw_payload_offset..]
    }
}

#[derive(Clone)]
pub struct StreamSender<H> {
    inner: Arc<dyn MultiplexedSocketWriter + Send + Sync>,
    stream_id: u16,
    payload_offset: usize,
    next_packet_index: u32,
    used_buffers: Vec<Vec<u8>>,
    _phantom: PhantomData<H>,
}

impl<H> StreamSender<H> {
    /// Shard and send a buffer with zero copies and zero allocations.
    /// The prefix of each shard is written over the previously sent shard to avoid reallocations.
    pub fn send(&mut self, mut buffer: Buffer<H>) -> Result<()> {
        self.inner
            .send(self.stream_id, self.next_packet_index, &mut buffer.inner)?;

        self.used_buffers.push(buffer.inner);

        self.next_packet_index = self.next_packet_index.wrapping_add(1);

        Ok(())
    }
}

impl<H: Serialize> StreamSender<H> {
    pub fn get_buffer(&mut self, header: &H, raw_payload_len: usize) -> Result<Buffer<H>> {
        let mut buffer = self.used_buffers.pop().unwrap_or_default();

        buffer.resize(self.payload_offset, 0);

        let encoded_payload_size =
            bincode::serde::encode_into_std_write(header, &mut buffer, config::standard())?;

        let raw_payload_offset = self.payload_offset + encoded_payload_size;

        buffer.resize(raw_payload_offset + raw_payload_len, 0);

        Ok(Buffer {
            inner: buffer,
            raw_payload_offset,
            _phantom: PhantomData,
        })
    }

    pub fn send_header_with_payload(&mut self, header: &H, raw_payload: &[u8]) -> Result<()> {
        let mut buffer = self.get_buffer(header, raw_payload.len())?;
        buffer.copy_from_slice(raw_payload);
        self.send(buffer)
    }

    pub fn send_header(&mut self, header: &H) -> Result<()> {
        self.send_header_with_payload(header, &[])
    }
}

pub struct ReceiverData<H> {
    buffer: Vec<u8>,
    payload_offset: usize,
    used_buffer_queue: mpsc::Sender<Vec<u8>>,
    had_packet_loss: bool,
    _phantom: PhantomData<H>,
}

impl<H> ReceiverData<H> {
    pub fn had_packet_loss(&self) -> bool {
        self.had_packet_loss
    }
}

impl<H: DeserializeOwned> ReceiverData<H> {
    pub fn get(&self) -> Result<(H, &[u8])> {
        let payload = &self.buffer[self.payload_offset..];

        let (header, decoded_size) =
            bincode::serde::decode_from_slice(payload, config::standard())?;

        Ok((header, &payload[decoded_size..]))
    }

    pub fn get_header(&self) -> Result<H> {
        Ok(self.get()?.0)
    }
}

impl<H> Drop for ReceiverData<H> {
    fn drop(&mut self) {
        self.used_buffer_queue
            .send(mem::take(&mut self.buffer))
            .ok();
    }
}

pub struct StreamReceiver<H> {
    payload_offset: usize,
    packet_receiver: mpsc::Receiver<ReconstructedPacket>,
    used_buffer_queue: mpsc::Sender<Vec<u8>>,
    last_packet_index: Option<u32>,
    _phantom: PhantomData<H>,
}

// `buffer` contains the payload offset by `prefix_size`
fn send_sharded(
    buffer: &mut [u8],
    prefix_size: usize,
    max_packet_size: usize,
    stream_id: u16,
    packet_index: u32,
    mut send_shard: impl FnMut(&mut [u8]) -> Result<()>,
) -> Result<()> {
    let max_shard_size = max_packet_size - prefix_size;
    let payload_size = buffer.len() - prefix_size;
    // rounding up:
    let shards_count = usize::max(payload_size.div_ceil(max_shard_size), 1);

    for shard_idx in 0..shards_count {
        // this overlaps with the previous shard, this is intended behavior and allows to
        // reduce allocations
        let shard_start_position = shard_idx * max_shard_size;
        let shard_size = usize::min(max_shard_size, payload_size - shard_start_position);

        let shard_view = &mut buffer[shard_start_position..][..prefix_size + shard_size];

        ShardPrefix {
            stream_id,
            packet_index,
            shards_count,
            shard_index: shard_idx,
        }
        .write(shard_view);

        send_shard(shard_view)?;
    }

    Ok(())
}

// Wrapping comparison for packet indices.
// Problem: packet indices have a finite bit-width and we have to wrap around upon reaching the
// maximum value. This function provides proper ordering capability when wrapping around. The
// maximum index distance that two packets can have is u32::MAX / 2 (which is plenty for any
// reasonable circumstance).
fn wrapping_cmp(lhs: u32, rhs: u32) -> Ordering {
    const LOWER_BOUND: u32 = u32::MAX / 4;
    const UPPER_BOUND: u32 = u32::MAX - LOWER_BOUND;

    if lhs < LOWER_BOUND && rhs > UPPER_BOUND {
        // lhs wrapped around, so its "unwrapped" value would be `lhs as u64 + u32::MAX`
        Ordering::Greater
    } else if lhs > UPPER_BOUND && rhs < LOWER_BOUND {
        // rhs wrapped around
        Ordering::Less
    } else {
        lhs.cmp(&rhs)
    }
}

impl<H: DeserializeOwned + Serialize> StreamReceiver<H> {
    pub fn recv(&mut self, timeout: Duration) -> ConResult<ReceiverData<H>> {
        let packet = self
            .packet_receiver
            .recv_timeout(timeout)
            .handle_try_again()?;

        let mut had_packet_loss = false;

        if let Some(last_idx) = self.last_packet_index {
            // Use wrapping arithmetics
            match wrapping_cmp(packet.index, last_idx.wrapping_add(1)) {
                Ordering::Equal => (),
                Ordering::Greater => {
                    // Skipped some indices
                    had_packet_loss = true
                }
                Ordering::Less => {
                    // Old packet, discard
                    self.used_buffer_queue.send(packet.buffer).to_con()?;
                    return alvr_common::try_again();
                }
            }
        }
        self.last_packet_index = Some(packet.index);

        Ok(ReceiverData {
            buffer: packet.buffer,
            payload_offset: self.payload_offset,
            used_buffer_queue: self.used_buffer_queue.clone(),
            had_packet_loss,
            _phantom: PhantomData,
        })
    }

    pub fn recv_header(&mut self, timeout: Duration) -> ConResult<H> {
        self.recv(timeout)?.get_header().to_con()
    }
}

pub struct SocketPoller {
    receive_socket: Box<dyn MultiplexedSocketReader + Send>,
    queues: HashMap<u16, StreamRecvQueues>,
}

impl SocketPoller {
    pub fn poll(&mut self) -> ConResult {
        self.receive_socket.recv(&self.queues)
    }
}

pub struct SocketConnectionConfig {
    pub timeout: Duration,
    pub buffer_config: SocketBufferConfig,
    pub max_packet_size: usize,
    pub dscp: Option<DscpTos>,
}

pub enum ServerConnectionResult {
    Connected(SocketConnection),
    Restarting,
}

pub struct SocketConnection {
    tcp_writer: Arc<tcp::MultiplexedTcpWriter>,
    tcp_poller: SocketPoller,
    maybe_udp_writer: Option<Arc<udp::MultiplexedUdpWriter>>,
    maybe_udp_poller: Option<SocketPoller>,
}

impl SocketConnection {
    fn new(
        tcp_stream: TcpStream,
        maybe_udp_socket: Option<UdpSocket>,
        config: &SocketConnectionConfig,
    ) -> ConResult<Self> {
        let (tcp_writer, tcp_reader) = tcp::split_multiplexed(tcp_stream, config).to_con()?;

        let (maybe_udp_writer, maybe_udp_poller) = match maybe_udp_socket {
            Some(socket) => {
                let (udp_writer, udp_reader) = udp::split_multiplexed(socket, config).to_con()?;

                (
                    Some(Arc::new(udp_writer)),
                    Some(SocketPoller {
                        receive_socket: Box::new(udp_reader),
                        queues: HashMap::new(),
                    }),
                )
            }
            None => (None, None),
        };

        Ok(Self {
            tcp_writer: Arc::new(tcp_writer),
            tcp_poller: SocketPoller {
                receive_socket: Box::new(tcp_reader),
                queues: HashMap::new(),
            },
            maybe_udp_writer,
            maybe_udp_poller,
        })
    }

    // Note: the timeout resets after each internal operation
    pub fn from_client_connection(
        mut handshake_connection: HandshakeConnection,
        stream_config_packet: impl Serialize,
        config: SocketConnectionConfig,
    ) -> ConResult<Self> {
        let client_ip = handshake_connection.inner.peer_addr().to_con()?.ip();

        handshake_connection.send(&stream_config_packet).to_con()?;

        handshake_connection
            .send(&ServerControlPacket::StartStream)
            .to_con()?;

        let signal = handshake_connection.recv(config.timeout)?;
        if !matches!(signal, ClientControlPacket::StreamReady) {
            con_bail!("Got unexpected packet waiting for stream ack");
        }

        let maybe_udp_socket = match udp::bind(SERVER_PORT, client_ip, CLIENT_PORT) {
            Ok(socket) => {
                if udp::probe_peer(&socket, config.timeout).to_con()? {
                    Some(socket)
                } else {
                    warn!("UDP handshake timed out, falling back to TCP");

                    None
                }
            }
            Err(e) => {
                warn!("Failed to create UDP socket, falling back to TCP: {e}");

                None
            }
        };

        let udp_available = maybe_udp_socket.is_some();
        handshake_connection.send(&udp_available).to_con()?;

        Self::new(handshake_connection.inner, maybe_udp_socket, &config)
    }

    // Note: the timeout resets after each internal operation
    pub fn from_server_connection(
        mut handshake_connection: HandshakeConnection,
        config: SocketConnectionConfig,
    ) -> ConResult<ServerConnectionResult> {
        let server_ip = handshake_connection.inner.peer_addr().to_con()?.ip();

        match handshake_connection.recv(config.timeout)? {
            ServerControlPacket::StartStream => (),
            ServerControlPacket::Restarting => return Ok(ServerConnectionResult::Restarting),
            _ => con_bail!("Got unexpected packet waiting for stream start"),
        }

        let maybe_udp_socket = udp::bind(CLIENT_PORT, server_ip, SERVER_PORT)
            .inspect_err(|e| warn!("Failed to create UDP socket, falling back to TCP: {e}"))
            .ok();

        handshake_connection
            .send(&ClientControlPacket::StreamReady)
            .to_con()?;

        if let Some(socket) = &maybe_udp_socket {
            udp::answer_probes(socket, config.timeout).to_con()?;
        }

        let deadline = Instant::now() + config.timeout * 2;
        let udp_available = loop {
            match handshake_connection.recv::<bool>(config.timeout) {
                Err(ConnectionError::TryAgain(_)) if Instant::now() < deadline => continue,
                res => break res?,
            }
        };

        let maybe_udp_socket = if udp_available {
            if maybe_udp_socket.is_none() {
                con_bail!("Server selected UDP but the client UDP socket is unavailable");
            }

            maybe_udp_socket
        } else {
            warn!("UDP handshake failed, falling back to TCP");

            None
        };

        let connection = Self::new(handshake_connection.inner, maybe_udp_socket, &config)?;

        Ok(ServerConnectionResult::Connected(connection))
    }

    pub fn request_stream<T>(
        &self,
        stream_id: u16,
        preferred_transport: SocketTransport,
    ) -> StreamSender<T> {
        let writer: Arc<dyn MultiplexedSocketWriter + Send + Sync> =
            match (preferred_transport, &self.maybe_udp_writer) {
                (SocketTransport::Udp, Some(udp_writer)) => udp_writer.clone(),
                _ => self.tcp_writer.clone(),
            };

        StreamSender {
            payload_offset: writer.payload_offset(),
            inner: writer,
            stream_id,
            next_packet_index: 0,
            used_buffers: vec![],
            _phantom: PhantomData,
        }
    }

    // max_concurrent_buffers: number of buffers allocated by this call which will be reused to
    // receive packets for this stream ID. If packets are not read fast enough, the shards received
    // for this particular stream will be discarded
    pub fn subscribe_to_stream<T>(
        &mut self,
        stream_id: u16,
        preferred_transport: SocketTransport,
        max_concurrent_buffers: usize,
    ) -> StreamReceiver<T> {
        let poller = match (preferred_transport, &mut self.maybe_udp_poller) {
            (SocketTransport::Udp, Some(udp_poller)) => udp_poller,
            _ => &mut self.tcp_poller,
        };

        let (packet_sender, packet_receiver) = mpsc::channel();
        let (used_buffer_sender, used_buffer_receiver) = mpsc::channel();

        for _ in 0..max_concurrent_buffers {
            used_buffer_sender.send(vec![]).ok();
        }

        poller.queues.insert(
            stream_id,
            StreamRecvQueues {
                used_buffer_sender: used_buffer_sender.clone(),
                used_buffer_receiver,
                packet_queue: packet_sender,
            },
        );

        StreamReceiver {
            payload_offset: poller.receive_socket.payload_offset(),
            packet_receiver,
            used_buffer_queue: used_buffer_sender,
            last_packet_index: None,
            _phantom: PhantomData,
        }
    }

    pub fn into_pollers(self) -> (SocketPoller, Option<SocketPoller>) {
        (self.tcp_poller, self.maybe_udp_poller)
    }
}
