use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::data_sources;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn start_steamvr() {

    let session = data_sources::get_read_only_local_session();
    let steamvr_settings = &session.settings().extra.steamvr_launcher;
    let quick_launch = steamvr_settings.quick_launch_steamvr;
    let steamvr_path = steamvr_settings.quick_launch_steamvr;

    if quick_launch {
        Command::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR\\bin\\win64\\vrstartup.exe")
            .spawn()
            .ok();
    } else {
        Command::new("cmd")
            .args(["/C", "start", "steam://rungameid/250820"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok();
    }
}

pub fn launch_steam_app(app_id: u32) {
    Command::new("cmd")
            .args(["/C", "start", "steam://rungameid/", &app_id.to_string()])
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
