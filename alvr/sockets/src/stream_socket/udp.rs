use super::{
    MultiplexedSocketReader, MultiplexedSocketWriter, ReconstructedPacket, StreamRecvQueues,
};
use crate::LOCAL_IP;
use alvr_common::{ConResult, HandleTryAgain, ToCon, anyhow::Result};
use alvr_session::{DscpTos, SocketBufferSize};
use socket2::{MaybeUninitSlice, Socket};
use std::ffi::c_int;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    mem::{self, MaybeUninit},
    net::{IpAddr, UdpSocket},
    ptr,
    time::Duration,
};

pub const SHARD_PREFIX_SIZE: usize = mem::size_of::<u16>() // stream ID
    + mem::size_of::<u32>() // packet index
    + mem::size_of::<u32>() // shards count
    + mem::size_of::<u32>(); // shards index

fn socket_peek(socket: &mut Socket, buffer: &mut [u8]) -> ConResult<usize> {
    #[cfg(windows)]
    const FLAGS: c_int = 0x02 | 0x8000; // MSG_PEEK | MSG_PARTIAL
    #[cfg(not(windows))]
    const FLAGS: c_int = 0x02 | 0x20; // MSG_PEEK | MSG_TRUNC

    let buffer =
        MaybeUninitSlice::new(unsafe { &mut *(ptr::from_mut(buffer) as *mut [MaybeUninit<u8>]) });
    // NB: Using the non vectored call doesn't seem to work
    Ok(socket
        .recv_vectored_with_flags(&mut [buffer], FLAGS)
        .handle_try_again()?
        .0)
}

