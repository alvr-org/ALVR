// Note: for StreamSocket, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate

mod quic;
mod tcp;
mod udp;

use super::*;
use crate::{data::*, *};
use bytes::{
    buf::{BufExt, BufMutExt},
    Bytes, BytesMut,
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use quinn::{Connection, IncomingUniStreams, RecvStream, SendStream};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, marker::PhantomData, net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use tokio_util::{
    codec::{Framed, FramedRead, FramedWrite},
    udp::UdpFramed,
};

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub enum StreamId {
    Video(),
    Audio,
    Input,
    Haptics,
}

#[derive(Serialize, Deserialize)]
struct QuicStreamConfigPacket {
    stream_id: StreamId,
    reliable: bool,
}

#[allow(clippy::type_complexity)]
enum StreamSenderType {
    Udp {
        peer_addr: SocketAddr,
        send_socket: Arc<Mutex<SplitSink<UdpFramed<LDC>, (Bytes, SocketAddr)>>>,
    },
    Tcp(Arc<Mutex<SplitSink<Framed<TcpStream, LDC>, Bytes>>>),
    QuicReliable(FramedWrite<SendStream, LDC>),
    QuicUnreliable(Connection),
}

pub struct StreamSender<T> {
    stream_id_bytes: Vec<u8>,
    buffer: Option<BytesMut>,
    sender_type: StreamSenderType,
    _phantom: PhantomData<T>,
}

impl<T: Serialize> StreamSender<T> {
    // send() uses a buffer where memory added (if needed) to fit the serialized packet, and removed
    // when the packet is sent.
    // todo: check if this really helps reducing allocations.
    // NB: memory transferred to the socket cannot be reused because of its send() signature (it
    // takes ownership of a Bytes object)
    pub async fn send(&mut self, packet: &T) -> StrResult {
        let mut buffer = self.buffer.take().unwrap_or_default();
        buffer.clear();

        buffer.extend_from_slice(&self.stream_id_bytes);
        let mut buffer_writer = buffer.writer();
        trace_err!(bincode::serialize_into(&mut buffer_writer, packet))?;

        let mut buffer = buffer_writer.into_inner();
        let packet_bytes = buffer.split().freeze();
        self.buffer = Some(buffer);

        match &mut self.sender_type {
            StreamSenderType::Udp {
                peer_addr,
                send_socket,
            } => trace_err!(
                send_socket
                    .lock()
                    .await
                    .send((packet_bytes, *peer_addr))
                    .await
            ),

            StreamSenderType::Tcp(send_stream) => {
                trace_err!(send_stream.lock().await.send(packet_bytes).await)
            }
            StreamSenderType::QuicReliable(send_stream) => {
                trace_err!(send_stream.send(packet_bytes).await)
            }
            StreamSenderType::QuicUnreliable(connection) => {
                trace_err!(connection.send_datagram(packet_bytes))
            }
        }
    }
}

enum StreamReceiverType {
    Dequeuer(mpsc::UnboundedReceiver<Bytes>),
    QuicReliable(FramedRead<RecvStream, LDC>),
}

pub struct StreamReceiver<T> {
    receiver_type: StreamReceiverType,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> StreamReceiver<T> {
    pub async fn recv(&mut self) -> StrResult<T> {
        let bytes = match &mut self.receiver_type {
            StreamReceiverType::Dequeuer(dequeuer) => trace_none!(dequeuer.next().await)?,
            StreamReceiverType::QuicReliable(receive_stream) => {
                trace_err!(trace_none!(receive_stream.next().await)?)?.freeze()
            }
        };

        trace_err!(bincode::deserialize_from(bytes.reader()))
    }
}

#[allow(clippy::type_complexity)]
enum StreamSocketType {
    Udp {
        peer_addr: SocketAddr,
        send_socket: Arc<Mutex<SplitSink<UdpFramed<LDC>, (Bytes, SocketAddr)>>>,
    },
    Tcp(Arc<Mutex<SplitSink<Framed<TcpStream, LDC>, Bytes>>>),
    Quic {
        connection: Connection,
        reliable_streams_listener: IncomingUniStreams,
        unpaired_stream_receivers: HashMap<StreamId, StreamReceiverType>,
    },
}

pub struct StreamSocket {
    socket_type: StreamSocketType,
    packet_enqueuers: Arc<Mutex<HashMap<StreamId, mpsc::UnboundedSender<Bytes>>>>,
}

impl StreamSocket {
    pub async fn request_stream<T>(
        &self,
        stream_id: StreamId,
        mode: StreamMode,
    ) -> StrResult<StreamSender<T>> {
        let sender_type = match &self.socket_type {
            StreamSocketType::Udp {
                peer_addr,
                send_socket,
            } => StreamSenderType::Udp {
                peer_addr: *peer_addr,
                send_socket: send_socket.clone(),
            },

            StreamSocketType::Tcp(send_stream) => StreamSenderType::Tcp(send_stream.clone()),

            StreamSocketType::Quic { connection, .. } => {
                quic::request_stream(stream_id, mode, connection).await?
            }
        };

        Ok(StreamSender {
            stream_id_bytes: trace_err!(bincode::serialize(&stream_id))?,
            buffer: None,
            sender_type,
            _phantom: PhantomData,
        })
    }

    pub async fn subscribe_to_stream<T>(
        &mut self,
        stream_id: StreamId,
    ) -> StrResult<StreamReceiver<T>> {
        let receiver_type = match &mut self.socket_type {
            StreamSocketType::Udp { .. } | StreamSocketType::Tcp(_) => {
                let (enqueuer, dequeuer) = mpsc::unbounded_channel();
                self.packet_enqueuers
                    .lock()
                    .await
                    .insert(stream_id, enqueuer);

                StreamReceiverType::Dequeuer(dequeuer)
            }

            StreamSocketType::Quic {
                reliable_streams_listener,
                unpaired_stream_receivers,
                ..
            } => {
                quic::subscribe_to_stream(
                    stream_id,
                    reliable_streams_listener,
                    unpaired_stream_receivers,
                    self.packet_enqueuers.clone(),
                )
                .await?
            }
        };

        Ok(StreamReceiver {
            receiver_type,
            _phantom: PhantomData,
        })
    }

    pub async fn connect_to_server(
        server_ip: IpAddr,
        port: u16,
        certificate_pem: String,
        key_pem: String,
        stream_socket_config: SocketConfig,
    ) -> StrResult<Self> {
        let server_addr = (server_ip, port).into();
        match stream_socket_config {
            SocketConfig::Udp => udp::create_socket(server_addr).await,
            SocketConfig::Tcp => tcp::connect_to_server(server_addr).await,
            SocketConfig::Quic(quic_config) => {
                quic::connect_to_server(server_addr, certificate_pem, key_pem, quic_config).await
            }
        }
    }

    pub async fn connect_to_client(
        client_ip: IpAddr,
        port: u16,
        client_identity: PublicIdentity,
        stream_socket_config: SocketConfig,
    ) -> StrResult<Self> {
        let client_addr = (client_ip, port).into();
        match stream_socket_config {
            SocketConfig::Udp => udp::create_socket(client_addr).await,
            SocketConfig::Tcp => tcp::connect_to_client(client_addr).await,
            SocketConfig::Quic(quic_config) => {
                quic::connect_to_client(client_addr, client_identity, quic_config).await
            }
        }
    }
}
