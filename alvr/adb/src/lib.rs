mod commands;
mod parse;

use alvr_common::anyhow::Result;
use alvr_common::{dbg_connection, error};
use std::collections::HashSet;

pub enum WiredConnectionStatus {
    Ready,
    NotReady(String),
}

pub struct WiredConnection {
    adb_path: String,
}

impl WiredConnection {
    pub fn new(
        layout: &alvr_filesystem::Layout,
        download_progress_callback: impl Fn(usize, Option<usize>),
    ) -> Result<Self> {
        let adb_path = commands::require_adb(layout, download_progress_callback)?;

        Ok(Self { adb_path })
    }

    pub fn setup(&self, control_port: u16, stream_port: u16) -> Result<WiredConnectionStatus> {
        let Some(device_serial) = commands::list_devices(&self.adb_path)?
            .into_iter()
            .filter_map(|d| d.serial)
            .find(|s| !s.starts_with("127.0.0.1"))
        else {
            return Ok(WiredConnectionStatus::NotReady(
                "No wired devices found".to_owned(),
            ));
        };

        let ports = HashSet::from([control_port, stream_port]);
        let forwarded_ports: HashSet<u16> =
            commands::list_forwarded_ports(&self.adb_path, &device_serial)?
                .into_iter()
                .map(|f| f.local)
                .collect();
        let missing_ports = ports.difference(&forwarded_ports);
        for port in missing_ports {
            commands::forward_port(&self.adb_path, &device_serial, *port)?;
            dbg_connection!(
                "setup_wired_connection: Forwarded port {port} of device {device_serial}"
            );
        }

        #[cfg(debug_assertions)]
        let process_name = "alvr.client.dev";
        #[cfg(not(debug_assertions))]
        let process_name = "alvr.client.stable";

        if commands::get_process_id(&self.adb_path, &device_serial, process_name)?.is_none() {
            Ok(WiredConnectionStatus::NotReady(
                "ALVR client is not running".to_owned(),
            ))
        } else if !commands::is_activity_resumed(&self.adb_path, &device_serial, process_name)? {
            Ok(WiredConnectionStatus::NotReady(
                "ALVR client is paused".to_owned(),
            ))
        } else {
            Ok(WiredConnectionStatus::Ready)
        }
    }
}

impl Drop for WiredConnection {
    fn drop(&mut self) {
        dbg_connection!("wired_connection: Killing ADB server");
        if let Err(e) = commands::kill_server(&self.adb_path) {
            error!("{e:?}");
        }
    }
}
