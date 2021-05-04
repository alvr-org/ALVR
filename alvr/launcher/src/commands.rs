use alvr_common::{
    commands,
    logging::{self, CRASH_LOG_FNAME},
    prelude::*,
};
use serde_json as json;
use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessExt, RefreshKind, System, SystemExt};

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
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    !system
        .get_process_by_name(&commands::exec_fname("vrserver"))
        .is_empty()
}

pub fn maybe_launch_steamvr() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    if system
        .get_process_by_name(&commands::exec_fname("vrserver"))
        .is_empty()
    {
        #[cfg(windows)]
        spawn_no_window(Command::new("cmd").args(&["/C", "start", "steam://rungameid/250820"]));
        #[cfg(not(windows))]
        spawn_no_window(Command::new("steam").args(&["steam://rungameid/250820"]));
    }
}

#[cfg(windows)]
fn kill_process(pid: usize) {
    use std::os::windows::process::CommandExt;
    Command::new("taskkill.exe")
        .args(&["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

// this will not kill the child process "ALVR launcher"
pub fn kill_steamvr() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    // first kill vrmonitor, then kill vrserver if it is hung.

    for process in system.get_process_by_name(&commands::exec_fname("vrmonitor")) {
        #[cfg(not(windows))]
        process.kill(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid());
    }

    thread::sleep(Duration::from_secs(1));

    for process in system.get_process_by_name(&commands::exec_fname("vrserver")) {
        #[cfg(not(windows))]
        process.kill(sysinfo::Signal::Term);
        #[cfg(windows)]
        kill_process(process.pid());
    }
}

pub fn check_steamvr_installation() -> bool {
    commands::openvr_source_file_path().is_ok()
}

pub fn unblock_alvr_addon() -> StrResult {
    let config_path = commands::steam_config_dir()?.join("steamvr.vrsettings");

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

pub fn maybe_register_alvr_driver() -> StrResult {
    let current_alvr_dir = current_alvr_dir()?;

    commands::store_alvr_dir(&current_alvr_dir)?;

    let driver_registered = commands::get_alvr_dir_from_registered_drivers()
        .ok()
        .filter(|dir| *dir == current_alvr_dir.clone())
        .is_some();

    #[cfg(target_os = "linux")]
    maybe_wrap_vrcompositor_launcher()?;

    if !driver_registered {
        let paths_backup = match commands::get_registered_drivers() {
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

        commands::maybe_save_driver_paths_backup(&paths_backup)?;

        commands::driver_registration(&paths_backup, false)?;

        commands::driver_registration(&[current_alvr_dir], true)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn maybe_wrap_vrcompositor_launcher() -> StrResult {
    use std::{fs::File, io::prelude::*, os::unix::fs::PermissionsExt};

    let steamvr_bin_dir = commands::steamvr_root_dir()?.join("bin").join("linux64");
    let real_launcher_path = steamvr_bin_dir.join("vrcompositor-launcher.real");
    let launcher_path = steamvr_bin_dir.join("vrcompositor-launcher");

    if !real_launcher_path.exists() {
        trace_err!(fs::rename(&launcher_path, &real_launcher_path))?;
    }

    let wrapper_data = include_bytes!("../res/vrcompositor_launcher_wrapper.sh");

    let launcher_data = trace_err!(fs::read_to_string(launcher_path.clone()))?;
    if launcher_data.as_bytes() != wrapper_data {
        let mut launcher_file = trace_err!(File::create(launcher_path))?;
        trace_err!(launcher_file.write_all(wrapper_data))?;

        let mut perms = trace_err!(launcher_file.metadata())?.permissions();
        perms.set_mode(0o755); // rwxr-xr-x
        trace_err!(launcher_file.set_permissions(perms))?;
    }

    Ok(())
}

pub fn fix_steamvr() {
    // If ALVR driver does not start use a more destructive approach: delete openvrpaths.vrpath then recreate it
    if let Ok(path) = commands::openvr_source_file_path() {
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

    if logging::show_err(maybe_register_alvr_driver()).is_some() {
        maybe_launch_steamvr();
    }
}

pub fn invoke_installer() {
    try_close_steamvr_gracefully();

    spawn_no_window(Command::new(commands::installer_path()).arg("-q"));

    // delete crash_log.txt (take advantage of the occasion to do some routine cleaning)
    fs::remove_file(current_alvr_dir().unwrap().join(CRASH_LOG_FNAME)).ok();
}
