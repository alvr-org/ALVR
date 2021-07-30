use crate::prelude::*;
use encoding_rs_io::DecodeReaderBytes;
use serde_json as json;
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

const DRIVER_PATHS_BACKUP_FNAME: &str = "alvr_drivers_paths_backup.txt";
const INSTALLER_FNAME: &str = "alvr_installer";

#[cfg(not(windows))]
pub fn exec_fname(name: &str) -> String {
    name.to_owned()
}
#[cfg(windows)]
pub fn exec_fname(name: &str) -> String {
    format!("{}.exe", name)
}

pub fn installer_path() -> PathBuf {
    env::temp_dir().join(exec_fname(INSTALLER_FNAME))
}

///////////// openvrpaths.vrpath interop ///////////////

pub fn openvr_source_file_path() -> StrResult<PathBuf> {
    let path = trace_none!(if cfg!(windows) {
        dirs::cache_dir()
    } else {
        dirs::config_dir()
    })?
    .join("openvr/openvrpaths.vrpath");

    if path.exists() {
        Ok(path)
    } else {
        fmt_e!("{} does not exist", path.to_string_lossy())
    }
}

fn load_openvr_paths_json() -> StrResult<json::Value> {
    let file = trace_err!(File::open(openvr_source_file_path()?))?;

    let mut file_content_decoded = String::new();
    trace_err!(DecodeReaderBytes::new(&file).read_to_string(&mut file_content_decoded))?;

    trace_err!(json::from_str(&file_content_decoded))
}

fn save_openvr_paths_json(openvr_paths: &json::Value) -> StrResult {
    let file_content = trace_err!(json::to_string_pretty(openvr_paths))?;

    trace_err!(fs::write(openvr_source_file_path()?, file_content))
}

