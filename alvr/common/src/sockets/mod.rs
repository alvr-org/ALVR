mod control_socket;

pub use control_socket::*;

use crate::{data::*, logging::*, *};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr},
};
use tokio::net::*;

type LDC = tokio_util::codec::LengthDelimitedCodec;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;

// client_found_cb: returns true if client is trusted, false otherwise
pub async fn search_client_loop<F: Future<Output = bool>>(
    client_found_cb: impl Fn(IpAddr, ClientHandshakePacket) -> F,
) -> StrResult {
    // use naked UdpSocket + [u8] packet buffer to have more control over datagram data
    let handshake_socket = trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        let (handshake_packet_size, address) =
            match handshake_socket.recv_from(&mut packet_buffer).await {
                Ok(pair) => pair,
                Err(e) => {
                    break fmt_e!("Error receiving handshake packet: {}", e);
                }
            };

        let handshake_packet = if let Ok(HandshakePacket::Client(packet)) =
            bincode::deserialize(&packet_buffer[..handshake_packet_size])
        {
            packet
        } else if &packet_buffer[..5] == b"\x01ALVR" {
            log_id(LogId::ClientFoundWrongVersion("v11 or previous".into()));
            return fmt_e!("ALVR client version is too old!");
        } else if &packet_buffer[..4] == b"ALVR" {
            log_id(LogId::ClientFoundWrongVersion("v12.x.x - v13.x.x".into()));
            return fmt_e!("ALVR client version is too old!");
        } else {
            debug!("Found unrelated packet during client discovery");
            continue;
        };

        if handshake_packet.alvr_name != ALVR_NAME {
            log_id(LogId::ClientFoundInvalid);
            return fmt_e!("Error while identifying client");
        }

        if !is_version_compatible(&handshake_packet.version) {
            let response_bytes = trace_err!(bincode::serialize(&HandshakePacket::Server(
                ServerHandshakePacket::IncompatibleVersions
            )))?;
            handshake_socket
                .send_to(&response_bytes, address)
                .await
                .ok();

            log_id(LogId::ClientFoundWrongVersion(
                handshake_packet.version.to_string(),
            ));
            return fmt_e!("Found ALVR client with incompatible version");
        }

        if !client_found_cb(address.ip(), handshake_packet).await {
            let response_bytes = trace_err!(bincode::serialize(&HandshakePacket::Server(
                ServerHandshakePacket::ClientUntrusted
            )))?;

            handshake_socket
                .send_to(&response_bytes, address)
                .await
                .ok();
        }
    }
}
