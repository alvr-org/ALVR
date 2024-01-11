use crate::data_sources;
use alvr_common::{debug, once_cell::sync::Lazy, parking_lot::Mutex};
use alvr_filesystem as afs;
use alvr_session::{DriverLaunchAction, DriversBackup};
use std::{
    env,
    marker::PhantomData,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn is_steamvr_running() -> bool {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    system
        .processes_by_name(&afs::exec_fname("vrserver"))
        .count()
        != 0
}

#[cfg(target_os = "linux")]
pub fn maybe_wrap_vrcompositor_launcher() -> alvr_common::anyhow::Result<()> {
    use std::fs;

    let steamvr_bin_dir = alvr_server_io::steamvr_root_dir()?
        .join("bin")
        .join("linux64");
    let launcher_path = steamvr_bin_dir.join("vrcompositor");

    // In case of SteamVR update, vrcompositor will be restored
    if fs::read_link(&launcher_path).is_ok() {
        fs::remove_file(&launcher_path)?; // recreate the link
    } else {
        fs::rename(&launcher_path, steamvr_bin_dir.join("vrcompositor.real"))?;
    }

    std::os::unix::fs::symlink(
        afs::filesystem_layout_from_dashboard_exe(&env::current_exe().unwrap())
            .vrcompositor_wrapper(),
        &launcher_path,
    )?;

    Ok(())
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    use std::os::windows::process::CommandExt;
    Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

pub fn maybe_kill_steamvr() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    // first kill vrmonitor, then kill vrserver if it is hung.

    for process in system.processes_by_name(&afs::exec_fname("vrmonitor")) {
        debug!("Killing vrmonitor");

        #[cfg(not(windows))]
        process.kill_with(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());

        thread::sleep(Duration::from_secs(1));
    }

    system.refresh_processes();

    for process in system.processes_by_name(&afs::exec_fname("vrserver")) {
        debug!("Killing vrserver");

        #[cfg(not(windows))]
        process.kill_with(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());

        thread::sleep(Duration::from_secs(1));
    }
}

pub struct Launcher {
    _phantom: PhantomData<()>,
}

impl Launcher {
    pub fn launch_steamvr(&self) {
        let mut data_source = data_sources::get_local_data_source();

        let launch_action = &data_source.settings().steamvr_launcher.driver_launch_action;

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
        alvr_common::show_err(maybe_wrap_vrcompositor_launcher());

        if !is_steamvr_running() {
            debug!("SteamVR is dead. Launching...");

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                Command::new("cmd")
                    .args(["/C", "start", "steam://rungameid/250820"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
                    .ok();
            }
            #[cfg(not(windows))]
            {
                Command::new("xdg-open")
                    .args(["steam://rungameid/250820"])
                    .spawn()
                    .ok();
            }
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
