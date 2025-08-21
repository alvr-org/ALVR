#[cfg(target_os = "linux")]
mod linux_steamvr;
#[cfg(windows)]
mod windows_steamvr;

use crate::data_sources;
use alvr_adb::commands as adb;
use alvr_common::{
    anyhow::{Context, Result},
    debug,
    glam::bool,
    parking_lot::Mutex,
    warn,
};
use alvr_filesystem as afs;
use serde_json::{self, json};
use std::{
    ffi::OsStr,
    fs,
    marker::PhantomData,
    process::Command,
    slice, thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessesToUpdate, System};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const DRIVER_KEY: &str = "driver_alvr_server";
const BLOCKED_KEY: &str = "blocked_by_safe_mode";

pub fn is_server_running() -> bool {
    #[cfg(feature = "steamvr-server")]
    let process_name = afs::exec_fname("vrserver");
    #[cfg(feature = "mock-server")]
    let process_name = afs::mock_server_fname();

    System::new_all()
        .processes_by_name(OsStr::new(&process_name))
        .count()
        != 0
}

pub fn maybe_kill_server() {
    let mut system = System::new_all();

    #[cfg(feature = "steamvr-server")]
    let process_names = [afs::exec_fname("vrmonitor"), afs::exec_fname("vrserver")];
    #[cfg(feature = "mock-server")]
    let process_names = [afs::mock_server_fname()];

    #[cfg_attr(target_os = "macos", expect(unused_variables))]
    for process_name in process_names {
        system.refresh_processes(ProcessesToUpdate::All, true);

        for process in system.processes_by_name(OsStr::new(&process_name)) {
            debug!("Killing {}", process_name);

            #[cfg(target_os = "linux")]
            linux_steamvr::terminate_process(process);
            #[cfg(windows)]
            windows_steamvr::kill_process(process.pid().as_u32());

            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn unblock_alvr_driver() -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Ok(());
    }

    let path = alvr_server_io::steamvr_settings_file_path()?;
    let text = fs::read_to_string(&path).with_context(|| format!("Failed to read {path:?}"))?;
    let new_text = unblock_alvr_driver_within_vrsettings(text.as_str())
        .with_context(|| "Failed to rewrite .vrsettings.")?;
    fs::write(&path, new_text)
        .with_context(|| "Failed to write .vrsettings back after changing it.")?;
    Ok(())
}

// Reads and writes back steamvr.vrsettings in order to
// ensure the ALVR driver is not blocked (safe mode).
fn unblock_alvr_driver_within_vrsettings(text: &str) -> Result<String> {
    let mut settings = serde_json::from_str::<serde_json::Value>(text)?;
    let values = settings
        .as_object_mut()
        .with_context(|| "Failed to parse .vrsettings.")?;
    let blocked = values
        .get(DRIVER_KEY)
        .and_then(|driver| driver.get(BLOCKED_KEY))
        .and_then(|blocked| blocked.as_bool())
        .unwrap_or(false);

    if blocked {
        debug!("Unblocking ALVR driver in SteamVR.");
        if !values.contains_key(DRIVER_KEY) {
            values.insert(DRIVER_KEY.into(), json!({}));
        }
        let driver = settings[DRIVER_KEY]
            .as_object_mut()
            .with_context(|| "Did not find ALVR key in settings.")?;
        driver.insert(BLOCKED_KEY.into(), json!(false)); // overwrites if present
    } else {
        debug!("ALVR is not blocked in SteamVR.");
    }

    Ok(serde_json::to_string_pretty(&settings)?)
}

pub struct Launcher {
    _phantom: PhantomData<()>,
}

impl Launcher {
    pub fn launch_server(&self) {
        let filesystem_layout = crate::get_filesystem_layout();

        // The ADB server might be left running because of a unclean termination of SteamVR
        // Note that this will also kill a system wide ADB server not started by ALVR
        let wired_enabled = data_sources::get_read_only_local_session()
            .session()
            .client_connections
            .contains_key(alvr_sockets::WIRED_CLIENT_HOSTNAME);
        if wired_enabled && let Some(path) = adb::get_adb_path(&filesystem_layout) {
            adb::kill_server(&path).ok();
        }

        if cfg!(feature = "steamvr-server") {
            #[cfg(target_os = "linux")]
            linux_steamvr::linux_hardware_checks();

            let alvr_driver_dir = &filesystem_layout.openvr_driver_root_dir;

            // Make sure to unregister any other ALVR driver because it would cause a socket conflict
            let other_alvr_dirs = alvr_server_io::get_registered_drivers()
                .unwrap_or_default()
                .into_iter()
                .filter(|path| {
                    path.to_string_lossy().to_lowercase().contains("alvr")
                        && path != alvr_driver_dir
                })
                .collect::<Vec<_>>();
            alvr_server_io::driver_registration(&other_alvr_dirs, false).ok();

            alvr_server_io::driver_registration(slice::from_ref(alvr_driver_dir), true).ok();

            if let Err(err) = unblock_alvr_driver() {
                warn!("Failed to unblock ALVR driver: {:?}", err);
            }

            #[cfg(target_os = "linux")]
            {
                let vrcompositor_wrap_result = linux_steamvr::maybe_wrap_vrcompositor_launcher();
                alvr_common::show_err(linux_steamvr::maybe_wrap_vrcompositor_launcher());
                if vrcompositor_wrap_result.is_err() {
                    return;
                }
            }
        }

        if !is_server_running() {
            debug!("Server is dead. Launching...");

            if cfg!(feature = "steamvr-server") {
                #[cfg(windows)]
                windows_steamvr::start_steamvr();

                #[cfg(target_os = "linux")]
                linux_steamvr::start_steamvr();
            } else if cfg!(feature = "mock-server") {
                Command::new(filesystem_layout.mock_server_exe())
                    .spawn()
                    .ok();
            }
        }
    }

    pub fn ensure_server_shutdown(&self) {
        debug!("Waiting for server to shutdown...");
        let start_time = Instant::now();
        while start_time.elapsed() < SHUTDOWN_TIMEOUT && is_server_running() {
            thread::sleep(Duration::from_millis(500));
        }

        maybe_kill_server();
    }

    pub fn restart_server(&self) {
        self.ensure_server_shutdown();
        self.launch_server();
    }
}

// Singleton with exclusive access
pub static LAUNCHER: Mutex<Launcher> = Mutex::new(Launcher {
    _phantom: PhantomData,
});
