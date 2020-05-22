use alvr_common::{data::*, logging::*, *};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::*;

const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 4_000;
const HANDSHAKE_PORT: u16 = 9943;
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);

async fn try_find_client(
    listener: &mut UdpSocket,
    packet_buffer: &mut [u8],
    target_client_ip: Option<IpAddr>,
) -> Result<(IpAddr, ClientHandshakePacket), ()> {
    let (hanshake_packet_size, address) = listener
        .recv_from(packet_buffer)
        .await
        .map_err(|e| debug!("Error receiving handshake packet: {}", e))?;

    if let Some(ip) = target_client_ip {
        if address.ip() != ip {
            info!(id: LogId::ClientFoundWrongIp);
            return Err(());
        }
    }

    let client_handshake_packet: ClientHandshakePacket =
        bincode::deserialize(&packet_buffer[..hanshake_packet_size]).map_err(|e| {
            warn!(
                id: LogId::ClientFoundInvalid,
                "Received handshake packet: {}", e
            )
        })?;

    if client_handshake_packet.alvr_name != ALVR_NAME {
        warn!(
            id: LogId::ClientFoundInvalid,
            "Handshake packet has wrong name"
        );
        return Err(());
    }

    if is_version_compatible(&client_handshake_packet.version, ALVR_CLIENT_VERSION_REQ).unwrap() {
        warn!(
            id: LogId::ClientFoundWrongVersion(client_handshake_packet.version),
            "Handshake packet is invalid"
        );
        return Err(());
    }

    Ok((address.ip(), client_handshake_packet))
}

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
        if let Ok(pair) =
            try_find_client(&mut listener, &mut packet_buffer, maybe_target_client_ip).await
        {
            break Ok(pair);
        }
    }
}
