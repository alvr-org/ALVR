mod control_socket;
mod stream_socket;

pub use control_socket::*;
pub use stream_socket::*;

use crate::{data::*, logging::*, *};
use futures::Future;
use std::net::{IpAddr, Ipv4Addr};
use tokio::net::*;

type LDC = tokio_util::codec::LengthDelimitedCodec;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;

async fn try_connect_to_client(
    handshake_socket: &mut UdpSocket,
    packet_buffer: &mut [u8],
) -> StrResult<Option<(IpAddr, PublicIdentity)>> {
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
        bincode::deserialize(&packet_buffer[..handshake_packet_size]),
        id: LogId::ClientFoundInvalid
    )?;

    if handshake_packet.alvr_name != ALVR_NAME {
        return trace_str!(id: LogId::ClientFoundInvalid);
    }

    if !is_version_compatible(&handshake_packet.version, &ALVR_CLIENT_VERSION) {
        return trace_str!(id: LogId::ClientFoundWrongVersion(handshake_packet.version.to_string()));
    }

    let identity = trace_none!(handshake_packet.identity, id: LogId::ClientFoundInvalid)?;

    Ok(Some((address.ip(), identity)))
}

pub async fn search_client_loop<F: Future>(
    client_found_cb: impl Fn(IpAddr, PublicIdentity) -> F,
) -> StrResult {
    // use naked UdpSocket + [u8] packet buffer to have more control over datagram data
    let mut handshake_socket = trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;

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
