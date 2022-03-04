use alvr_common::prelude::*;
use alvr_sockets::{
    ClientHandshakePacket, HandshakePacket, ServerHandshakePacket, CONTROL_PORT, LOCAL_IP,
    MAX_HANDSHAKE_PACKET_SIZE_BYTES,
};
use std::{net::Ipv4Addr, time::Duration};
use tokio::{net::UdpSocket, time};

const CLIENT_HANDSHAKE_RESEND_INTERVAL: Duration = Duration::from_secs(1);

pub enum ConnectionError {
    ServerMessage(ServerHandshakePacket),
    NetworkUnreachable,
}

pub async fn announce_client_loop(
    handshake_packet: ClientHandshakePacket,
) -> StrResult<ConnectionError> {
    let mut handshake_socket = UdpSocket::bind((LOCAL_IP, CONTROL_PORT))
        .await
        .map_err(err!())?;
    handshake_socket.set_broadcast(true).map_err(err!())?;

    let client_handshake_packet =
        bincode::serialize(&HandshakePacket::Client(handshake_packet)).map_err(err!())?;

    loop {
        let broadcast_result = handshake_socket
            .send_to(
                &client_handshake_packet,
                (Ipv4Addr::BROADCAST, CONTROL_PORT),
            )
            .await;
        if broadcast_result.is_err() {
            break Ok(ConnectionError::NetworkUnreachable);
        }

        let receive_response_loop = {
            let handshake_socket = &mut handshake_socket;
            async move {
                let mut server_response_buffer = [0; MAX_HANDSHAKE_PACKET_SIZE_BYTES];
                loop {
                    // this call will receive also the broadcasted client packet that must be ignored
                    let (packet_size, _) = handshake_socket
                        .recv_from(&mut server_response_buffer)
                        .await
                        .map_err(err!())?;

                    if let Ok(HandshakePacket::Server(handshake_packet)) =
                        bincode::deserialize(&server_response_buffer[..packet_size])
                    {
                        warn!("received packet {handshake_packet:?}");
                        break Ok(ConnectionError::ServerMessage(handshake_packet));
                    }
                }
            }
        };

        tokio::select! {
            res = receive_response_loop => break res,
            _ = time::sleep(CLIENT_HANDSHAKE_RESEND_INTERVAL) => {
                warn!("Server not found, resending handhake packet");
            }
        }
    }
}
