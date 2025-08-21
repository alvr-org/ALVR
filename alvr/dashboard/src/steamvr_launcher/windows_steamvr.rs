use std::os::windows::process::CommandExt;
use std::process::Command;

use alvr_common::error;

use std::path::PathBuf;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn launch_steamvr_with_steam() {
    Command::new("cmd")
        .args(["/C", "start", "steam://rungameid/250820"])
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

fn get_steamvr_root_dir() -> PathBuf {
    match alvr_server_io::steamvr_root_dir() {
        Ok(dir) => dir,
        Err(e) => {
            error!(
                "Couldn't find OpenVR or SteamVR files. \
                Please make sure you have installed and ran SteamVR at least once. {e}"
            );
            "".into()
        }
    }
}
