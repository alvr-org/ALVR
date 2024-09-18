#[cfg(target_os = "linux")]
mod linux_steamvr;
#[cfg(windows)]
mod windows_steamvr;

use crate::data_sources;
use alvr_common::{debug, glam::bool, once_cell::sync::Lazy, parking_lot::Mutex};
use alvr_filesystem as afs;
use alvr_session::{DriverLaunchAction, DriversBackup};
use std::{
    env,
    ffi::OsStr,
    marker::PhantomData,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

pub fn is_steamvr_running() -> bool {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes(ProcessesToUpdate::All);

    system
        .processes_by_name(OsStr::new(&afs::exec_fname("vrserver")))
        .count()
        != 0
}

pub fn maybe_kill_steamvr() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes(ProcessesToUpdate::All);

    // first kill vrmonitor, then kill vrserver if it is hung.

    #[allow(unused_variables)]
    for process in system.processes_by_name(OsStr::new(&afs::exec_fname("vrmonitor"))) {
        debug!("Killing vrmonitor");

        #[cfg(target_os = "linux")]
        linux_steamvr::terminate_process(process);
        #[cfg(windows)]
        windows_steamvr::kill_process(process.pid().as_u32());

        thread::sleep(Duration::from_secs(1));
    }

    system.refresh_processes(ProcessesToUpdate::All);

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
