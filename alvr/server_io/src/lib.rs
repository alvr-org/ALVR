mod firewall;
mod openvr_drivers;
mod openvrpaths;

pub use firewall::*;
pub use openvr_drivers::*;
pub use openvrpaths::*;

use alvr_common::{
    anyhow::{bail, Result},
    error, info, ConnectionState,
};
use alvr_events::EventType;
use alvr_packets::{AudioDevicesList, ClientListAction, PathSegment, PathValuePair};
use alvr_session::{ClientConnectionConfig, SessionConfig, Settings};
use serde_json as json;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Debug},
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

fn save_session(session: &SessionConfig, path: &Path) -> Result<()> {
    fs::write(path, json::to_string_pretty(session)?)?;

    Ok(())
}

// SessionConfig wrapper that saves session.json on destruction.
pub struct SessionLock<'a> {
    session_desc: &'a mut SessionConfig,
    session_path: Option<&'a Path>,
    settings: &'a mut Settings,
}

impl Deref for SessionLock<'_> {
    type Target = SessionConfig;
    fn deref(&self) -> &SessionConfig {
        self.session_desc
    }
}

impl DerefMut for SessionLock<'_> {
    fn deref_mut(&mut self) -> &mut SessionConfig {
        self.session_desc
    }
}

impl Drop for SessionLock<'_> {
    fn drop(&mut self) {
        if let Some(session_path) = self.session_path {
            save_session(self.session_desc, session_path).ok();
        }

        *self.settings = self.session_desc.to_settings();
        alvr_events::send_event(EventType::Session(Box::new(self.session_desc.clone())));
    }
}

// Correct usage:
// SessionManager should be used behind a Mutex. Each write of the session should be preceded by a
// read, within the same lock.
// fixme: the dashboard is doing this wrong because it is holding its own session state. If read and
// write need to happen on separate threads, a critical region should be implemented.
pub struct ServerSessionManager {
    session_config: SessionConfig,
    settings: Settings,
    session_path: Option<PathBuf>,
}

impl ServerSessionManager {
    pub fn new(session_path: Option<PathBuf>) -> Self {
        let session_config = if let Some(session_path) = &session_path {
            let config_dir = session_path.parent().unwrap();
            fs::create_dir_all(config_dir).ok();
            Self::load_session(session_path, config_dir)
        } else {
            SessionConfig::default()
        };

        Self {
            session_config: session_config.clone(),
            settings: session_config.to_settings(),
            session_path: session_path.to_owned(),
        }
    }

    fn load_session(session_path: &Path, config_dir: &Path) -> SessionConfig {
        let session_string = fs::read_to_string(session_path).unwrap_or_default();

        if session_string.is_empty() {
            return SessionConfig::default();
        }

        let session_json = json::from_str::<json::Value>(&session_string)
            .unwrap_or_else(|e| {
                error!(
                    "{} {} {}\n{}",
                    "Failed to load session.json.",
                    "Its contents will be reset and the original file content stored as session_invalid.json.",
                    "See error message below for details:",
                    e
                );
                json::Value::Null
            });

        if session_json.is_null() {
            fs::write(config_dir.join("session_invalid.json"), &session_string).ok();
            return SessionConfig::default();
        }

        json::from_value(session_json.clone()).unwrap_or_else(|_| {
            fs::write(config_dir.join("session_old.json"), &session_string).ok();
            let mut session_desc = SessionConfig::default();
            match session_desc.merge_from_json(&session_json) {
                Ok(_) => info!(
                    "{} {}",
                    "Session extrapolated successfully.",
                    "Old session.json is stored as session_old.json"
                ),
                Err(e) => error!(
                    "{} {} {}",
                    "Error while extrapolating session.",
                    "Old session.json is stored as session_old.json.",
                    e
                ),
            }
            // not essential, but useful to avoid duplicated errors
            save_session(&session_desc, session_path).ok();

            session_desc
        })
    }

    // prefer settings()
    pub fn session(&self) -> &SessionConfig {
        &self.session_config
    }

