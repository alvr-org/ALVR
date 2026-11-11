use alvr_common::anyhow::{Result, bail};
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub struct AnnouncerSocket {
    hostname: String,
    daemon: ServiceDaemon,
    control_port: u16,
}

impl AnnouncerSocket {
    /// Announces on the well-known [`alvr_sockets::CONTROL_PORT`].
    #[allow(dead_code)] // Kept for callers that never need a custom control port.
    pub fn new(hostname: &str) -> Result<Self> {
        Self::new_with_control_port(hostname, alvr_sockets::CONTROL_PORT)
    }

    /// Announces a control port other than the well-known [`alvr_sockets::CONTROL_PORT`].
    ///
    /// Only one process per machine can own the well-known port, so clients that need to share a
    /// machine (several emulated headsets, for instance) bind an OS-assigned port instead and
    /// advertise it here. Servers that do not understand the advertised port fall back to the
    /// well-known one, which is why [`Self::new`] stays byte-compatible with older releases.
    pub fn new_with_control_port(hostname: &str, control_port: u16) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;

        Ok(Self {
            daemon,
            hostname: hostname.to_owned(),
            control_port,
        })
    }

    pub fn announce(&self) -> Result<()> {
        let local_ip = alvr_system_info::local_ip();
        if local_ip.is_unspecified() {
            bail!("IP is unspecified");
        }

        let mut properties = vec![(
            alvr_sockets::MDNS_PROTOCOL_KEY,
            alvr_common::protocol_id().to_string(),
        )];

        // Advertised only when it differs from the well-known port, so the records published by
        // ordinary clients are unchanged.
        if self.control_port != alvr_sockets::CONTROL_PORT {
            properties.push((
                alvr_sockets::MDNS_CONTROL_PORT_KEY,
                self.control_port.to_string(),
            ));
        }

        let properties = properties
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect::<Vec<_>>();

        self.daemon.register(ServiceInfo::new(
            alvr_sockets::MDNS_SERVICE_TYPE,
            &format!("alvr{}", rand::random::<u16>()),
            &self.hostname,
            local_ip,
            5353,
            &properties[..],
        )?)?;

        Ok(())
    }
}

impl Drop for AnnouncerSocket {
    fn drop(&mut self) {
        // The daemon owns a thread parked in a blocking receive that only stops on shutdown.
        // Without this the thread outlives the socket and any join on the connection thread that
        // owns it never returns, which hangs process exit.
        self.daemon.shutdown().ok();
    }
}
