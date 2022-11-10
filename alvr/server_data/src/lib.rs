use alvr_common::prelude::*;
use alvr_events::EventType;
use alvr_session::{ClientConnectionDesc, SessionDesc, Settings};
use alvr_sockets::{AudioDevicesList, ClientListAction, GpuVendor, PathSegment};
use cpal::traits::{DeviceTrait, HostTrait};
use serde_json as json;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
use wgpu::Adapter;

fn save_session(session: &SessionDesc, path: &Path) -> StrResult {
    fs::write(path, json::to_string_pretty(session).map_err(err!())?).map_err(err!())
}

// SessionDesc wrapper that saves settings.json and session.json on destruction.
pub struct SessionLock<'a> {
    session_desc: &'a mut SessionDesc,
    session_path: &'a Path,
    settings: &'a mut Settings,
}

impl Deref for SessionLock<'_> {
    type Target = SessionDesc;
    fn deref(&self) -> &SessionDesc {
        self.session_desc
    }
}

impl DerefMut for SessionLock<'_> {
    fn deref_mut(&mut self) -> &mut SessionDesc {
        self.session_desc
    }
}

impl Drop for SessionLock<'_> {
    fn drop(&mut self) {
        save_session(self.session_desc, self.session_path).unwrap();
        *self.settings = self.session_desc.to_settings();
        alvr_events::send_event(EventType::SessionUpdated); // deprecated
        alvr_events::send_event(EventType::Session(Box::new(self.session_desc.clone())));
    }
}

// Correct usage:
// SessionManager should be used behind a Mutex. Each write of the session should be preceded by a
// read, within the same lock.
// fixme: the dashboard is doing this wrong because it is holding its own session state. If read and
// write need to happen on separate threads, a critical region should be implemented.
pub struct ServerDataManager {
    session: SessionDesc,
    settings: Settings,
    session_path: PathBuf,
    script_engine: rhai::Engine,
    gpu_adapters: Vec<Adapter>,
}

impl ServerDataManager {
    pub fn new(session_path: &Path) -> Self {
        let config_dir = session_path.parent().unwrap();
        fs::create_dir_all(config_dir).ok();

        let session_desc = match fs::read_to_string(session_path) {
            Ok(session_string) => {
                let json_value = json::from_str::<json::Value>(&session_string).unwrap();
                match json::from_value(json_value.clone()) {
                    Ok(session_desc) => session_desc,
                    Err(_) => {
                        fs::write(config_dir.join("session_old.json"), &session_string).ok();
                        let mut session_desc = SessionDesc::default();
                        match session_desc.merge_from_json(&json_value) {
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
                    }
                }
            }
            Err(_) => SessionDesc::default(),
        };

        let gpu_adapters = {
            let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);

            instance
                .enumerate_adapters(wgpu::Backends::VULKAN)
                .collect()
        };

        let script_engine = rhai::Engine::new();

        Self {
            session: session_desc.clone(),
            settings: session_desc.to_settings(),
            session_path: session_path.to_owned(),
            script_engine,
            gpu_adapters,
        }
    }

    // prefer settings()
    pub fn session(&self) -> &SessionDesc {
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

    pub fn client_list(&self) -> &HashMap<String, ClientConnectionDesc> {
        &self.session.client_connections
    }

    // Note: "value" can be any session subtree, in json format.
    pub fn set_single_value(&mut self, path: Vec<PathSegment>, value: &str) -> StrResult {
        let mut session_json = serde_json::to_value(self.session.clone()).map_err(err!())?;

        let mut session_ref = &mut session_json;
        for segment in path {
            session_ref = match segment {
                PathSegment::Name(name) => session_ref.get_mut(name).ok_or_else(enone!())?,
                PathSegment::Index(index) => session_ref.get_mut(index).ok_or_else(enone!())?,
            };
        }

        *session_ref = serde_json::from_str(value).map_err(err!())?;

        // session_json has been updated
        self.session = serde_json::from_value(session_json).map_err(err!())?;
        self.settings = self.session.to_settings();

        save_session(&self.session, &self.session_path).unwrap();
        alvr_events::send_event(EventType::Session(Box::new(self.session.clone())));

        Ok(())
    }

    pub fn execute_script(&self, code: &str) -> StrResult<String> {
        // Note: the scope is recreated every time to avoid cross-invocation interference
        let mut scope = rhai::Scope::new();
        scope.push_constant_dynamic(
            "session",
            rhai::serde::to_dynamic(self.session.clone()).unwrap(),
        );

        self.script_engine
            .eval_with_scope::<rhai::Dynamic>(&mut scope, code)
            .map(|d| d.to_string())
            .map_err(|e| e.to_string())
    }

    pub fn get_gpu_vendor(&self) -> GpuVendor {
        if let Some(adapter) = self.gpu_adapters.get(0) {
            match adapter.get_info().vendor {
                0x10de => GpuVendor::Nvidia,
                0x1002 => GpuVendor::Amd,
                _ => GpuVendor::Other,
            }
        } else {
            GpuVendor::Other
        }
    }

    pub fn get_gpu_name(&self) -> String {
        self.gpu_adapters
            .get(0)
            .map(|a| a.get_info().name)
            .unwrap_or_else(|| "".into())
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

    pub fn update_client_list(&mut self, hostname: String, action: ClientListAction) {
        let mut client_connections = self.session.client_connections.clone();

        let maybe_client_entry = client_connections.entry(hostname);

        let mut updated = false;
        match action {
            ClientListAction::AddIfMissing => {
                if let Entry::Vacant(new_entry) = maybe_client_entry {
                    let client_connection_desc = ClientConnectionDesc {
                        trusted: false,
                        manual_ips: HashSet::new(),
                        display_name: "Unknown".into(),
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
            ClientListAction::AddIp(ip) => {
                if let Entry::Occupied(mut entry) = maybe_client_entry {
                    entry.get_mut().manual_ips.insert(ip);

                    updated = true;
                }
            }
            ClientListAction::RemoveIp(ip) => {
                if let Entry::Occupied(mut entry) = maybe_client_entry {
                    entry.get_mut().manual_ips.remove(&ip);

                    updated = true;
                }
            }
            ClientListAction::RemoveEntry => {
                if let Entry::Occupied(entry) = maybe_client_entry {
                    entry.remove_entry();

                    updated = true;
                }
            }
        }

        if updated {
            self.session.client_connections = client_connections;

            save_session(&self.session, &self.session_path).unwrap();
            alvr_events::send_event(EventType::SessionUpdated); // deprecated
            alvr_events::send_event(EventType::Session(Box::new(self.session.clone())));
        }
    }
}
