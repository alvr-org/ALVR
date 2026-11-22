use super::{
    MultiplexedSocketReader, MultiplexedSocketWriter, ReconstructedPacket,
    SHARD_COMMON_PREFIX_SIZE, ShardPrefix, SocketConnectionConfig, StreamRecvQueues,
};
use crate::LOCAL_IP;
use alvr_common::{ConResult, HandleTryAgain, ToCon, anyhow::Result, con_bail, parking_lot::Mutex};
use socket2::Socket;
use std::{
    collections::HashMap,
    io::Write,
    mem::{self, MaybeUninit},
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    time::Duration,
};

pub const SHARD_PREFIX_SIZE: usize = SHARD_COMMON_PREFIX_SIZE + mem::size_of::<u32>(); // shard size

pub fn bind(timeout: Duration, port: u16) -> Result<TcpListener> {
    let socket: Socket = TcpListener::bind((LOCAL_IP, port))?.into();

    socket.set_read_timeout(Some(timeout))?;

    Ok(socket.into())
}

pub fn accept_from_server(
    listener: &TcpListener,
    server_ip: Option<IpAddr>,
    timeout: Duration,
) -> ConResult<TcpStream> {
    // Uses timeout set during bind()
    let (socket, server_address) = listener.accept().handle_try_again()?;

    if let Some(ip) = server_ip
        && server_address.ip() != ip
    {
        con_bail!(
            "Connected to wrong client: Expected: {ip}, Found {}",
            server_address.ip()
        );
    }

    socket.set_read_timeout(Some(timeout)).to_con()?;
    socket.set_nodelay(true).to_con()?;

    Ok(socket)
}

pub fn connect_to_client(
    timeout: Duration,
    client_ips: &[IpAddr],
    port: u16,
) -> ConResult<TcpStream> {
    let split_timeout = timeout / client_ips.len() as u32;

    let mut res = alvr_common::try_again();
    for ip in client_ips {
        res = TcpStream::connect_timeout(&SocketAddr::new(*ip, port), split_timeout)
            .handle_try_again();

        if res.is_ok() {
            break;
        }
    }
    let socket = res?;

    socket.set_read_timeout(Some(timeout)).to_con()?;
    socket.set_nodelay(true).to_con()?;

    Ok(socket)
}

pub struct MultiplexedTcpWriter {
    inner: Mutex<Arc<Socket>>,
    max_packet_size: usize,
}

impl MultiplexedSocketWriter for MultiplexedTcpWriter {
    fn payload_offset(&self) -> usize {
        SHARD_PREFIX_SIZE
    }

    fn send(&self, stream_id: u16, packet_index: u32, buffer: &mut Vec<u8>) -> Result<()> {
        super::send_sharded(
            buffer,
            SHARD_PREFIX_SIZE,
            self.max_packet_size,
            stream_id,
            packet_index,
            |shard| {
                let shard_size = (shard.len() - SHARD_PREFIX_SIZE) as u32;
                shard[SHARD_COMMON_PREFIX_SIZE..SHARD_PREFIX_SIZE]
                    .copy_from_slice(&shard_size.to_le_bytes());

                self.inner.lock().as_ref().write_all(shard)?;

                Ok(())
            },
        )
    }
}

struct RecvState {
    prefix: ShardPrefix,
    shard_length: usize,  // contains prefix length itself
    packet_cursor: usize, // counts also the prefix bytes
    overwritten_data_backup: Option<[MaybeUninit<u8>; SHARD_PREFIX_SIZE]>,
    should_discard: bool,
}

struct InProgressPacket {
    packet_index: u32,
    buffer: Vec<u8>,    // contains the prefix
    buffer_size: usize, // size of the packet counting prefix
    shards_count: usize,
    received_shards_count: usize,
}

pub struct MultiplexedTcpReader {
    inner: Arc<Socket>,
    max_packet_size: usize,
    shard_recv_state: Option<RecvState>,
    in_progress_packets: HashMap<u16, InProgressPacket>,
    discarded_shards_sink: Vec<u8>,
    used_buffers_poll_timeout: Duration,
}

impl MultiplexedSocketReader for MultiplexedTcpReader {
    fn payload_offset(&self) -> usize {
        SHARD_PREFIX_SIZE
    }

