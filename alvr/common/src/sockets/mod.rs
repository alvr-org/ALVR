mod control_socket;

pub use control_socket::*;

use crate::{data::*, logging::*, *};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::{net::*, time};

type LDC = tokio_util::codec::LengthDelimitedCodec;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;
const DISCOVERY_PAUSE_INTERVAL: Duration = Duration::from_millis(500);

async fn try_connect_to_client(
    handshake_socket: &mut UdpSocket,
    packet_buffer: &mut [u8],
) -> StrResult<Option<(SocketAddr, ClientHandshakePacket)>> {
    let (handshake_packet_size, address) = match handshake_socket.recv_from(packet_buffer).await {
        Ok(pair) => pair,
        Err(e) => {
            debug!("Error receiving handshake packet: {}", e);
            return Ok(None);
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
        return Ok(None);
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

    Ok(Some((address, handshake_packet)))
}

// client_found_cb: returns true if client is trusted, false otherwise
pub async fn search_client_loop<F: Future<Output = bool>>(
    client_found_cb: impl Fn(IpAddr, ClientHandshakePacket) -> F,
) -> StrResult {
    // use naked UdpSocket + [u8] packet buffer to have more control over datagram data
    let mut handshake_socket = trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        match try_connect_to_client(&mut handshake_socket, &mut packet_buffer).await {
            Ok(Some((client_address, handshake_packet))) => {
                if !client_found_cb(client_address.ip(), handshake_packet).await {
                    let response_bytes = trace_err!(bincode::serialize(&HandshakePacket::Server(
                        ServerHandshakePacket::ClientUntrusted
                    )))?;

                    handshake_socket
                        .send_to(&response_bytes, client_address)
                        .await
                        .ok();
                }
            }
            Err(e) => warn!("Error while connecting to client: {}", e),
            Ok(None) => (),
        }

        // small pause to avoid fast looping
        time::sleep(DISCOVERY_PAUSE_INTERVAL).await;
    }
}
