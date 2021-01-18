use alvr_common::{commands::*, logging::*, *};
use serde_json as json;
use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessExt, RefreshKind, System, SystemExt};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(windows)]
fn kill_process(pid: usize) {
    Command::new("taskkill.exe")
        .args(&["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

pub fn is_steamvr_running() -> bool {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    !system
        .get_process_by_name(&exec_fname("vrserver"))
        .is_empty()
}

pub fn maybe_launch_steamvr() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    if system
        .get_process_by_name(&exec_fname("vrserver"))
        .is_empty()
    {
        Command::new("cmd")
            .args(&["/C", "start", "steam://rungameid/250820"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok();
    }
}

// this will not kill the child process "ALVR launcher"
pub fn kill_steamvr() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    // first kill vrmonitor, then kill vrserver if it is hung.

    for process in system.get_process_by_name(&exec_fname("vrmonitor")) {
        #[cfg(not(windows))]
        process.kill(Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid());
    }

    thread::sleep(Duration::from_secs(1));

    for process in system.get_process_by_name(&exec_fname("vrserver")) {
        #[cfg(not(windows))]
        process.kill(Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid());
    }
}

pub fn check_steamvr_installation() -> bool {
    openvr_source_file_path().is_ok()
}

#[cfg(windows)]
pub fn check_msvcp_installation() -> StrResult<bool> {
    let output = trace_err!(Command::new("where")
        .arg("msvcp140_2.dll")
        .creation_flags(CREATE_NO_WINDOW)
        .output())?;
    let output = String::from_utf8_lossy(&output.stdout);

    Ok(output.contains(r"C:\Windows\System32\msvcp140_2.dll"))
}

pub fn unblock_alvr_addon() -> StrResult {
    let config_path = steam_config_dir()?.join("steamvr.vrsettings");

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

pub fn current_alvr_dir() -> StrResult<PathBuf> {
    let current_path = trace_err!(env::current_exe())?;
    Ok(trace_none!(current_path.parent())?.to_owned())
}

// Return a backup of the registered drivers if ALVR driver wasn't registered, otherwise return none
pub fn maybe_register_alvr_driver() -> StrResult {
    let current_alvr_dir = current_alvr_dir()?;

    store_alvr_dir(&current_alvr_dir)?;

    let driver_registered = get_alvr_dir_from_registered_drivers()
        .ok()
        .filter(|dir| *dir == current_alvr_dir.clone())
        .is_some();

    if !driver_registered {
        let paths_backup = match get_registered_drivers() {
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

        maybe_save_driver_paths_backup(&paths_backup)?;

        driver_registration(&paths_backup, false)?;

        driver_registration(&[current_alvr_dir], true)?;
    }

    Ok(())
}

pub fn fix_steamvr() {
    // If ALVR driver does not start use a more destructive approach: delete openvrpaths.vrpath then recreate it
    if let Ok(path) = openvr_source_file_path() {
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

    if show_err(maybe_register_alvr_driver()).is_some() {
        maybe_launch_steamvr();
    }
}

pub fn invoke_installer() {
    try_close_steamvr_gracefully();

    Command::new(commands::installer_path())
        .arg("-q")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok();

    // delete crash_log.txt (take advantage of the occasion to do some routine cleaning)
    fs::remove_file(current_alvr_dir().unwrap().join(CRASH_LOG_FNAME)).ok();
}
