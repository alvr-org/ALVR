//! Clients and servers exchange discovery packets through UDP broadcast.
//! The packet format is as follows (in order):
//! 16 bytes: name == ALVR
//! 8 bytes: protocol version (hash)
//! 8 bytes: peer type (server or client)
//! N bytes: peer public key, used for authentication

use alvr_common::{prelude::*, ALVR_NAME};
use libp2p::PeerId;
use libp2p_identity::Keypair;
use std::{
    io::ErrorKind,
    mem,
    net::{IpAddr, Ipv4Addr, UdpSocket},
};

#[repr(u64)]
pub enum PeerType {
    Server = 0,
    Client = 1,
}

const DISCOVERY_PORT: u16 = 9943;

const PROTOCOL_NAME_MAX_SIZE: usize = 16;
const PROTOCOL_ID_SIZE: usize = mem::size_of::<u64>();
const PEER_TYPE_SIZE: usize = mem::size_of::<u64>();

const PROTOCOL_ID_OFFSET: usize = PROTOCOL_NAME_MAX_SIZE;
const PEER_TYPE_OFFSET: usize = PROTOCOL_ID_OFFSET + PROTOCOL_ID_SIZE;
const PUBLIC_KEY_OFFSET: usize = PEER_TYPE_OFFSET + PEER_TYPE_SIZE;

pub const MAX_HANDSHAKE_PACKET_SIZE_BYTES: usize = 256; // This should not change between protocols

pub struct DiscoverySocket {
    socket: UdpSocket,
    outgoing_packet: Vec<u8>,
    ingoing_buffer: [u8; MAX_HANDSHAKE_PACKET_SIZE_BYTES],
}

impl DiscoverySocket {
    pub fn new(self_keypair: &[u8], self_peer_type: PeerType) -> StrResult<Self> {
        let socket =
            UdpSocket::bind((IpAddr::V4(Ipv4Addr::UNSPECIFIED), DISCOVERY_PORT)).map_err(err!())?;
        socket.set_nonblocking(true).map_err(err!())?;

        let public_key_bytes = Keypair::from_protobuf_encoding(self_keypair)
            .unwrap()
            .public()
            .to_peer_id()
            .to_bytes();

        let mut outgoing_packet = vec![0; PUBLIC_KEY_OFFSET + public_key_bytes.len()];
        outgoing_packet[0..ALVR_NAME.len()].copy_from_slice(ALVR_NAME.as_bytes());
        outgoing_packet[PROTOCOL_ID_OFFSET..PROTOCOL_ID_OFFSET + PROTOCOL_ID_SIZE]
            .copy_from_slice(&alvr_common::protocol_id().to_le_bytes());
        outgoing_packet[PEER_TYPE_OFFSET..PEER_TYPE_OFFSET + PEER_TYPE_OFFSET]
            .copy_from_slice(&(self_peer_type as u64).to_le_bytes());
        outgoing_packet[PUBLIC_KEY_OFFSET..PUBLIC_KEY_OFFSET + public_key_bytes.len()]
            .copy_from_slice(&public_key_bytes);

        Ok(Self {
            socket,
            outgoing_packet,
            ingoing_buffer: [0; MAX_HANDSHAKE_PACKET_SIZE_BYTES],
        })
    }

    pub fn broadcast(&self) -> StrResult {
        self.socket
            .send_to(&self.outgoing_packet, (Ipv4Addr::BROADCAST, DISCOVERY_PORT))
            .map_err(err!())?;
        Ok(())
    }

    // Returns: client IP, client hostname
    pub fn recv_non_blocking(&mut self) -> IntResult<(PeerType, PeerId, IpAddr)> {
        let (ingoing_packet_size, address) = match self.socket.recv_from(&mut self.ingoing_buffer) {
            Ok(pair) => pair,
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted {
                    return interrupt();
                } else {
                    return int_fmt_e!("{e}");
                }
            }
        };

        if ingoing_packet_size >= PROTOCOL_ID_OFFSET + PROTOCOL_ID_SIZE
            && &self.ingoing_buffer[..ALVR_NAME.len()] == ALVR_NAME.as_bytes()
            && self.ingoing_buffer[ALVR_NAME.len()..PROTOCOL_NAME_MAX_SIZE]
                .iter()
                .all(|b| *b == 0)
        {
            let mut protocol_id_bytes = [0; PROTOCOL_ID_SIZE];
            protocol_id_bytes.copy_from_slice(
                &self.ingoing_buffer[PROTOCOL_ID_OFFSET..PROTOCOL_ID_OFFSET + PROTOCOL_ID_SIZE],
            );
            let received_protocol_id = u64::from_le_bytes(protocol_id_bytes);

            if received_protocol_id != alvr_common::protocol_id() {
                warn!("Found incompatible client! Upgrade or downgrade\nExpected protocol ID {}, Found {received_protocol_id}",
                alvr_common::protocol_id());

                return interrupt();
            }

            // This is the splitting point, where anything before needs not to change between
            // versions to ensure compatibility
            //--------------------------------------------------------------------------------------

            let peer_type = if self.ingoing_buffer[PEER_TYPE_OFFSET] == 0 {
                PeerType::Server
            } else if self.ingoing_buffer[PEER_TYPE_OFFSET] == 1 {
                PeerType::Client
            } else {
                return interrupt();
            };

            let peer_id =
                PeerId::from_bytes(&self.ingoing_buffer[PUBLIC_KEY_OFFSET..ingoing_packet_size])
                    .map_err(int_e!())?;

            Ok((peer_type, peer_id, address.ip()))
        } else {
            // Unexpected packet.
            // Note: no need to check for v12 and v13, not found in the wild anymore
            interrupt()
        }
    }
}
