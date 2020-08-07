#![windows_subsystem = "windows"]

mod logging_backend;

use alvr_common::{logging::show_err, process::*, *};
use std::env;

fn window_mode() -> StrResult {
    let mutex = single_instance::SingleInstance::new("alvr_server_bootstrap_mutex").unwrap();
    if mutex.is_single() {
        if steamvr_bin_dir().is_err() {}

        let maybe_alvr_dir = get_alvr_dir();

        if get_alvr_dir().is_err() {
            match get_registered_drivers() {
                Ok(paths) => {
                    if !paths.is_empty() {
                        trace_err!(backup_driver_paths(&paths))?;
                    }
                }
                Err(_) => return Err("Please install SteamVR, run it once, then close it.".into()),
            }

            let current_path = trace_err!(env::current_exe())?;
            let alvr_dir = trace_none!(current_path.parent())?;
            driver_registration(alvr_dir, true)?;
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
