// Note: for the StreamManager, the client uses a server socket, the server uses a client socket.
// This is because of certificate management. The server needs to trust a client and its certificate

mod control_socket;
mod quic_stream_socket;

pub use control_socket::*;
pub use quic_stream_socket::*;

use crate::{data::*, logging::*, *};
use async_trait::async_trait;
use erased_serde as erased;
use futures::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::*;

type BoxPacket = Box<dyn erased::Serialize + Send>;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 256;

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

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum StreamId {
    Video,
    Audio,
    Input,
    Haptics,
}

pub enum StreamMode {
    PreferReliable,
    PreferUnreliable,
}

#[derive(Serialize, Deserialize)]
struct IdPacket<T> {
    id: StreamId,
    packet: T,
}

// Traits with generics cannot be made into objects. Erased traits must be used.
// todo: find a way to propagate packet type information.

#[async_trait]
pub trait StreamSender {
    async fn send(&mut self, packet: BoxPacket) -> StrResult;
}

pub struct ReceivedPacket {}

impl ReceivedPacket {
    pub fn get<'a, T: Deserialize<'a>>(&'a self) -> StrResult<T> {
        todo!()
    }
}

#[async_trait]
pub trait StreamReceiver {
    async fn recv(&self) -> ReceivedPacket;
}

#[async_trait]
pub trait StreamSocket {
    type Sender: StreamSender;
    type Receiver: StreamReceiver;

    async fn request_stream(
        &self,
        stream_type: StreamId,
        mode: StreamMode,
    ) -> StrResult<Self::Sender>;

    async fn subscribe_to_stream(&self, stream_type: StreamId) -> StrResult<Self::Receiver>;
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

fn handle_old_client(packet_bytes: &[u8]) {
    if packet_bytes.len() > 5 {
        if packet_bytes[..5] == *b"\x01ALVR" {
            warn!(id: LogId::ClientFoundWrongVersion("11 or previous".into()));
        } else if packet_bytes[..4] == *b"ALVR" {
            warn!(id: LogId::ClientFoundWrongVersion("12.x.x".into()));
        } else {
            debug!("Found unrelated packet during client discovery");
        }
    } else {
        debug!("Found unrelated packet during client discovery");
    }
}

// todo: use CBOR with SymmetricallyFramed
pub async fn search_client_loop<F: Future>(
    client_found_cb: impl Fn(IpAddr, Identity) -> F,
) -> StrResult {
    let mut handshake_socket =
        trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        let (handshake_packet_size, address) =
            match handshake_socket.recv_from(&mut packet_buffer).await {
                Ok(pair) => pair,
                Err(e) => {
                    debug!("Error receiving handshake packet: {}", e);
                    continue;
                }
            };

        if address.ip() != MULTICAST_ADDR {
            handle_old_client(&packet_buffer[..handshake_packet_size]);
            continue;
        }

        let handshake_packet: HandshakePacket =
            match serde_cbor::from_slice(&packet_buffer[..handshake_packet_size]) {
                Ok(client_handshake_packet) => client_handshake_packet,
                Err(e) => {
                    warn!(
                        id: LogId::ClientFoundInvalid,
                        "Received handshake packet: {}", e
                    );
                    continue;
                }
            };

        if handshake_packet.alvr_name != ALVR_NAME {
            warn!(
                id: LogId::ClientFoundInvalid,
                "Received handshake packet: wrong name"
            );
            continue;
        }

        match is_version_compatible(&handshake_packet.version, ALVR_CLIENT_VERSION_REQ) {
            Ok(compatible) => {
                if !compatible {
                    warn!(id: LogId::ClientFoundWrongVersion(handshake_packet.version));
                    continue;
                }
            }
            Err(e) => {
                warn!(
                    id: LogId::ClientFoundInvalid,
                    "Received handshake packet: {}", e
                );
                continue;
            }
        }

        let identity = match handshake_packet.identity {
            Some(id) => id,
            None => {
                warn!(
                    id: LogId::ClientFoundInvalid,
                    "Received handshake packet: no identity",
                );
                continue;
            }
        };

        client_found_cb(address.ip(), identity).await;
    }
}
