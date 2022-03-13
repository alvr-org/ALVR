use alvr_common::prelude::*;
use alvr_session::{ServerEvent, SessionDesc};
use alvr_sockets::{AudioDevicesList, GpuVendor};
use cpal::traits::{DeviceTrait, HostTrait};
use serde_json as json;
use std::{
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
use wgpu::Adapter;

pub fn load_session(path: &Path) -> StrResult<SessionDesc> {
    json::from_str(&fs::read_to_string(path).map_err(err!())?).map_err(err!())
}

pub fn save_session(session_desc: &SessionDesc, path: &Path) -> StrResult {
    fs::write(path, json::to_string_pretty(session_desc).map_err(err!())?).map_err(err!())
}

// SessionDesc wrapper that saves settings.json and session.json on destruction.
pub struct SessionLock<'a> {
    session_desc: &'a mut SessionDesc,
    session_path: &'a Path,
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
        alvr_session::log_event(ServerEvent::SessionUpdated); // deprecated
        alvr_session::log_event(ServerEvent::Session(Box::new(self.session_desc.clone())));
    }
}

// Correct usage:
// SessionManager should be used behind a Mutex. Each write of the session should be preceded by a
// read, within the same lock.
// fixme: the dashboard is doing this wrong because it is holding its own session state. If read and
// write need to happen on separate threads, a critical region should be implemented.
pub struct ServerDataManager {
    session: SessionDesc,
    session_path: PathBuf,
    gpu_adapters: Vec<Adapter>,
}

impl ServerDataManager {
    pub fn new(session_path: &Path) -> Self {
        let config_dir = session_path.parent().unwrap();
        fs::create_dir_all(config_dir).ok();

        let session_desc = match fs::read_to_string(&session_path) {
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
            let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

            instance
                .enumerate_adapters(wgpu::Backends::PRIMARY)
                .collect()
        };

        Self {
            session: session_desc,
            session_path: session_path.to_owned(),
            gpu_adapters,
        }
    }

    pub fn session(&self) -> &SessionDesc {
        &self.session
    }

    pub fn session_mut(&mut self) -> SessionLock {
        SessionLock {
            session_desc: &mut self.session,
            session_path: &self.session_path,
        }
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

    pub fn get_gpu_names(&self) -> Vec<String> {
        self.gpu_adapters
            .iter()
            .map(|a| a.get_info().name)
            .collect::<Vec<_>>()
    }

    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
    pub fn get_audio_devices_list(&self) -> StrResult<AudioDevicesList> {
        #[cfg(target_os = "linux")]
        let host = match self.session.session_settings.audio.linux_backend {
            LinuxAudioBackend::Alsa => cpal::host_from_id(cpal::HostId::Alsa),
            LinuxAudioBackend::Jack => cpal::host_from_id(cpal::HostId::Jack),
        }
        .map_err(err!());
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
