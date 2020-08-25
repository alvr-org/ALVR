#![windows_subsystem = "windows"]

use alvr_common::{commands::*, *};
use logging::show_err;
use serde_json as json;
use std::env;

fn maybe_register_alvr_driver() -> StrResult {
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
            Err(_) => return trace_str!("Please install SteamVR, run it once, then close it."),
        }
        driver_registration(current_alvr_dir, true)?;
        store_alvr_dir(current_alvr_dir)?
    }
    Ok(())
}

fn window_mode() -> StrResult {
    let mutex = trace_err!(single_instance::SingleInstance::new("alvr_launcher_mutex"))?;
    if mutex.is_single() {
        maybe_delete_alvr_dir_store();

        let html_content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/html/index.html"));
        let window = alcro::UIBuilder::new()
            .content(alcro::Content::Html(html_content))
            .size(200, 200)
            .custom_args(&["--disk-cache-size=1"])
            .run();

        trace_err!(window.bind("checkSteamvrInstallation", |_| {
            Ok(json::to_value(steamvr_bin_dir().is_ok()).unwrap())
        }))?;

        trace_err!(window.bind("checkMsvcpInstallation", |_| {
            Ok(json::to_value(check_msvcp_installation().unwrap()).unwrap())
        }))?;

        trace_err!(window.bind("maybeLaunchSteamvr", |_| {
            show_err(maybe_register_alvr_driver()).ok();
            maybe_launch_steamvr();
            Ok(json::Value::Null)
        }))?;

        trace_err!(window.bind("killSteamvr", |_| {
            kill_steamvr();
            Ok(json::Value::Null)
        }))?;

        trace_err!(window.eval("init()"))?;

        window.wait_finish();
    }
    Ok(())
}

fn main() {
    show_err(window_mode()).ok();
}
