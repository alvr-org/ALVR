use alvr_common::{prelude::*, ALVR_NAME};
use alvr_sockets::{
    ClientHandshakePacket, HandshakePacket, ServerHandshakePacket, CONTROL_PORT, LOCAL_IP,
    MAX_HANDSHAKE_PACKET_SIZE_BYTES,
};
use std::{future::Future, net::IpAddr};
use tokio::net::UdpSocket;

// client_found_cb: returns true if client is trusted, false otherwise
pub async fn search_client_loop<F: Future<Output = bool>>(
    client_found_cb: impl Fn(ClientHandshakePacket) -> F,
) -> StrResult<(IpAddr, ClientHandshakePacket)> {
    // use naked UdpSocket + [u8] packet buffer to have more control over datagram data
    let handshake_socket = trace_err!(UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).await)?;

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        let (handshake_packet_size, client_address) =
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
            log_event(Event::ClientFoundWrongVersion("v11 or previous".into()));
            return fmt_e!("ALVR client version is too old!");
        } else if &packet_buffer[..4] == b"ALVR" {
            log_event(Event::ClientFoundWrongVersion("v12.x.x - v13.x.x".into()));
            return fmt_e!("ALVR client version is too old!");
        } else {
            debug!("Found unrelated packet during client discovery");
            continue;
        };

        if handshake_packet.alvr_name != ALVR_NAME {
            log_event(Event::ClientFoundInvalid);
            return fmt_e!("Error while identifying client");
        }

        if !alvr_common::is_version_compatible(&handshake_packet.version) {
            let response_bytes = trace_err!(bincode::serialize(&HandshakePacket::Server(
                ServerHandshakePacket::IncompatibleVersions
            )))?;
            handshake_socket
                .send_to(&response_bytes, client_address)
                .await
                .ok();

            log_event(Event::ClientFoundWrongVersion(
                handshake_packet.version.to_string(),
            ));
            return fmt_e!("Found ALVR client with incompatible version");
        }

        if !client_found_cb(handshake_packet.clone()).await {
            let response_bytes = trace_err!(bincode::serialize(&HandshakePacket::Server(
                ServerHandshakePacket::ClientUntrusted
            )))?;

            handshake_socket
                .send_to(&response_bytes, client_address)
                .await
                .ok();
        } else {
            break Ok((client_address.ip(), handshake_packet));
        }
    }
}
