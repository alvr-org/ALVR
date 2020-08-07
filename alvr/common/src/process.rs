use crate::*;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::*,
};
use sysinfo::*;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const DRIVER_BACKUP_FNAME: &str = "alvr_steamvr_drivers_backup.txt";
pub const ALVR_DIR_STORE_FNAME: &str = "alvr_dir.txt";

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(target_os = "linux")]
pub fn exec_fname(name: &str) -> String {
    name.to_owned()
}
#[cfg(windows)]
pub fn exec_fname(name: &str) -> String {
    format!("{}.exe", name)
}

#[cfg(target_os = "linux")]
fn steamvr_bin_dir() -> StrResult<PathBuf> {
    Ok(dirs::home_dir()
        .unwrap()
        .join(".steam/steam/steamapps/common/SteamVR/bin/linux64"))
}
#[cfg(windows)]
pub fn steamvr_bin_dir() -> StrResult<PathBuf> {
    use winreg::*;
    let key = trace_err!(
        RegKey::predef(enums::HKEY_CLASSES_ROOT).open_subkey("vrmonitor\\Shell\\Open\\Command")
    )?;
    let command_string: String = trace_err!(key.get_value(""))?;

    let path_string = trace_err!(regex::Regex::new(r#""(.+)\\vrmonitor.exe""#))?
        .captures(&command_string)
        .ok_or_else(|| "regex failed")?
        .get(1)
        .ok_or_else(|| "regex failed")?
        .as_str();

    Ok(PathBuf::from(path_string))
}

fn steamvr_dir() -> StrResult<PathBuf> {
    Ok(trace_none!(trace_none!(steamvr_bin_dir()?.parent())?.parent())?.into())
}

#[cfg(windows)]
fn kill_process(pid: usize) {
    Command::new("taskkill.exe")
        .args(&["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

pub fn maybe_launch_steamvr() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    if system
        .get_process_by_name(&exec_fname("vrserver"))
        .is_empty()
    {
        Command::new("cmd")
            .args(&["/C", "start", "steam://run/250820"])
            .spawn()
            .ok();
    }
}

// this does not kill any child processes, including possibly the web server
pub fn kill_steamvr() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    for process_name in ["vrserver", "vrcompositor", "vrdashboard", "vrmonitor"].iter() {
        for process in system.get_process_by_name(&exec_fname(process_name)) {
            #[cfg(not(windows))]
            process.kill(Signal::Term);
            #[cfg(windows)]
            kill_process(process.pid());
        }
    }
}

pub fn driver_registration(root_server_dir: &Path, register: bool) -> StrResult {
    let steamvr_bin_dir = steamvr_bin_dir()?;
    if cfg!(target_os = "linux") {
        env::set_var("LD_LIBRARY_PATH", &steamvr_bin_dir);
    }

    let exec = steamvr_bin_dir.join(exec_fname("vrpathreg"));
    let subcommand = if register {
        "adddriver"
    } else {
        "removedriver"
    };

    let exit_status = trace_err!(Command::new(exec)
        .args(&[subcommand, &root_server_dir.to_string_lossy()])
        .status())?;

    if exit_status.success() {
        Ok(())
    } else {
        Err(format!("Error registering driver: {}", exit_status).into())
    }
}

fn netsh_add_rule_command_string(rule_name: &str, program_path: &Path) -> String {
    format!(
        "netsh advfirewall firewall add rule name=\"{}\" dir=in program=\"{}\" action=allow",
        rule_name,
        program_path.to_string_lossy()
    )
}

fn netsh_delete_rule_command_string(rule_name: &str) -> String {
    format!(
        "netsh advfirewall firewall delete rule name=\"{}\"",
        rule_name,
    )
}

// Errors:
// 1: firewall rule is already set
// other: command failed
pub fn firewall_rules(add: bool) -> Result<(), i32> {
    let script_path = env::temp_dir().join("alvr_firewall_rules.bat");

    let firewall_rules_script_content = if add {
        format!(
            "{}\n{}",
            netsh_add_rule_command_string(
                "SteamVR ALVR vrserver",
                &steamvr_dir()
                    .map_err(|_| -1)?
                    .join("bin")
                    .join("win64")
                    .join("vrserver.exe")
            ),
            netsh_add_rule_command_string(
                "SteamVR ALVR vrserver",
                &steamvr_dir()
                    .map_err(|_| -1)?
                    .join("bin")
                    .join("win32")
                    .join("vrserver.exe")
            ),
        )
    } else {
        format!(
            "{}\n{}",
            netsh_delete_rule_command_string("ALVR Launcher"),
            netsh_delete_rule_command_string("SteamVR ALVR vrserver")
        )
    };
    fs::write(&script_path, firewall_rules_script_content).map_err(|_| -1)?;

    // run with admin priviles
    let exit_status = runas::Command::new(script_path)
        .show(cfg!(target_os = "linux"))
        .gui(true) // UAC, if available
        .status()
        .map_err(|_| -1)?;

    if exit_status.success() {
        Ok(())
    } else {
        Err(exit_status.code().unwrap())
    }
}

pub fn get_registered_drivers() -> StrResult<Vec<PathBuf>> {
    let output =
        trace_err!(Command::new(steamvr_bin_dir()?.join(exec_fname("vrpathreg"))).output())?;
    let output = String::from_utf8_lossy(&output.stdout);

    let dirs = trace_err!(regex::Regex::new(r"\t([^\t\r\n]*)"))?
        .captures_iter(&output)
        .filter_map(|captures| captures.get(1))
        .map(|cap_match| PathBuf::from(cap_match.as_str()))
        .collect::<Vec<_>>();
    Ok(dirs)
}

pub fn get_alvr_dir() -> StrResult<PathBuf> {
    let alvr_dir_store_path = env::temp_dir().join(ALVR_DIR_STORE_FNAME);
    if let Ok(path) = fs::read_to_string(alvr_dir_store_path) {
        return Ok(PathBuf::from(path));
    }

    for dir in get_registered_drivers()? {
        if dir.join(exec_fname("ALVR")).exists() && dir.join("web_gui").exists() {
            return Ok(dir);
        }
    }

    Err("ALVR driver is not registered".into())
}

pub fn store_alvr_dir(alvr_dir: &Path) -> StrResult {
    let alvr_dir_store_path = env::temp_dir().join(ALVR_DIR_STORE_FNAME);

    trace_err!(fs::write(
        alvr_dir_store_path,
        alvr_dir.to_string_lossy().as_bytes()
    ))
}

pub fn maybe_delete_alvr_dir_store() {
    fs::remove_file(env::temp_dir().join(ALVR_DIR_STORE_FNAME)).ok();
}

pub fn unregister_all_drivers() -> StrResult {
    for dir in get_registered_drivers()? {
        driver_registration(&dir, false)?;
    }

    Ok(())
}

pub fn backup_driver_paths(paths: &[PathBuf]) -> StrResult {
    let backup_path = env::temp_dir().join(DRIVER_BACKUP_FNAME);
    let backup_content = paths
        .iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    trace_err!(fs::write(backup_path, backup_content))
}

pub fn restore_driver_paths_backup() -> StrResult {
    let backup_path = env::temp_dir().join(DRIVER_BACKUP_FNAME);

    let paths = trace_err!(fs::read_to_string(&backup_path))?
        .split("\n")
        .map(|s| PathBuf::from(s))
        .collect::<Vec<_>>();

    for path in paths {
        driver_registration(&path, true)?;
    }

    trace_err!(fs::remove_file(backup_path))
}
