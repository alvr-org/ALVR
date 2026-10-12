use alvr_common::{
    ToAny,
    anyhow::{Result, bail},
    warn,
};
use flume::TryRecvError;
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};
use std::{collections::HashMap, net::IpAddr};

pub struct WelcomeSocket {
    mdns_receiver: Receiver<ServiceEvent>,
}

impl WelcomeSocket {
    pub fn new() -> Result<Self> {
        let mdns_receiver = ServiceDaemon::new()?.browse(alvr_sockets::MDNS_SERVICE_TYPE)?;

        Ok(Self { mdns_receiver })
    }

    /// Returns the discovered clients, keyed by hostname, with the address and control port to
    /// reach each of them on.
    ///
    /// The control port is only advertised by clients that cannot use the well-known
    /// [`alvr_sockets::CONTROL_PORT`], such as several emulated headsets sharing one machine.
    /// Clients that do not advertise one are reached on the well-known port exactly as before, so
    /// older clients keep working against this server.
    pub fn recv_all(&self) -> Result<HashMap<String, (IpAddr, u16)>> {
        let mut clients = HashMap::new();

        loop {
            match self.mdns_receiver.try_recv() {
                Ok(event) => {
                    if let ServiceEvent::ServiceResolved(info) = event {
                        let hostname = info
                            .get_property_val_str(alvr_sockets::MDNS_DEVICE_ID_KEY)
                            .unwrap_or_else(|| info.get_hostname());
                        let address = info.get_addresses().iter().next().to_any()?;

                        let client_protocol = info
                            .get_property_val_str(alvr_sockets::MDNS_PROTOCOL_KEY)
                            .to_any()?;
                        let server_protocol = alvr_common::protocol_id();
                        let client_is_dev = client_protocol.contains("-dev");
                        let server_is_dev = server_protocol.contains("-dev");

                        if client_protocol != server_protocol {
                            let reason = if client_is_dev && server_is_dev {
                                "Please use matching nightly versions."
                            } else if client_is_dev {
                                "Please use nightly server or stable client."
                            } else if server_is_dev {
                                "Please use stable server or nightly client."
                            } else {
                                "Please use matching stable versions."
                            };
                            let protocols = format!(
                                "Protocols: server={server_protocol}, client={client_protocol}"
                            );
                            warn!("Found incompatible client {hostname}! {reason}\n{protocols}");
                        }

                        // A malformed value is treated as absent rather than as a failure: the
                        // well-known port is still the correct guess for any client that did not
                        // deliberately move off it.
                        let control_port = info
                            .get_property_val_str(alvr_sockets::MDNS_CONTROL_PORT_KEY)
                            .and_then(|port| port.parse().ok())
                            .unwrap_or(alvr_sockets::CONTROL_PORT);

                        clients.insert(hostname.into(), (address.to_ip_addr(), control_port));
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(e) => bail!(e),
            }
        }

        Ok(clients)
    }
}
