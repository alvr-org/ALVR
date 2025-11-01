use super::{
    MultiplexedSocketReader, MultiplexedSocketWriter, ReconstructedPacket, StreamRecvQueues,
};
use crate::LOCAL_IP;
use alvr_common::{ConResult, HandleTryAgain, ToCon, anyhow::Result, con_bail, error};
use alvr_session::{DscpTos, SocketBufferSize};
use socket2::Socket;
use std::{
    collections::HashMap,
    io::Write,
    mem::{self, MaybeUninit},
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

pub const PACKET_PREFIX_SIZE: usize = mem::size_of::<u16>() // stream ID
    + mem::size_of::<u32>() // packet index
    + mem::size_of::<u32>(); // payload size

pub fn bind(
    timeout: Duration,
    port: u16,
    dscp: Option<DscpTos>,
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
) -> Result<TcpListener> {
    let socket = TcpListener::bind((LOCAL_IP, port))?.into();

    crate::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();

    crate::set_dscp(&socket, dscp);

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
    send_buffer_bytes: SocketBufferSize,
    recv_buffer_bytes: SocketBufferSize,
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
    let socket = res?.into();

    crate::set_socket_buffers(&socket, send_buffer_bytes, recv_buffer_bytes).ok();
    socket.set_read_timeout(Some(timeout)).to_con()?;

    let socket = TcpStream::from(socket);

    socket.set_nodelay(true).to_con()?;

    Ok(socket)
}

pub struct MultiplexedTcpWriter {
    inner: TcpStream,
}

impl MultiplexedSocketWriter for MultiplexedTcpWriter {
    fn payload_offset(&self) -> usize {
        PACKET_PREFIX_SIZE
    }

    // `buffer` contains the payload offset by `payload_offset()`
    fn send(&mut self, stream_id: u16, packet_index: u32, buffer: &mut Vec<u8>) -> Result<()> {
        let payload_size = buffer.len() - PACKET_PREFIX_SIZE;

        buffer[0..2].copy_from_slice(&stream_id.to_le_bytes());
        buffer[2..6].copy_from_slice(&packet_index.to_le_bytes());
        buffer[6..10].copy_from_slice(&(payload_size as u32).to_le_bytes());

        self.inner.write_all(buffer)?;

        Ok(())
    }
}

struct InProgressPacket {
    stream_id: u16,
    packet_index: u32,
    buffer: Vec<u8>,
    buffer_size: usize,
    cursor: usize,
}

pub struct MultiplexedTcpReader {
    inner: Socket,
    in_progress_packet: Option<InProgressPacket>,
    used_buffers_poll_timeout: Duration,
}

impl MultiplexedSocketReader for MultiplexedTcpReader {
    fn payload_offset(&self) -> usize {
        PACKET_PREFIX_SIZE
    }

    fn recv(&mut self, stream_queues: &HashMap<u16, StreamRecvQueues>) -> ConResult {
        let in_progress_packet = if let Some(packet) = &mut self.in_progress_packet {
            packet
        } else {
            let mut prefix_bytes = [0u8; PACKET_PREFIX_SIZE];

            loop {
                let count = self
                    .inner
                    .peek(unsafe {
                        &mut *(&mut prefix_bytes as *mut [u8] as *mut [MaybeUninit<u8>])
                    })
                    .handle_try_again()?;
                error!("recv: peek count {}", count);
                if count == PACKET_PREFIX_SIZE {
                    break;
                }
            }

            let stream_id = u16::from_le_bytes(prefix_bytes[0..2].try_into().unwrap());
            let packet_index = u32::from_le_bytes(prefix_bytes[2..6].try_into().unwrap());
            let payload_size = u32::from_le_bytes(prefix_bytes[6..10].try_into().unwrap()) as usize;

            let mut buffer = match stream_queues.get(&stream_id) {
                Some(queue) => queue
                    .used_buffer_receiver
                    .recv_timeout(self.used_buffers_poll_timeout)
                    .handle_try_again()?,
                None => {
                    // This is a packet with an invalid stream id, but we must read it anyway. We
                    // can't obtain a used buffer so we create a new one
                    vec![]
                }
            };

            buffer.clear();
            buffer.reserve(PACKET_PREFIX_SIZE + payload_size);

            self.in_progress_packet.insert(InProgressPacket {
                stream_id,
                packet_index,
                buffer,
                buffer_size: PACKET_PREFIX_SIZE + payload_size,
                cursor: 0,
            })
        };

        while in_progress_packet.cursor < in_progress_packet.buffer_size {
            let sub_buffer = &mut in_progress_packet.buffer.spare_capacity_mut()
                [in_progress_packet.cursor..in_progress_packet.buffer_size];

            in_progress_packet.cursor += self.inner.recv(sub_buffer).handle_try_again()?;
        }

        // All writing was done to uninit capacity, here we set the final buffer length
        unsafe {
            in_progress_packet
                .buffer
                .set_len(in_progress_packet.buffer_size)
        };

        if let Some(queues) = stream_queues.get(&in_progress_packet.stream_id) {
            // Safety: here self.in_progress_packet is always Some
            queues
                .packet_queue
                .send(ReconstructedPacket {
                    index: in_progress_packet.packet_index,
                    buffer: self.in_progress_packet.take().unwrap().buffer,
                })
                .to_con()?;
        } else {
            // The packet had an invalid stream ID. Discard the buffer
            self.in_progress_packet.take();
        }

        Ok(())
    }
}

pub fn split_multiplexed(
    socket: TcpStream,
    used_buffers_poll_timeout: Duration,
) -> Result<(
    Box<dyn MultiplexedSocketWriter + Send>,
    Box<dyn MultiplexedSocketReader + Send>,
)> {
    let writer = MultiplexedTcpWriter {
        inner: socket.try_clone()?,
    };

    let reader = MultiplexedTcpReader {
        inner: socket.into(),
        in_progress_packet: None,
        used_buffers_poll_timeout,
    };

    Ok((Box::new(writer), Box::new(reader)))
}
