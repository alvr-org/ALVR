use crate::platform;
use alvr_common::anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub struct AnnouncerSocket {
    daemon: ServiceDaemon,
}

impl AnnouncerSocket {
    pub fn new(hostname: &str) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;

        daemon.register(ServiceInfo::new(
            alvr_sockets::MDNS_SERVICE_TYPE,
            "alvr",
            hostname,
            platform::local_ip(),
            5200,
            &[(
                alvr_sockets::MDNS_PROTOCOL_KEY,
                alvr_common::protocol_id().as_str(),
            )][..],
        )?)?;

        Ok(Self { daemon })
    }
}

impl Drop for AnnouncerSocket {
    fn drop(&mut self) {
        self.daemon.shutdown().ok();
    }
}