    fn recv(&mut self, stream_queues: &HashMap<u16, StreamRecvQueues>) -> ConResult {
        let max_shard_data_size = self.max_packet_size - SHARD_PREFIX_SIZE;

        let shard_recv_state_mut = if let Some(state) = &mut self.shard_recv_state {
            state
        } else {
            let mut prefix_bytes = [0u8; SHARD_PREFIX_SIZE];

            loop {
                let count = self
                    .inner
                    .peek(unsafe {
                        &mut *(&mut prefix_bytes as *mut [u8] as *mut [MaybeUninit<u8>])
                    })
                    .handle_try_again()?;
                if count == SHARD_PREFIX_SIZE {
                    break;
                } else if count == 0 {
                    con_bail!("Connection closed by peer");
                }
            }

            let prefix = ShardPrefix::read(&prefix_bytes);
            let shard_payload_size = u32::from_le_bytes(
                prefix_bytes[SHARD_COMMON_PREFIX_SIZE..SHARD_PREFIX_SIZE]
                    .try_into()
                    .unwrap(),
            ) as usize;

            if prefix.shard_index >= prefix.shards_count || shard_payload_size > max_shard_data_size
            {
                con_bail!(
                    "Invalid shard {} of {} with size {shard_payload_size}",
                    prefix.shard_index,
                    prefix.shards_count
                );
            }

            self.shard_recv_state.insert(RecvState {
                prefix,
                shard_length: SHARD_PREFIX_SIZE + shard_payload_size,
                packet_cursor: 0,
                overwritten_data_backup: None,
                should_discard: false,
            })
        };

        let stream_id = shard_recv_state_mut.prefix.stream_id;
        let packet_index = shard_recv_state_mut.prefix.packet_index;
        let shard_index = shard_recv_state_mut.prefix.shard_index;

        if !shard_recv_state_mut.should_discard {
            if let Some(queues) = stream_queues.get(&stream_id) {
                let is_new_packet = !self
                    .in_progress_packets
                    .get(&stream_id)
                    .is_some_and(|packet| packet.packet_index == packet_index);

                if is_new_packet {
                    if shard_index != 0 {
                        con_bail!("Shard {shard_index} of unknown packet for stream {stream_id}");
                    }

                    let mut buffer = queues
                        .used_buffer_receiver
                        .recv_timeout(self.used_buffers_poll_timeout)
                        .handle_try_again()?;

                    // The first shard prefix will dictate the actual number of shards of the
                    // packet. The reserved capacity is an upper bound: we don't know yet the exact
                    // size, the last shard could be smaller than max_shard_data_size
                    let shards_count = shard_recv_state_mut.prefix.shards_count;
                    buffer.clear();
                    buffer.reserve(SHARD_PREFIX_SIZE + max_shard_data_size * shards_count);

                    if let Some(stale_packet) = self.in_progress_packets.insert(
                        stream_id,
                        InProgressPacket {
                            packet_index,
                            buffer,
                            buffer_size: 0,
                            shards_count,
                            received_shards_count: 0,
                        },
                    ) {
                        queues.used_buffer_sender.send(stale_packet.buffer).ok();
                    }
                }
            } else {
                shard_recv_state_mut.should_discard = true;
            }
        }

        let (buffer, packet_start_index) = if shard_recv_state_mut.should_discard {
            self.discarded_shards_sink
                .reserve(shard_recv_state_mut.shard_length);

            (&mut self.discarded_shards_sink, 0)
        } else {
            let Some(packet) = self.in_progress_packets.get_mut(&stream_id) else {
                con_bail!("Missing in-progress packet for stream {stream_id}");
            };
            if packet.received_shards_count != shard_index {
                con_bail!(
                    "Out of order shard {shard_index} for stream {stream_id}, expected {}",
                    packet.received_shards_count
                );
            }

            // Note: there is no prefix offset, since we want to write the prefix too.
            let packet_start_index = packet.buffer_size.saturating_sub(SHARD_PREFIX_SIZE);

            (&mut packet.buffer, packet_start_index)
        };

        // Note: this is a MaybeUninit slice
        let sub_buffer = &mut buffer.spare_capacity_mut()[packet_start_index..]
            [..shard_recv_state_mut.shard_length];

        // Backup the small section of bytes that will be overwritten by reading from socket.
        if shard_recv_state_mut.overwritten_data_backup.is_none() {
            shard_recv_state_mut.overwritten_data_backup =
                Some(sub_buffer[..SHARD_PREFIX_SIZE].try_into().unwrap());
        }

        // This loop may bail out at any time if a timeout is reached. This is correctly handled
        // by the previous code.
        while shard_recv_state_mut.packet_cursor < shard_recv_state_mut.shard_length {
            let size = self
                .inner
                .recv(&mut sub_buffer[shard_recv_state_mut.packet_cursor..])
                .handle_try_again()?;
            if size == 0 {
                con_bail!("Connection closed by peer");
            }

            shard_recv_state_mut.packet_cursor += size;
        }

        // Restore backed up bytes
        if let Some(backup) = shard_recv_state_mut.overwritten_data_backup.take() {
            sub_buffer[..SHARD_PREFIX_SIZE].copy_from_slice(&backup);
        }

        let shard_end_index = packet_start_index + shard_recv_state_mut.shard_length;
        let should_discard = shard_recv_state_mut.should_discard;

        // Mark current shard as read and allow for a new shard to be read
        self.shard_recv_state = None;

        if should_discard {
            return Ok(());
        }

        let Some(packet) = self.in_progress_packets.get_mut(&stream_id) else {
            con_bail!("Missing in-progress packet for stream {stream_id}");
        };
        packet.buffer_size = usize::max(packet.buffer_size, shard_end_index);
        packet.received_shards_count += 1;

        // Check if packet is complete and send
        if packet.received_shards_count == packet.shards_count
            && let Some(mut packet) = self.in_progress_packets.remove(&stream_id)
            && let Some(queues) = stream_queues.get(&stream_id)
        {
            // All writing was done to uninit capacity, here we set the final buffer length
            unsafe { packet.buffer.set_len(packet.buffer_size) };

            queues
                .packet_queue
                .send(ReconstructedPacket {
                    index: packet_index,
                    buffer: packet.buffer,
                })
                .ok();
        }

        Ok(())
    }
}

pub fn split_multiplexed(
    socket: TcpStream,
    config: &SocketConnectionConfig,
) -> Result<(MultiplexedTcpWriter, MultiplexedTcpReader)> {
    let socket = Arc::new(Socket::from(socket));
    crate::configure_socket(&socket, config)?;

    let writer = MultiplexedTcpWriter {
        inner: Mutex::new(Arc::clone(&socket)),
        max_packet_size: config.max_packet_size,
    };

    let reader = MultiplexedTcpReader {
        inner: socket,
        max_packet_size: config.max_packet_size,
        shard_recv_state: None,
        in_progress_packets: HashMap::new(),
        discarded_shards_sink: vec![],
        used_buffers_poll_timeout: config.timeout,
    };

    Ok((writer, reader))
}
