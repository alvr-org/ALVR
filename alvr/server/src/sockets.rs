use alvr_common::{
    anyhow::{bail, Result},
    warn, ToAny,
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

    // Returns: client IP, client hostname
    pub fn recv_all(&mut self) -> Result<HashMap<String, IpAddr>> {
        let mut clients = HashMap::new();

        loop {
            match self.mdns_receiver.try_recv() {
                Ok(event) => {
                    if let ServiceEvent::ServiceResolved(info) = event {
                        let hostname = info
                            .get_property_val_str(alvr_sockets::MDNS_DEVICE_ID_KEY)
                            .unwrap_or_else(|| info.get_hostname());
                        let address = *info.get_addresses().iter().next().to_any()?;

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
