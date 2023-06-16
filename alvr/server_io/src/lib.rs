mod firewall;
mod openvr_drivers;
mod openvrpaths;

pub use firewall::*;
pub use openvr_drivers::*;
pub use openvrpaths::*;

use alvr_common::prelude::*;
use alvr_events::EventType;
use alvr_packets::{AudioDevicesList, ClientListAction, GpuVendor, PathSegment, PathValuePair};
use alvr_session::{ClientConnectionConfig, ConnectionState, SessionConfig, Settings};
use cpal::traits::{DeviceTrait, HostTrait};
use serde_json as json;
use std::{
    collections::{hash_map::Entry, HashMap},
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
use wgpu::AdapterInfo;

fn save_session(session: &SessionConfig, path: &Path) -> StrResult {
    fs::write(path, json::to_string_pretty(session).map_err(err!())?).map_err(err!())
}

// SessionConfig wrapper that saves session.json on destruction.
pub struct SessionLock<'a> {
    session_desc: &'a mut SessionConfig,
    session_path: &'a Path,
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
        save_session(self.session_desc, self.session_path).unwrap();
        *self.settings = self.session_desc.to_settings();
        alvr_events::send_event(EventType::Session(Box::new(self.session_desc.clone())));
    }
}

// Correct usage:
// SessionManager should be used behind a Mutex. Each write of the session should be preceded by a
// read, within the same lock.
// fixme: the dashboard is doing this wrong because it is holding its own session state. If read and
// write need to happen on separate threads, a critical region should be implemented.
pub struct ServerDataManager {
    session: SessionConfig,
    settings: Settings,
    session_path: PathBuf,
    gpu_infos: Vec<AdapterInfo>,
}

impl ServerDataManager {
    pub fn new(session_path: &Path) -> Self {
        let config_dir = session_path.parent().unwrap();
        fs::create_dir_all(config_dir).ok();
        let session_desc = Self::load_session(session_path, config_dir);

        let vk_adapters: Vec<wgpu::Adapter> = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
        })
        .enumerate_adapters(wgpu::Backends::VULKAN)
        .collect();

        let gpu_infos = vk_adapters
            .iter()
            .map(|adapter| adapter.get_info())
            .collect();

        Self {
            session: session_desc.clone(),
            settings: session_desc.to_settings(),
            session_path: session_path.to_owned(),
            gpu_infos,
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
        &self.session
    }

    pub fn session_mut(&mut self) -> SessionLock {
        SessionLock {
            session_desc: &mut self.session,
            session_path: &self.session_path,
            settings: &mut self.settings,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    // Note: "value" can be any session subtree, in json format.
    pub fn set_values(&mut self, descs: Vec<PathValuePair>) -> StrResult {
        let mut session_json = serde_json::to_value(self.session.clone()).unwrap();

        for desc in descs {
            let mut session_ref = &mut session_json;
            for segment in &desc.path {
                session_ref = match segment {
                    PathSegment::Name(name) => {
                        if let Some(name) = session_ref.get_mut(name) {
                            name
                        } else {
                            return fmt_e!(
                                "From path {:?}: segment \"{}\" not found",
                                desc.path,
                                name
                            );
                        }
                    }
                    PathSegment::Index(index) => {
                        if let Some(index) = session_ref.get_mut(index) {
                            index
                        } else {
                            return fmt_e!(
                                "From path {:?}: segment [{}] not found",
                                desc.path,
                                index
                            );
                        }
                    }
                };
            }
            *session_ref = desc.value.clone();
        }

        // session_json has been updated
        self.session = serde_json::from_value(session_json).map_err(err!())?;
        self.settings = self.session.to_settings();

        save_session(&self.session, &self.session_path).unwrap();
        alvr_events::send_event(EventType::Session(Box::new(self.session.clone())));

        Ok(())
    }

    pub fn client_list(&self) -> &HashMap<String, ClientConnectionConfig> {
        &self.session.client_connections
    }

    pub fn update_client_list(&mut self, hostname: String, action: ClientListAction) {
        let mut client_connections = self.session.client_connections.clone();

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
            self.session.client_connections = client_connections;

            save_session(&self.session, &self.session_path).unwrap();
            alvr_events::send_event(EventType::Session(Box::new(self.session.clone())));
        }
    }

    pub fn get_gpu_vendors(&self) -> Vec<GpuVendor> {
        return self
            .gpu_infos
            .iter()
            .map(|adapter_info| match adapter_info.vendor {
                0x10de => GpuVendor::Nvidia,
                0x1002 => GpuVendor::Amd,
                _ => GpuVendor::Other,
            })
            .collect();
    }

    pub fn get_gpu_names(&self) -> Vec<String> {
        return self
            .gpu_infos
            .iter()
            .map(|adapter_info| adapter_info.name.clone())
            .collect();
    }

    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
    pub fn get_audio_devices_list(&self) -> StrResult<AudioDevicesList> {
        #[cfg(target_os = "linux")]
        let host = match self.session.to_settings().audio.linux_backend {
            alvr_session::LinuxAudioBackend::Alsa => cpal::host_from_id(cpal::HostId::Alsa),
            alvr_session::LinuxAudioBackend::Jack => cpal::host_from_id(cpal::HostId::Jack),
        }
        .map_err(err!())?;
        #[cfg(not(target_os = "linux"))]
        let host = cpal::default_host();

        let output = host
            .output_devices()
            .map_err(err!())?
            .filter_map(|d| d.name().ok())
            .collect::<Vec<_>>();
        let input = host
            .input_devices()
            .map_err(err!())?
            .filter_map(|d| d.name().ok())
            .collect::<Vec<_>>();

        Ok(AudioDevicesList { output, input })
    }
}
