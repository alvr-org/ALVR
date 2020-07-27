use crate::{data::*, logging::*, *};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{mpsc::Sender, Arc},
};
use thread_loop::ThreadLoop;
use tokio::net::*;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const CONTROL_PORT: u16 = 9943;
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 256;

pub async fn search_client(
    client_ip: Option<String>,
    client_found_cb: impl Fn(IpAddr),
) -> StrResult {
    let mut handshake_socket =
        trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, CONTROL_PORT)).await)?;
    trace_err!(handshake_socket.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED))?;

    let maybe_target_client_ip = match client_ip {
        Some(ip_str) => Some(trace_err!(ip_str.parse::<IpAddr>(), "Client IP")?),
        None => None,
    };

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        let (hanshake_packet_size, address) =
            match handshake_socket.recv_from(&mut packet_buffer).await {
                Ok(pair) => pair,
                Err(e) => {
                    debug!("Error receiving handshake packet: {}", e);
                    continue;
                }
            };

        if let Some(ip) = maybe_target_client_ip {
            if address.ip() != ip {
                info!(id: LogId::ClientFoundWrongIp);
                continue;
            }
        }

        let handshake_packet: HandshakePacket =
            match bincode::deserialize(&packet_buffer[..hanshake_packet_size]) {
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

        client_found_cb(address.ip());

        // if let Some(server_handshake_packet) = maybe_server_handshake_packet {
        //     let packet = trace_err!(bincode::serialize(&server_handshake_packet))?;
        //     handshake_socket
        //         .send_to(&packet, SocketAddr::new(address.ip(), 9944))
        //         .await
        //         .ok();
        // }
    }
}

struct ControlSocket {
}

impl ControlSocket {
    pub fn connect_to_client(client_ip: IpAddr) {

    }
}

trait StreamSocket {
    fn receive(&self);
    fn send_reliable(&self, stream_id: u8);
    fn send_unreliable(&self, stream_id: u8);
}

struct QuicSocket {}

// impl StreamSocket for QuicSocket {

// }