// Create tokio socket, convert to socket2, apply settings, convert back to tokio. This is done to
// let tokio set all the internal parameters it needs from the start.
pub fn bind(
    port: u16,
    dscp: Option<DscpTos>,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<UdpSocket> {
    let socket = UdpSocket::bind((LOCAL_IP, port))?.into();

    crate::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();

    crate::set_dscp(&socket, dscp);

    Ok(socket.into())
}

pub fn connect(socket: &UdpSocket, peer_ip: IpAddr, port: u16, timeout: Duration) -> Result<()> {
    socket.connect((peer_ip, port))?;
    socket.set_read_timeout(Some(timeout))?;

    Ok(())
}

pub struct MultiplexedUdpWriter {
    inner: UdpSocket,
    max_packet_size: usize,
}

impl MultiplexedSocketWriter for MultiplexedUdpWriter {
    fn payload_offset(&self) -> usize {
        SHARD_PREFIX_SIZE
    }

    fn send(&mut self, stream_id: u16, packet_index: u32, buffer: &mut Vec<u8>) -> Result<()> {
        let max_shard_size = self.max_packet_size - SHARD_PREFIX_SIZE;
        let payload_size = buffer.len() - SHARD_PREFIX_SIZE;
        // rounding up:
        let shards_count = payload_size.div_ceil(max_shard_size);

        for shard_idx in 0..shards_count {
            // this overlaps with the previous shard, this is intended behavior and allows to
            // reduce allocations
            let shard_start_position = shard_idx * max_shard_size;
            let shard_size = usize::min(max_shard_size, payload_size - shard_start_position);

            let shard_view = &mut buffer[shard_start_position..][..SHARD_PREFIX_SIZE + shard_size];

            shard_view[0..2].copy_from_slice(&stream_id.to_le_bytes());
            shard_view[2..6].copy_from_slice(&packet_index.to_le_bytes());
            shard_view[6..10].copy_from_slice(&(shards_count as u32).to_le_bytes());
            shard_view[10..14].copy_from_slice(&(shard_idx as u32).to_le_bytes());

            self.inner.send(shard_view)?;
        }

        Ok(())
    }
}

// We need to store the size seaparately because we use use the buffer as unallocated memory and
// the capacity cannot be set precisely.
// Note: Why do we need to keep space for the prefix in the final buffer?
// UDP works with discrete packets, reading must be done in a single call. There is no way to
// preemptively discard the bytes of the prefix.
struct InProgressPacket {
    buffer: Vec<u8>,    // contains the prefix
    buffer_size: usize, // size of the packet counting prefix
    shards_count: usize,
    received_shard_indices: HashSet<usize>,
}

pub struct MultiplexedUdpReader {
    inner: Socket,
    max_packet_size: usize,
    in_progress_packets: HashMap<u16, HashMap<u32, InProgressPacket>>,
}

impl MultiplexedSocketReader for MultiplexedUdpReader {
    fn payload_offset(&self) -> usize {
        SHARD_PREFIX_SIZE
    }

    fn recv(&mut self, stream_queues: &HashMap<u16, StreamRecvQueues>) -> ConResult {
        let max_shard_data_size = self.max_packet_size - SHARD_PREFIX_SIZE;

        let discard_and_try_again = move |socket: &Socket| {
            // Reading with any sized buffer (even 0) will consume the whole datagram
            socket.recv(&mut []).ok();
            alvr_common::try_again()
        };

        let mut prefix_bytes = [0; SHARD_PREFIX_SIZE];
        let peek_size = socket_peek(&mut self.inner, &mut prefix_bytes)?;
        if peek_size < SHARD_PREFIX_SIZE {
            return discard_and_try_again(&self.inner);
        }

        // The values obtained from the prefix (stream ID, packet index, shards count, shard index)
        // could be corrupted somehow. This method has safety checks against corrupted values and
        // the relative packet would be discarded.
        let stream_id = u16::from_le_bytes(prefix_bytes[0..2].try_into().unwrap());
        let packet_index = u32::from_le_bytes(prefix_bytes[2..6].try_into().unwrap());
        let maybe_shards_count =
            u32::from_le_bytes(prefix_bytes[6..10].try_into().unwrap()) as usize;
        let shard_index = u32::from_le_bytes(prefix_bytes[10..14].try_into().unwrap()) as usize;

        if maybe_shards_count == 0 {
            return discard_and_try_again(&self.inner);
        }

        let Some(queues) = stream_queues.get(&stream_id) else {
            return discard_and_try_again(&self.inner);
        };
        let in_progress_packets = self.in_progress_packets.entry(stream_id).or_default();

        let in_progress_packet = if let Some(packet) = in_progress_packets.get_mut(&packet_index) {
            packet
        } else if let Some(mut buffer) = queues.used_buffer_receiver.try_recv().ok().or_else(|| {
            // By default, try to dequeue a used buffer. In case none were found, recycle one of the
            // in progress packets, chances are these buffers are "dead" because one of their shards
            // has been dropped by the network.
            let idx = *in_progress_packets.iter().next()?.0;
            Some(in_progress_packets.remove(&idx).unwrap().buffer)
        }) {
            // The first shard prefix will dictate the actual number of shards of the packet. The
            // reserved capacity is an upper bound: we don't know yet the exact size, the last
            // shard could be smaller than max_shard_data_size
            buffer.clear();
            buffer.reserve(SHARD_PREFIX_SIZE + max_shard_data_size * maybe_shards_count);

            in_progress_packets
                .entry(packet_index)
                .or_insert(InProgressPacket {
                    buffer,
                    buffer_size: 0,
                    shards_count: maybe_shards_count,
                    // todo: find a way to skipping this allocation
                    received_shard_indices: HashSet::with_capacity(maybe_shards_count),
                })
        } else {
            // This branch may be hit in case the thread related to the stream hangs for some reason
            return discard_and_try_again(&self.inner);
        };

        if shard_index >= in_progress_packet.shards_count
            || in_progress_packet
                .received_shard_indices
                .contains(&shard_index)
        {
            return discard_and_try_again(&self.inner);
        }

        // Note: there is no prefix offset, since we want to write the prefix too.
        let packet_start_index = shard_index * max_shard_data_size;

        // Note: this is a MaybeUninit slice
        let sub_buffer = &mut in_progress_packet.buffer.spare_capacity_mut()[packet_start_index..];

        // Safety: bound checks lead from the previous code
        let overwritten_data_backup: [_; SHARD_PREFIX_SIZE] =
            sub_buffer[..SHARD_PREFIX_SIZE].try_into().unwrap();

        // This call should never fail because the peek call succeded before.
        // Note: in unexpected circumstances, here .to_con() is used not to emit TryAgain, which
        // would mess with the state of the code. The connection would need to be closed instead.
        // NB: the received_size contains the prefix
        let received_size = self.inner.recv(sub_buffer).to_con()?;

        // Restore backed up bytes
        sub_buffer[..SHARD_PREFIX_SIZE].copy_from_slice(&overwritten_data_backup);

        in_progress_packet.buffer_size = usize::max(
            in_progress_packet.buffer_size,
            packet_start_index + received_size,
        );

        in_progress_packet
            .received_shard_indices
            .insert(shard_index);

        // Check if packet is complete (and not dummy) and send
        if in_progress_packet.received_shard_indices.len() == in_progress_packet.shards_count {
            if let Some(mut packet) = in_progress_packets.remove(&packet_index) {
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

            // Keep only shards with later packet index (using wrapping logic)
            while let Some((idx, _)) = in_progress_packets
                .iter()
                .find(|(idx, _)| super::wrapping_cmp(**idx, packet_index) == Ordering::Less)
            {
                let idx = *idx; // fix borrow rule
                let packet = in_progress_packets.remove(&idx).unwrap();

                // Recycle buffer
                queues.used_buffer_sender.send(packet.buffer).ok();
            }
        }

        Ok(())
    }
}

pub fn split_multiplexed(
    socket: UdpSocket,
    max_packet_size: usize,
) -> Result<(
    Box<dyn MultiplexedSocketWriter + Send>,
    Box<dyn MultiplexedSocketReader + Send>,
)> {
    let writer = MultiplexedUdpWriter {
        inner: socket.try_clone()?,
        max_packet_size,
    };

    let reader = MultiplexedUdpReader {
        inner: socket.into(),
        max_packet_size,
        in_progress_packets: HashMap::new(),
    };

    Ok((Box::new(writer), Box::new(reader)))
}
