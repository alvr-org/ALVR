use alvr_common::{anyhow::Result, ALVR_NAME};
use alvr_sockets::{CONTROL_PORT, LOCAL_IP};
use std::net::{Ipv4Addr, UdpSocket};

pub struct AnnouncerSocket {
    socket: UdpSocket,
    packet: [u8; 56],
}

impl AnnouncerSocket {
    pub fn new(hostname: &str) -> Result<Self> {
        let socket = UdpSocket::bind((LOCAL_IP, CONTROL_PORT))?;
        socket.set_broadcast(true)?;

        let mut packet = [0; 56];
        packet[0..ALVR_NAME.len()].copy_from_slice(ALVR_NAME.as_bytes());
        packet[16..24].copy_from_slice(&alvr_common::protocol_id_u64().to_le_bytes());
        packet[24..24 + hostname.len()].copy_from_slice(hostname.as_bytes());

        Ok(Self { socket, packet })
    }

    pub fn announce_broadcast(&self) -> Result<()> {
        self.socket
            .send_to(&self.packet, (Ipv4Addr::BROADCAST, CONTROL_PORT))?;

        Ok(())
    }
}
