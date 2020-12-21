mod control_socket;

pub use control_socket::*;

use crate::{data::*, logging::LogId, *};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr},
};
use tokio::net::*;

type LDC = tokio_util::codec::LengthDelimitedCodec;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;

// pub enum SearchResult {
//     ClientReady(ServerHandshakePacket),
//     Wait,
//     Exit,
// }

// pub async fn search_client<F: Future<Output = SearchResult>>(
//     client_ip: Option<String>,
//     mut client_found_cb: impl FnMut(IpAddr, ClientHandshakePacket) -> F,
// ) -> StrResult {
//     let mut handshake_socket =
//         trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;

//     let maybe_target_client_ip = match client_ip {
//         Some(ip_str) => Some(trace_err!(ip_str.parse::<IpAddr>(), "Client IP")?),
//         None => None,
//     };

//     let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

//     loop {
//         let (hanshake_packet_size, address) =
//             match handshake_socket.recv_from(&mut packet_buffer).await {
//                 Ok(pair) => pair,
//                 Err(e) => {
//                     debug!("Error receiving handshake packet: {}", e);
//                     continue;
//                 }
//             };

//         if let Some(ip) = maybe_target_client_ip {
//             if address.ip() != ip {
//                 info!(id: LogId::ClientFoundWrongIp);
//                 continue;
//             }
//         }

//         let client_handshake_packet: ClientHandshakePacket =
//             match bincode::deserialize(&packet_buffer[..hanshake_packet_size]) {
//                 Ok(client_handshake_packet) => client_handshake_packet,
//                 Err(e) => {
//                     warn!(
//                         id: LogId::ClientFoundInvalid,
//                         "Received handshake packet: {}", e
//                     );
//                     continue;
//                 }
//             };

//         if client_handshake_packet.alvr_name != [b'A', b'L', b'V', b'R'] {
//             warn!(
//                 id: LogId::ClientFoundInvalid,
//                 "Received handshake packet: wrong name"
//             );
//             continue;
//         }

//         let version = {
//             let nul_range_end = client_handshake_packet
//                 .version
//                 .iter()
//                 .position(|&c| c == b'\0')
//                 .unwrap_or_else(|| client_handshake_packet.version.len());
//             String::from_utf8_lossy(&client_handshake_packet.version[0..nul_range_end])
//         };

//         if !is_version_compatible(
//             &semver::Version::parse(&version).unwrap(),
//             &ALVR_CLIENT_VERSION,
//         ) {
//             warn!(id: LogId::ClientFoundWrongVersion(version.into()));
//             continue;
//         }

//         let result = client_found_cb(address.ip(), client_handshake_packet).await;

//         match result {
//             SearchResult::ClientReady(server_handshake_packet) => {
//                 let packet = trace_err!(bincode::serialize(&server_handshake_packet))?;
//                 handshake_socket
//                     .send_to(&packet, SocketAddr::new(address.ip(), 9944))
//                     .await
//                     .ok();
//             }
//             SearchResult::Wait => (),
//             SearchResult::Exit => break Ok(()),
//         }
//     }
// }

async fn try_connect_to_client(
    handshake_socket: &mut UdpSocket,
    packet_buffer: &mut [u8],
) -> StrResult<Option<(IpAddr, HandshakePacket)>> {
    let (handshake_packet_size, address) = match handshake_socket.recv_from(packet_buffer).await {
        Ok(pair) => pair,
        Err(e) => {
            debug!("Error receiving handshake packet: {}", e);
            return Ok(None);
        }
    };

    let handshake_packet: HandshakePacket = if let Ok(handshake_packet) =
        bincode::deserialize(&packet_buffer[..handshake_packet_size])
    {
        handshake_packet
    } else if &packet_buffer[..5] == b"\x01ALVR" {
        return trace_str!(id: LogId::ClientFoundWrongVersion("v11 or previous".into()));
    } else if &packet_buffer[..4] == b"ALVR" {
        return trace_str!(id: LogId::ClientFoundWrongVersion("v12.x.x - v13.x.x".into()));
    } else {
        debug!("Found unrelated packet during client discovery");
        return Ok(None);
    };

    if handshake_packet.alvr_name != ALVR_NAME {
        return trace_str!(id: LogId::ClientFoundInvalid);
    }

    if !is_version_compatible(&handshake_packet.version, &ALVR_CLIENT_VERSION) {
        return trace_str!(id: LogId::ClientFoundWrongVersion(handshake_packet.version.to_string()));
    }

    Ok(Some((address.ip(), handshake_packet)))
}

pub async fn search_client_loop<F: Future>(
    client_found_cb: impl Fn(IpAddr, HandshakePacket) -> F,
) -> StrResult {
    // use naked UdpSocket + [u8] packet buffer to have more control over datagram data
    let mut handshake_socket = trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        match try_connect_to_client(&mut handshake_socket, &mut packet_buffer).await {
            Ok(Some((client_ip, handshake_packet))) => {
                client_found_cb(client_ip, handshake_packet).await;
            }
            Err(e) => warn!("Error while connecting to client: {}", e),
            Ok(None) => (),
        }
    }
}