    pub fn session_mut(&mut self) -> SessionLock {
        SessionLock {
            session_desc: &mut self.session_config,
            session_path: self.session_path.as_deref(),
            settings: &mut self.settings,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    // Note: "value" can be any session subtree, in json format.
    pub fn set_values(&mut self, descs: Vec<PathValuePair>) -> Result<()> {
        let mut session_json = serde_json::to_value(self.session_config.clone()).unwrap();

        for desc in descs {
            let mut session_ref = &mut session_json;
            for segment in &desc.path {
                session_ref = match segment {
                    PathSegment::Name(name) => {
                        if let Some(name) = session_ref.get_mut(name) {
                            name
                        } else {
                            bail!("From path {:?}: segment \"{name}\" not found", desc.path);
                        }
                    }
                    PathSegment::Index(index) => {
                        if let Some(index) = session_ref.get_mut(index) {
                            index
                        } else {
                            bail!("From path {:?}: segment [{index}] not found", desc.path);
                        }
                    }
                };
            }
            *session_ref = desc.value.clone();
        }

        // session_json has been updated
        self.session_config = serde_json::from_value(session_json)?;
        self.settings = self.session_config.to_settings();

        if let Some(session_path) = &self.session_path {
            save_session(&self.session_config, session_path)?;
        }

        alvr_events::send_event(EventType::Session(Box::new(self.session_config.clone())));

        Ok(())
    }

    pub fn client_list(&self) -> &HashMap<String, ClientConnectionConfig> {
        &self.session_config.client_connections
    }

    pub fn update_client_list(&mut self, hostname: String, action: ClientListAction) {
        let mut client_connections = self.session_config.client_connections.clone();

        let maybe_client_entry = client_connections.entry(hostname);

        let mut updated = false;
        match action {
            ClientListAction::AddIfMissing {
                trusted,
                manual_ips,
            } => {
                if let Entry::Vacant(new_entry) = maybe_client_entry {
                    let client_connection_desc = ClientConnectionConfig {
                        display_name: "Unknown".into(),
                        current_ip: None,
                        manual_ips: manual_ips.into_iter().collect(),
                        trusted,
                        connection_state: ConnectionState::Disconnected,
                    };
                    new_entry.insert(client_connection_desc);

                    updated = true;
                }
            }
            ClientListAction::SetDisplayName(name) => {
                if let Entry::Occupied(mut entry) = maybe_client_entry {
                    entry.get_mut().display_name = name;

                    updated = true;
                }
            }
            ClientListAction::Trust => {
                if let Entry::Occupied(mut entry) = maybe_client_entry {
                    entry.get_mut().trusted = true;

                    updated = true;
                }
            }
            ClientListAction::SetManualIps(ips) => {
                if let Entry::Occupied(mut entry) = maybe_client_entry {
                    entry.get_mut().manual_ips = ips.into_iter().collect();

                    updated = true;
                }
            }
            ClientListAction::RemoveEntry => {
                if let Entry::Occupied(entry) = maybe_client_entry {
                    entry.remove_entry();

                    updated = true;
                }
            }
            ClientListAction::UpdateCurrentIp(current_ip) => {
                if let Entry::Occupied(mut entry) = maybe_client_entry {
                    if entry.get().current_ip != current_ip {
                        entry.get_mut().current_ip = current_ip;

                        updated = true;
                    }
                }
            }
            ClientListAction::SetConnectionState(state) => {
                if let Entry::Occupied(mut entry) = maybe_client_entry {
                    if entry.get().connection_state != state {
                        entry.get_mut().connection_state = state;

                        updated = true;
                    }
                }
            }
        }

        if updated {
            self.session_config.client_connections = client_connections;

            if let Some(session_path) = &self.session_path {
                save_session(&self.session_config, session_path).ok();
            }
            alvr_events::send_event(EventType::Session(Box::new(self.session_config.clone())));
        }
    }

    pub fn client_hostnames(&self) -> Vec<String> {
        self.session_config
            .client_connections
            .keys()
            .cloned()
            .collect()
    }

    // Run at the start of dashboard or server
    pub fn clean_client_list(&mut self) {
        let connections = self.client_list().clone();
        for (hostname, connection) in connections {
            if connection.trusted {
                self.update_client_list(
                    hostname,
                    ClientListAction::SetConnectionState(ConnectionState::Disconnected),
                )
            } else {
                self.update_client_list(hostname, ClientListAction::RemoveEntry);
            }
        }

        for hostname in self.client_hostnames() {
            self.update_client_list(hostname.clone(), ClientListAction::UpdateCurrentIp(None));
        }
    }

    pub fn get_audio_devices_list(&self) -> Result<AudioDevicesList> {
        #[cfg(not(target_os = "linux"))]
        {
            use cpal::traits::{DeviceTrait, HostTrait};

            let host = cpal::default_host();

            let output = host
                .output_devices()?
                .filter_map(|d| d.name().ok())
                .collect::<Vec<_>>();
            let input = host
                .input_devices()?
                .filter_map(|d| d.name().ok())
                .collect::<Vec<_>>();

            Ok(AudioDevicesList { output, input })
        }
        #[cfg(target_os = "linux")]
        {
            Ok(AudioDevicesList {
                input: vec![],
                output: vec![],
            })
        }
    }
}

impl Debug for ServerSessionManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.session_path)
    }
}
