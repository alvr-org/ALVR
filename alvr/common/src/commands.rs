use crate::*;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::*,
};
use sysinfo::*;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

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

// Launch web server. If another instance exists, the one just spawned will close itself.
pub fn maybe_launch_web_server(root_server_dir: &Path) {
    let mut command = Command::new(root_server_dir.join("alvr_web_server"));

    // somehow the console is always empty, so it's useless
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command.spawn().ok();
}

// Kill web server and its child processes if only one of bootstrap or driver is alive.
pub fn maybe_kill_web_server() {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();

    let bootstrap_or_driver_count = system.get_process_by_name(&exec_fname("ALVR launcher")).len()
        + system.get_process_by_name(&exec_fname("vrserver")).len();

    if bootstrap_or_driver_count <= 1 {
        for process in system.get_processes().values() {
            if let Some(parent_pid) = process.parent() {
                if let Some(parent_proc) = system.get_process(parent_pid) {
                    if parent_proc.name() == exec_fname("alvr_web_server") {
                        // Using built-in method causes cmd to pop up repeatedly on Windows
                        #[cfg(not(windows))]
                        process.kill(Signal::Term);
                        #[cfg(windows)]
                        kill_process(process.pid());
                    }
                }
            }
        }
        for process in system.get_process_by_name(&exec_fname("alvr_web_server")) {
            #[cfg(not(windows))]
            process.kill(Signal::Term);
            #[cfg(windows)]
            kill_process(process.pid());
        }
    }
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
        Err(format!("Error registering driver: {}", exit_status))
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
pub fn firewall_rules(root_server_dir: &Path, add: bool) -> Result<(), i32> {
    let script_path = env::temp_dir().join("alvr_firewall_rules.bat");
    let web_server_path = root_server_dir.join(exec_fname("alvr_web_server"));

    let firewall_rules_script_content = if add {
        format!(
            "{}\n{}\n{}",
            netsh_add_rule_command_string("ALVR Launcher", &web_server_path),
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
    for dir in get_registered_drivers()? {
        if dir.join(exec_fname("ALVR launcher")).exists() && dir.join("web_gui").exists() {
            return Ok(dir);
        }
    }

    Err("ALVR driver is not registered".into())
}

pub fn unregister_all_drivers() -> StrResult {
    for dir in get_registered_drivers()? {
        driver_registration(&dir, false)?;
    }

    Ok(())
}
