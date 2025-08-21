use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use crate::steamvr_launcher::get_steamvr_root_dir;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn launch_steam_app(app_id: &str) {
    Command::new("cmd")
            .args(["/C", "start", "steam://rungameid/", app_id])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok();
}

pub fn kill_process(pid: u32) {
    Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok();
}

pub fn get_default_steamvr_executable_path() -> String {
    return get_steamvr_root_dir()
        .join("bin")
        .join("win64")
        .join("vrstartup.exe")
        .into_os_string()
        .into_string()
        .unwrap();
}
