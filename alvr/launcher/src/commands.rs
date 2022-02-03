use alvr_common::prelude::*;
use alvr_filesystem as afs;
use serde_json as json;
use std::{
    env, fs,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, Signal, System, SystemExt};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn spawn_no_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW).spawn().ok();
}

#[cfg(not(windows))]
fn spawn_no_window(command: &mut Command) {
    command.spawn().ok();
}

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

pub fn maybe_launch_steamvr() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    if system
        .processes_by_name(&afs::exec_fname("vrserver"))
        .count()
        == 0
    {
        #[cfg(windows)]
        spawn_no_window(Command::new("cmd").args(&["/C", "start", "steam://rungameid/250820"]));
        #[cfg(not(windows))]
        spawn_no_window(Command::new("steam").args(&["steam://rungameid/250820"]));
    }
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    use std::os::windows::process::CommandExt;
    Command::new("taskkill.exe")
        .args(&["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

// this will not kill the child process "ALVR launcher"
pub fn kill_steamvr() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes();

    // first kill vrmonitor, then kill vrserver if it is hung.

    for process in system.processes_by_name(&afs::exec_fname("vrmonitor")) {
        #[cfg(not(windows))]
        process.kill_with(Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());
    }

    thread::sleep(Duration::from_secs(1));

    for process in system.processes_by_name(&afs::exec_fname("vrserver")) {
        #[cfg(not(windows))]
        process.kill_with(Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid().as_u32());
    }
}

pub fn check_steamvr_installation() -> bool {
    alvr_commands::openvr_source_file_path().is_ok()
}

pub fn unblock_alvr_addon() -> StrResult {
    let config_path = alvr_commands::steam_config_dir()?.join("steamvr.vrsettings");

    let mut fields_ref: json::Map<String, json::Value> = trace_err!(json::from_str(&trace_err!(
        fs::read_to_string(&config_path)
    )?))?;

    fields_ref.remove("driver_alvr_server");

    trace_err!(fs::write(
        config_path,
        trace_err!(json::to_string_pretty(&fields_ref))?
    ))?;

    Ok(())
}

pub fn maybe_register_alvr_driver() -> StrResult {
    let alvr_driver_dir = afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap())
        .openvr_driver_root_dir;

    let driver_registered = alvr_commands::get_driver_dir_from_registered()
        .ok()
        .filter(|dir| *dir == alvr_driver_dir)
        .is_some();

    if !driver_registered {
        let paths_backup = match alvr_commands::get_registered_drivers() {
            Ok(paths) => paths,
            Err(e) => {
                return fmt_e!(
                "{}\n{}\n\n({})",
                "Failed to load registered drivers.",
                "Please reset the drivers installation with the apposite button on the launcher.",
                e
            )
            }
        };

        alvr_commands::maybe_save_driver_paths_backup(&paths_backup)?;

        alvr_commands::driver_registration(&paths_backup, false)?;

        alvr_commands::driver_registration(&[alvr_driver_dir], true)?;
    }

    #[cfg(target_os = "linux")]
    maybe_wrap_vrcompositor_launcher()?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn maybe_wrap_vrcompositor_launcher() -> StrResult {
    let steamvr_bin_dir = alvr_commands::steamvr_root_dir()?
        .join("bin")
        .join("linux64");
    let real_launcher_path = steamvr_bin_dir.join("vrcompositor.real");
    let launcher_path = steamvr_bin_dir.join("vrcompositor");

    // In case of SteamVR update, vrcompositor will be restored
    match fs::read_link(&launcher_path) {
        Err(_) => match fs::metadata(&launcher_path) {
            Err(_) => (), //file does not exist, do nothing
            Ok(_) => {
                trace_err!(fs::rename(&launcher_path, &real_launcher_path))?;
            }
        },
        Ok(_) => trace_err!(fs::remove_file(&launcher_path))?, // recreate the link
    };

    trace_err!(std::os::unix::fs::symlink(
        afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap())
            .vrcompositor_wrapper(),
        &launcher_path
    ))?;

    Ok(())
}

pub fn fix_steamvr() {
    // If ALVR driver does not start use a more destructive approach: delete openvrpaths.vrpath then recreate it
    if let Ok(path) = alvr_commands::openvr_source_file_path() {
        fs::remove_file(path).ok();

        maybe_launch_steamvr();
        thread::sleep(Duration::from_secs(5));
        kill_steamvr();
        thread::sleep(Duration::from_secs(5));
    }

    unblock_alvr_addon().ok();
}

fn try_close_steamvr_gracefully() {
    let start_time = Instant::now();
    while start_time.elapsed() < SHUTDOWN_TIMEOUT && is_steamvr_running() {
        thread::sleep(Duration::from_millis(500));
    }

    // Note: if SteamVR already shutdown cleanly, this does nothing
    kill_steamvr();

    thread::sleep(Duration::from_secs(2));
}

pub fn restart_steamvr() {
    try_close_steamvr_gracefully();

    if alvr_common::show_err(maybe_register_alvr_driver()).is_some() {
        maybe_launch_steamvr();
    }
}

pub fn invoke_installer() {
    try_close_steamvr_gracefully();

    spawn_no_window(Command::new(afs::installer_path()).arg("-q"));

    // delete crash_log.txt (take advantage of the occasion to do some routine cleaning)
    fs::remove_file(
        afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap()).crash_log(),
    )
    .ok();
}
