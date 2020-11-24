use crate::*;
use std::{collections::HashSet, env, fs, path::{Path, PathBuf}, process::*};
use sysinfo::*;
use serde_json as json;

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

fn openvr_source_file_path() -> StrResult<PathBuf> {
    Ok(trace_none!(dirs::cache_dir())?.join("openvr/openvrpaths.vrpath"))
}

fn load_openvr_paths_json() -> StrResult<json::Value> {
    let file_content = trace_err!(
        fs::read_to_string(openvr_source_file_path()?),
        "SteamVR probably not installed"
    )?;

    trace_err!(json::from_str(&file_content))
}

fn save_openvr_paths_json(openvr_paths: &json::Value) -> StrResult {
    let file_content = trace_err!(json::to_string_pretty(openvr_paths))?;

    trace_err!(fs::write(openvr_source_file_path()?, file_content))
}

fn from_openvr_paths(paths: &json::Value) -> StrResult<Vec<PathBuf>> {
    let paths_vec = match paths.as_array() {
        Some(vec) => vec,
        None => return Ok(vec![]),
    };

    Ok(paths_vec
        .iter()
        .filter_map(json::Value::as_str)
        .map(|s| PathBuf::from(s.replace(r"\\", r"\")))
        .collect())
}

fn to_openvr_paths(paths: &[PathBuf]) -> json::Value {
    let paths_vec = paths
        .iter()
        .map(|p| p.to_string_lossy().into())
        .map(json::Value::String) // backslashes gets duplicated here
        .collect::<Vec<_>>();

    json::Value::Array(paths_vec)
}

fn steamvr_root_dir() -> StrResult<PathBuf> {
    let openvr_paths_json = load_openvr_paths_json()?;
    let paths_json = trace_none!(openvr_paths_json.get("runtime"))?;
    trace_none!(from_openvr_paths(paths_json)?.get(0).cloned())
}

pub fn get_registered_drivers() -> StrResult<Vec<PathBuf>> {
    from_openvr_paths(trace_none!(
        load_openvr_paths_json()?.get_mut("external_drivers")
    )?)
}

pub fn driver_registration(driver_paths: &[PathBuf], register: bool) -> StrResult {
    let mut openvr_paths_json = load_openvr_paths_json()?;
    let paths_json_ref = trace_none!(openvr_paths_json.get_mut("external_drivers"))?;

    let mut paths: HashSet<_> = from_openvr_paths(paths_json_ref)?.into_iter().collect();

    if register {
        paths.extend(driver_paths.iter().cloned());
    } else {
        for path in driver_paths {
            paths.remove(path);
        }
    }

    // write into openvr_paths_json, the other fields are preserved
    *paths_json_ref = to_openvr_paths(paths.into_iter().collect::<Vec<_>>().as_slice());

    save_openvr_paths_json(&openvr_paths_json)
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

    let bootstrap_or_driver_count = system
        .get_process_by_name(&exec_fname("ALVR Launcher"))
        .len()
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
                &steamvr_root_dir()
                    .map_err(|_| -1)?
                    .join("bin")
                    .join("win64")
                    .join("vrserver.exe")
            ),
            netsh_add_rule_command_string(
                "SteamVR ALVR vrserver",
                &steamvr_root_dir()
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

pub fn get_alvr_dir() -> StrResult<PathBuf> {
    for dir in get_registered_drivers()? {
        if dir.join(exec_fname("ALVR Launcher")).exists() && dir.join("web_gui").exists() {
            return Ok(dir);
        }
    }

    Err("ALVR driver is not registered".into())
}

pub fn unregister_all_drivers() -> StrResult {
    for dir in get_registered_drivers()? {
        driver_registration(&[dir], false)?;
    }

    Ok(())
}

pub fn maybe_open_launcher(alvr_dir: &Path) {
    let mut command = Command::new(alvr_dir.join("ALVR Launcher"));
    command.creation_flags(CREATE_NO_WINDOW).spawn().ok();
}

pub fn check_steamvr_installation() -> bool {
    openvr_source_file_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn check_msvcp_installation() -> StrResult<bool> {
    let output = trace_err!(Command::new("where")
        .arg("msvcp140_2.dll")
        .creation_flags(CREATE_NO_WINDOW)
        .output())?;
    let output = String::from_utf8_lossy(&output.stdout);

    Ok(output.contains("msvcp140_2.dll"))
}