fn from_openvr_paths(paths: &json::Value) -> Vec<std::path::PathBuf> {
    let paths_vec = match paths.as_array() {
        Some(vec) => vec,
        None => return vec![],
    };

    paths_vec
        .iter()
        .filter_map(json::Value::as_str)
        .map(|s| PathBuf::from(s.replace(r"\\", r"\")))
        .collect()
}

fn to_openvr_paths(paths: &[PathBuf]) -> json::Value {
    let paths_vec = paths
        .iter()
        .map(|p| p.to_string_lossy().into())
        .map(json::Value::String) // backslashes gets duplicated here
        .collect::<Vec<_>>();

    json::Value::Array(paths_vec)
}

fn get_single_openvr_path(path_type: &str) -> StrResult<PathBuf> {
    let openvr_paths_json = load_openvr_paths_json()?;
    let paths_json = trace_none!(openvr_paths_json.get(path_type))?;
    trace_none!(from_openvr_paths(paths_json).get(0).cloned())
}

pub fn steamvr_root_dir() -> StrResult<PathBuf> {
    get_single_openvr_path("runtime")
}

pub fn steam_config_dir() -> StrResult<PathBuf> {
    get_single_openvr_path("config")
}

///////////////// driver paths management ///////////////////

pub fn get_registered_drivers() -> StrResult<Vec<PathBuf>> {
    Ok(from_openvr_paths(trace_none!(
        load_openvr_paths_json()?.get_mut("external_drivers")
    )?))
}

pub fn driver_registration(driver_paths: &[PathBuf], register: bool) -> StrResult {
    let mut openvr_paths_json = load_openvr_paths_json()?;
    let paths_json_ref = trace_none!(openvr_paths_json.get_mut("external_drivers"))?;

    let mut paths: HashSet<_> = from_openvr_paths(paths_json_ref).into_iter().collect();

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

pub fn get_alvr_dir_from_registered_drivers() -> StrResult<PathBuf> {
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct VrDriverManifest {
        name: String,
    }
    for dir in get_registered_drivers()? {
        let manifest = dir.join("driver.vrdrivermanifest");
        if manifest.exists() {
            let file = std::fs::File::open(manifest);
            if file.is_err() {
                continue;
            };

            let reader = std::io::BufReader::new(file.unwrap());
            let parsed_manifest: Result<VrDriverManifest, serde_json::Error> =
                serde_json::from_reader(reader);
            if parsed_manifest.is_err() {
                continue;
            };
            if parsed_manifest.unwrap().name == "alvr_server" {
                return alvr_filesystem_layout::alvr_dir_from_component(
                    &dir,
                    &alvr_filesystem_layout::LAYOUT.openvr_driver_dir,
                );
            }
        }
    }
    fmt_e!("ALVR driver path not registered")
}

pub fn get_alvr_dir() -> StrResult<PathBuf> {
    get_alvr_dir_from_registered_drivers()
        .map_err(|e| format!("ALVR driver path not stored and not registered ({})", e))
}

fn driver_paths_backup_present() -> bool {
    env::temp_dir().join(DRIVER_PATHS_BACKUP_FNAME).exists()
}

pub fn apply_driver_paths_backup(alvr_dir: PathBuf) -> StrResult {
    if driver_paths_backup_present() {
        let backup_path = env::temp_dir().join(DRIVER_PATHS_BACKUP_FNAME);
        let driver_paths = trace_err!(json::from_str::<Vec<_>>(&trace_err!(fs::read_to_string(
            &backup_path
        ))?))?;
        trace_err!(fs::remove_file(backup_path))?;

        driver_registration(&[alvr_dir], false)?;

        driver_registration(&driver_paths, true).ok();
    }

    Ok(())
}

pub fn maybe_save_driver_paths_backup(paths_backup: &[PathBuf]) -> StrResult {
    if !driver_paths_backup_present() {
        trace_err!(fs::write(
            env::temp_dir().join(DRIVER_PATHS_BACKUP_FNAME),
            trace_err!(json::to_string_pretty(paths_backup))?,
        ))?;
    }

    Ok(())
}

pub fn get_session_path(base: &Path) -> StrResult<PathBuf> {
    if cfg!(target_os = "linux") {
        Ok(trace_none!(dirs::config_dir())?
            .join("alvr")
            .join("session.json"))
    } else {
        Ok(base.join("session.json"))
    }
}

#[cfg(target_os = "linux")]
pub fn maybe_create_alvr_config_directory() -> StrResult {
    let alvr_dir = trace_none!(dirs::config_dir())?.join("alvr");
    if !alvr_dir.exists() {
        trace_err!(fs::create_dir(alvr_dir))?;
    }
    Ok(())
}

/////////////////// firewall //////////////////////

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
// 126: pkexec request dismissed
// other: command failed
pub fn firewall_rules(add: bool) -> Result<(), i32> {
    let exit_status;

    if cfg!(target_os = "linux") {
        let action = if add { "add" } else { "remove" };
        // run as normal user since we use pkexec to sudo
        exit_status = Command::new("bash")
            .arg("/usr/libexec/alvr/alvr_fw_config.sh")
            .arg(action)
            .status()
            .map_err(|_| -1)?;
    } else {
        let script_path = env::temp_dir().join("alvr_firewall_rules.bat");
        let firewall_rules_script_content = if add {
            format!(
                "{}\n{}",
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

        // run with admin privileges
        exit_status = runas::Command::new(script_path)
            .gui(true) // UAC, if available
            .status()
            .map_err(|_| -1)?;
    }

    if exit_status.success() {
        Ok(())
    } else {
        Err(exit_status.code().unwrap())
    }
}

/////////////////// launcher invocation ///////////////////////

fn invoke_launcher(alvr_dir: &Path, flag: &str) -> StrResult {
    trace_err!(
        Command::new(alvr_dir.join(&alvr_filesystem_layout::LAYOUT.launcher_exe))
            .arg(flag)
            .status()
    )?;

    Ok(())
}

pub fn restart_steamvr(alvr_dir: &Path) -> StrResult {
    invoke_launcher(alvr_dir, "--restart-steamvr")
}

pub fn invoke_application_update(alvr_dir: &Path) -> StrResult {
    invoke_launcher(alvr_dir, "--update")
}
