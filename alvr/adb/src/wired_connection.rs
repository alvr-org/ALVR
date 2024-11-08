use alvr_common::{dbg_connection, error};

use crate::kill_server;

pub struct WiredConnection {
    pub maybe_adb_path: Option<String>,
    pub status: WiredConnectionStatus,
}

impl Default for WiredConnection {
    fn default() -> Self {
        Self {
            maybe_adb_path: Default::default(),
            status: WiredConnectionStatus::Uninitialized,
        }
    }
}

impl Drop for WiredConnection {
    fn drop(&mut self) {
        let Some(adb_path) = &self.maybe_adb_path else {
            return;
        };
        dbg_connection!("wired_connection: Killing ADB server");
        if let Err(e) = kill_server(&adb_path) {
            error!("{e:?}");
        }
    }
}

pub enum WiredConnectionStatus {
    Uninitialized,
    NotReady(String),
    Ready,
}
