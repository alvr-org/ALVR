use alvr_common::{
    anyhow::{bail, Result},
    warn, ConnectionError, HandleTryAgain, ToAny, ALVR_NAME,
};
use alvr_sockets::{CONTROL_PORT, HANDSHAKE_PACKET_SIZE_BYTES, LOCAL_IP};
use flume::TryRecvError;
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};
use std::{
    collections::HashMap,
    net::{IpAddr, UdpSocket},
};

pub struct WelcomeSocket {
    buffer: [u8; HANDSHAKE_PACKET_SIZE_BYTES],
    broadcast_receiver: UdpSocket,
    mdns_receiver: Receiver<ServiceEvent>,
}

impl WelcomeSocket {
    pub fn new() -> Result<Self> {
        let socket = UdpSocket::bind((LOCAL_IP, CONTROL_PORT))?;
        // socket.set_read_timeout(Some(read_timeout))?;
        socket.set_nonblocking(true)?;

        let mdns_receiver = ServiceDaemon::new()?.browse(alvr_sockets::MDNS_SERVICE_TYPE)?;

        Ok(Self {
            buffer: [0; HANDSHAKE_PACKET_SIZE_BYTES],
            broadcast_receiver: socket,
            mdns_receiver,
        })
    }

    // Returns: client IP, client hostname
    pub fn recv_all(&mut self) -> Result<HashMap<String, IpAddr>> {
        let mut clients = HashMap::new();

        loop {
            match self
                .broadcast_receiver
                .recv_from(&mut self.buffer)
                .handle_try_again()
            {
                Ok((size, address)) => {
                    if size == HANDSHAKE_PACKET_SIZE_BYTES
                        && &self.buffer[..ALVR_NAME.len()] == ALVR_NAME.as_bytes()
                        && self.buffer[ALVR_NAME.len()..16].iter().all(|b| *b == 0)
                    {
                        let mut protocol_id_bytes = [0; 8];
                        protocol_id_bytes.copy_from_slice(&self.buffer[16..24]);
                        let received_protocol_id = u64::from_le_bytes(protocol_id_bytes);

                        if received_protocol_id != alvr_common::protocol_id_u64() {
                            warn!(
                                "Found incompatible client! Upgrade or downgrade\n{} {}, {} {}",
                                "Expected protocol ID",
                                alvr_common::protocol_id_u64(),
                                "Found",
                                received_protocol_id
                            );
                        }

                        let mut hostname_bytes = [0; 32];
                        hostname_bytes.copy_from_slice(&self.buffer[24..56]);
                        let hostname = std::str::from_utf8(&hostname_bytes)?
                            .trim_end_matches('\x00')
                            .to_owned();

                        clients.insert(hostname, address.ip());
                    } else if &self.buffer[..16]
                        == b"\x00\x00\x00\x00\x04\x00\x00\x00\x00\x00\x00\x00ALVR"
                        || &self.buffer[..5] == b"\x01ALVR"
                    {
                        warn!("Found old client. Please upgrade")
                    } else {
                        // Unexpected packet.
                        // Note: no need to check for v12 and v13, not found in the wild anymore
                        warn!("Found unrelated packet during discovery")
                    }
                }
                Err(ConnectionError::TryAgain(_)) => break,
                Err(ConnectionError::Other(e)) => return Err(e),
            }
        }

        loop {
            match self.mdns_receiver.try_recv() {
                Ok(event) => {
                    if let ServiceEvent::ServiceResolved(info) = event {
                        let hostname = info.get_hostname();
                        let address = *info.get_addresses().iter().next().to_any()?;

                        let protocol = info
                            .get_property_val_str(alvr_sockets::MDNS_PROTOCOL_KEY)
                            .to_any()?;

                        if protocol != alvr_common::protocol_id() {
                            let msg = format!(
                                r#"Expected protocol ID "{}", found "{}""#,
                                alvr_common::protocol_id(),
                                protocol
                            );
                            warn!(
                                "Found incompatible client {hostname}! Upgrade or downgrade\n{msg}"
                            );
                        }

                        clients.insert(hostname.into(), address);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(e) => bail!(e),
            }
        }

        Ok(clients)
    }
}
