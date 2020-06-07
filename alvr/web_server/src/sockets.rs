use alvr_common::{data::*, logging::*, *};
use std::{
    ffi::CStr,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::net::*;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;
const HANDSHAKE_PORT: u16 = 9943;
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);

pub async fn search_client(
    client_ip: Option<String>,
) -> StrResult<(IpAddr, ClientHandshakePacket)> {
    let mut listener =
        trace_err!(UdpSocket::bind(SocketAddr::new(LOCAL_IP, HANDSHAKE_PORT)).await)?;
    trace_err!(listener.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED))?;

    let maybe_target_client_ip = match client_ip {
        Some(ip_str) => Some(trace_err!(ip_str.parse::<IpAddr>(), "Client IP")?),
        None => None,
    };

    let mut packet_buffer = [0u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES];

    loop {
        let (hanshake_packet_size, address) = match listener.recv_from(&mut packet_buffer).await {
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

        let client_handshake_packet: ClientHandshakePacket =
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

        if client_handshake_packet.alvr_name != [b'A', b'L', b'V', b'R'] {
            warn!(
                id: LogId::ClientFoundInvalid,
                "Handshake packet has wrong name"
            );
            continue;
        }

        let maybe_compatible_condition =
            CStr::from_bytes_with_nul(&client_handshake_packet.version)
                .map_err(|e| e.to_string())
                .and_then(|client_version_cstr| {
                    is_version_compatible(
                        &client_version_cstr.to_string_lossy(),
                        ALVR_CLIENT_VERSION_REQ,
                    )
                });
        match maybe_compatible_condition {
            Ok(compatible) => {
                if !compatible {
                    let version_c_str =
                        CStr::from_bytes_with_nul(&client_handshake_packet.version).unwrap();
                    warn!(id: LogId::ClientFoundWrongVersion(
                        version_c_str.to_string_lossy().into_owned()
                    ));
                    continue;
                }
            }
            Err(e) => {
                warn!(id: LogId::ClientFoundInvalid, "{}", e);
                continue;
            }
        }

        break Ok((address.ip(), client_handshake_packet));
    }
}
