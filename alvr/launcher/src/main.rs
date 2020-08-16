#![windows_subsystem = "windows"]

mod logging_backend;

use alvr_common::{
    data::{load_session, SESSION_FNAME},
    logging::show_err,
    process::*,
    *,
};
use std::env;

fn window_mode() -> StrResult {
    let mutex = single_instance::SingleInstance::new("alvr_launcher_mutex").unwrap();
    if mutex.is_single() {
        maybe_delete_alvr_dir_store();

        let current_path = trace_err!(env::current_exe())?;
        let current_alvr_dir = trace_none!(current_path.parent())?;

        let driver_registered = get_alvr_dir()
            .ok()
            .filter(|dir| dir == current_alvr_dir)
            .is_some();
        if !driver_registered {
            match get_registered_drivers() {
                Ok(paths) => {
                    if !paths.is_empty() {
                        backup_driver_paths(&paths)?;
                    }
                }
                Err(_) => {
                    warn!("Please install SteamVR, run it once, then close it.");
                    return Ok(());
                }
            }
            driver_registration(current_alvr_dir, true)?;
        }

        let first_time_launch = load_session(&current_alvr_dir.join(SESSION_FNAME)).is_err();
        if first_time_launch {
            warn!(
                "{} {}",
                "If you didn't already, please install the latest Visual C++ Redistributable",
                "package. Go to github.com/JackD83/ALVR for the instructions."
            );
        }

        maybe_launch_steamvr();

        let window = alcro::UIBuilder::new()
            .content(alcro::Content::Url("http://127.0.0.1:8082"))
            .size(800, 600)
            .run();
        window.wait_finish();
    }
    Ok(())
}

fn main() {
    logging_backend::init_logging();
    show_err(window_mode()).ok();
}
