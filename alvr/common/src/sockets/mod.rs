// Note: for SteamSockets, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate

mod control_socket;
mod quic_stream_socket;

pub use control_socket::*;
pub use quic_stream_socket::*;

use crate::{data::*, logging::*, *};
use async_trait::async_trait;
use bytes::{
    buf::{BufExt, BufMutExt},
    Bytes, BytesMut,
};
use futures::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_cbor as cbor;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::*;

type LDC = tokio_util::codec::LengthDelimitedCodec;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;

pub struct Certificate {
    hostname: String,
    certificate_pem: String,
    key_pem: String,
}

pub fn create_certificate(hostname: Option<String>) -> StrResult<Certificate> {
    let hostname = hostname.unwrap_or(format!("{}.client.alvr", rand::random::<u16>()));

    let certificate = trace_err!(rcgen::generate_simple_self_signed(vec![hostname.clone()]))?;

    Ok(Certificate {
        hostname,
        certificate_pem: trace_err!(certificate.serialize_pem())?,
        key_pem: certificate.serialize_private_key_pem(),
    })
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub enum StreamId {
    Video(),
    Audio,
    Input,
    Haptics,
}

pub struct SendStorage {
    prefix: Vec<u8>,
    buffer: BytesMut,
}

impl SendStorage {
    pub fn encode<T: Serialize>(&mut self, packet: &T) -> StrResult {
        self.buffer.clear();
        self.buffer.extend_from_slice(&self.prefix);
        trace_err!(cbor::to_writer(self.buffer.as_mut().writer(), packet))
    }
}

pub struct ReceivedPacket(Bytes);

impl ReceivedPacket {
    pub fn decode<T: DeserializeOwned>(&self) -> StrResult<T> {
        trace_err!(cbor::from_reader(self.0.as_ref().reader()))
    }
}

// Traits with generics cannot be made into objects. Erased traits must be used.
// todo: find a way to propagate packet type information.

#[async_trait]
pub trait StreamSender {
    async fn get_storage(&self) -> SendStorage;
    async fn send(&mut self, packet: &mut SendStorage) -> StrResult;
}

#[async_trait]
pub trait StreamReceiver {
    async fn recv(&mut self) -> StrResult<ReceivedPacket>;
}

#[async_trait]
pub trait StreamSocket {
    type Sender: StreamSender;
    type Receiver: StreamReceiver;

    async fn request_stream(
        &self,
        stream_id: StreamId,
        mode: StreamMode,
    ) -> StrResult<Self::Sender>;

    async fn subscribe_to_stream(&mut self, stream_id: StreamId) -> StrResult<Self::Receiver>;
}

pub async fn connect_to_server(
    server_ip: IpAddr,
    port: u16,
    certificate_pem: String,
    key_pem: String,
    stream_socket_config: SocketConfig,
) -> StrResult<impl StreamSocket> {
    let server_addr = SocketAddr::new(server_ip, port);
    match stream_socket_config {
        SocketConfig::Quic(quic_config) => {
            QuicStreamSocket::connect_to_server(server_addr, certificate_pem, key_pem, quic_config)
                .await
        }
        _ => todo!(),
    }
}

pub async fn connect_to_client(
    client_ip: IpAddr,
    port: u16,
    client_identity: Identity,
    stream_socket_config: SocketConfig,
) -> StrResult<impl StreamSocket> {
    let client_addr = SocketAddr::new(client_ip, port);
    match stream_socket_config {
        SocketConfig::Quic(quic_config) => {
            QuicStreamSocket::connect_to_client(client_addr, client_identity, quic_config).await
        }
        _ => todo!(),
    }
}

async fn try_connect_to_client(
    handshake_socket: &mut UdpSocket,
    packet_buffer: &mut [u8],
) -> StrResult<Option<(IpAddr, Identity)>> {
    let (handshake_packet_size, address) = match handshake_socket.recv_from(packet_buffer).await {
        Ok(pair) => pair,
        Err(e) => {
            debug!("Error receiving handshake packet: {}", e);
            return Ok(None);
        }
    };

    if address.ip() != MULTICAST_ADDR {
        // Handle wrong client
        if &packet_buffer[..5] == b"\x01ALVR" {
            return trace_str!(id: LogId::ClientFoundWrongVersion("11 or previous".into()));
        } else if &packet_buffer[..4] == b"ALVR" {
            return trace_str!(id: LogId::ClientFoundWrongVersion("12.x.x".into()));
        } else {
            debug!("Found unrelated packet during client discovery");
        }
        return Ok(None);
    }

    let handshake_packet: HandshakePacket = trace_err!(
        serde_cbor::from_slice(&packet_buffer[..handshake_packet_size]),
        id: LogId::ClientFoundInvalid
    )?;

    if handshake_packet.alvr_name != ALVR_NAME {
        return trace_str!(id: LogId::ClientFoundInvalid);
    }

    let compatible = trace_err!(is_version_compatible(
        &handshake_packet.version,
        ALVR_CLIENT_VERSION_REQ
    ))?;
    if !compatible {
        return trace_str!(id: LogId::ClientFoundWrongVersion(handshake_packet.version));
    }

    let identity = trace_none!(handshake_packet.identity, id: LogId::ClientFoundInvalid)?;

    Ok(Some((address.ip(), identity)))
}

// todo: use CBOR with SymmetricallyFramed
pub async fn search_client_loop<F: Future>(
    client_found_cb: impl Fn(IpAddr, Identity) -> F,
) -> StrResult {
    // use naked UdpSocket + [u8] packet buffer to have more control over datagram data
    let mut handshake_socket =
        trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        match try_connect_to_client(&mut handshake_socket, &mut packet_buffer).await {
            Ok(Some((client_ip, identity))) => {
                client_found_cb(client_ip, identity).await;
            }
            Err(e) => warn!("Error while connecting to client: {}", e),
            Ok(None) => (),
        }
    }
}
