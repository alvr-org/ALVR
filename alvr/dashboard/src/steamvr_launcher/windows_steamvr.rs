use std::os::windows::process::CommandExt;
use std::process::Command;

use std::default;

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
    let default_steamvr_executable = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR\\bin\\win64\\vrstartup.exe".to_string();

    let steamvr_root_dir = alvr_server_io::steamvr_root_dir();

    if !steamvr_root_dir.exists() {
        return default_steamvr_executable;
    }

    let steamvr_bin_dir = steamvr_root_dir.join("bin").join("win64"); // Set to win32 for 32-bit systems
    let steamvr_bin;

    if !steamvr_bin_dir.exists() {
        steamvr_bin = steamvr_bin_dir.join("vrstartup.exe").to_string_lossy().into_owned()
    }

    return steamvr_bin;
}
