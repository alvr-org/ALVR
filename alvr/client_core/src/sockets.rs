use alvr_common::{StrResult, *};
use alvr_sockets::{CONTROL_PORT, LOCAL_IP};
use std::net::{Ipv4Addr, UdpSocket};

pub struct AnnouncerSocket {
    socket: UdpSocket,
    packet: [u8; 56],
}

impl AnnouncerSocket {
    pub fn new(hostname: &str) -> StrResult<Self> {
        let socket = UdpSocket::bind((LOCAL_IP, CONTROL_PORT)).map_err(err!())?;
        socket.set_broadcast(true).map_err(err!())?;

        let mut packet = [0; 56];
        packet[0..ALVR_NAME.len()].copy_from_slice(ALVR_NAME.as_bytes());
        packet[16..24].copy_from_slice(&alvr_common::protocol_id().to_le_bytes());
        packet[24..24 + hostname.len()].copy_from_slice(hostname.as_bytes());

        Ok(Self { socket, packet })
    }

    pub fn broadcast(&self) -> StrResult {
        self.socket
            .send_to(&self.packet, (Ipv4Addr::BROADCAST, CONTROL_PORT))
            .map_err(err!())?;
        Ok(())
    }
}
