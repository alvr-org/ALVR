#[cfg(target_os = "linux")]
mod linux_steamvr;
#[cfg(windows)]
mod windows_steamvr;

use crate::data_sources;
use alvr_common::{
    anyhow::{Context, Result},
    debug,
    glam::bool,
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    warn,
};
use alvr_filesystem as afs;
use alvr_session::{DriverLaunchAction, DriversBackup};
use serde_json::{self, json};
use std::{
    env,
    ffi::OsStr,
    fs,
    marker::PhantomData,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessesToUpdate, System};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const DRIVER_KEY: &str = "driver_alvr_server";
const BLOCKED_KEY: &str = "blocked_by_safe_mode";

pub fn is_steamvr_running() -> bool {
    System::new_all()
        .processes_by_name(OsStr::new(&afs::exec_fname("vrserver")))
        .count()
        != 0
}

pub fn maybe_kill_steamvr() {
    let mut system = System::new_all();

    #[allow(unused_variables)]
    for process in system.processes_by_name(OsStr::new(&afs::exec_fname("vrmonitor"))) {
        debug!("Killing vrmonitor");

        #[cfg(target_os = "linux")]
        linux_steamvr::terminate_process(process);
        #[cfg(windows)]
        windows_steamvr::kill_process(process.pid().as_u32());

        thread::sleep(Duration::from_secs(1));
    }

    system.refresh_processes(ProcessesToUpdate::All, true);

    #[allow(unused_variables)]
    for process in system.processes_by_name(OsStr::new(&afs::exec_fname("vrserver"))) {
        debug!("Killing vrserver");

        #[cfg(target_os = "linux")]
        linux_steamvr::terminate_process(process);
        #[cfg(windows)]
        windows_steamvr::kill_process(process.pid().as_u32());

        thread::sleep(Duration::from_secs(1));
    }
}

fn unblock_alvr_driver() -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Ok(());
    }

    let path = alvr_server_io::steamvr_settings_file_path()?;
    let text = fs::read_to_string(&path).with_context(|| format!("Failed to read {:?}", path))?;
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
    pub fn launch_steamvr(&self) {
        #[cfg(target_os = "linux")]
        linux_steamvr::linux_hardware_checks();

        let mut data_source = data_sources::get_local_session_source();

        let launch_action = &data_source
            .settings()
            .extra
            .steamvr_launcher
            .driver_launch_action;

        if !matches!(launch_action, DriverLaunchAction::NoAction) {
            let other_drivers_paths = if matches!(
                launch_action,
                DriverLaunchAction::UnregisterOtherDriversAtStartup
            ) && data_source.session().drivers_backup.is_none()
            {
                let drivers_paths = alvr_server_io::get_registered_drivers().unwrap_or_default();

                alvr_server_io::driver_registration(&drivers_paths, false).ok();

                drivers_paths
            } else {
                vec![]
            };
            let alvr_driver_dir =
                afs::filesystem_layout_from_dashboard_exe(&env::current_exe().unwrap())
                    .openvr_driver_root_dir;

            alvr_server_io::driver_registration(&[alvr_driver_dir.clone()], true).ok();

            data_source.session_mut().drivers_backup = Some(DriversBackup {
                alvr_path: alvr_driver_dir,
                other_paths: other_drivers_paths,
            });
        }

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

        if !is_steamvr_running() {
            debug!("SteamVR is dead. Launching...");

            #[cfg(windows)]
            windows_steamvr::start_steamvr();

            #[cfg(target_os = "linux")]
            linux_steamvr::start_steamvr();
        }
    }

    pub fn ensure_steamvr_shutdown(&self) {
        debug!("Waiting for SteamVR to shutdown...");
        let start_time = Instant::now();
        while start_time.elapsed() < SHUTDOWN_TIMEOUT && is_steamvr_running() {
            thread::sleep(Duration::from_millis(500));
        }

        maybe_kill_steamvr();
    }

    pub fn restart_steamvr(&self) {
        self.ensure_steamvr_shutdown();
        self.launch_steamvr();
    }
}

// Singleton with exclusive access
pub static LAUNCHER: Lazy<Mutex<Launcher>> = Lazy::new(|| {
    Mutex::new(Launcher {
        _phantom: PhantomData,
    })
});
