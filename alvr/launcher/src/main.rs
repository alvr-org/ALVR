#![windows_subsystem = "windows"]

use alvr_common::{commands::*, *};
use logging::{show_e, show_err};
use serde_json as json;
use std::{
    env,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

fn current_alvr_dir() -> StrResult<PathBuf> {
    let current_path = trace_err!(env::current_exe())?;
    Ok(trace_none!(current_path.parent())?.to_owned())
}

// Return a backup of the registered drivers if ALVR driver wasn't registered, otherwise return none
fn maybe_register_alvr_driver() -> StrResult {
    let current_alvr_dir = current_alvr_dir()?;

    store_alvr_dir(&current_alvr_dir)?;

    let driver_registered = get_alvr_dir_from_registered_drivers()
        .ok()
        .filter(|dir| *dir == current_alvr_dir.clone())
        .is_some();

    if !driver_registered {
        let paths_backup = match get_registered_drivers() {
            Ok(paths) => paths,
            Err(_) => return trace_str!("Please install SteamVR, run it once, then close it."),
        };

        maybe_save_driver_paths_backup(&paths_backup)?;

        driver_registration(&paths_backup, false)?;

        driver_registration(&[current_alvr_dir], true)?;
    }

    Ok(())
}

fn restart_steamvr() {
    let start_time = Instant::now();
    while start_time.elapsed() < SHUTDOWN_TIMEOUT && is_steamvr_running() {
        thread::sleep(Duration::from_millis(500));
    }

    // Note: if SteamVR already shutdown cleanly, this does nothing
    kill_steamvr();

    thread::sleep(Duration::from_secs(2));

    if show_err(maybe_register_alvr_driver()).is_ok() {
        maybe_launch_steamvr();
    }
}

fn window_mode() -> StrResult {
    let mutex = trace_err!(single_instance::SingleInstance::new("alvr_launcher_mutex"))?;
    if mutex.is_single() {
        maybe_delete_alvr_dir_storage();

        let html_content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/html/index.html"));
        let window = trace_err!(alcro::UIBuilder::new()
            .content(alcro::Content::Html(html_content))
            .size(200, 200)
            .custom_args(&["--disk-cache-size=1"])
            .run())?;

        trace_err!(window.bind("checkSteamvrInstallation", |_| {
            Ok(json::Value::Bool(check_steamvr_installation()))
        }))?;

        trace_err!(window.bind("checkMsvcpInstallation", |_| {
            Ok(json::Value::Bool(
                check_msvcp_installation().unwrap_or_else(|e| {
                    show_e(e);
                    false
                }),
            ))
        }))?;

        trace_err!(window.bind("startDriver", move |_| {
            if !is_steamvr_running() && show_err(maybe_register_alvr_driver()).is_ok() {
                maybe_launch_steamvr();
            }
            Ok(json::Value::Null)
        }))?;

        trace_err!(window.bind("restartSteamvr", |_| {
            restart_steamvr();
            Ok(json::Value::Null)
        }))?;

        trace_err!(window.eval("init()"))?;

        window.wait_finish();

        // This is needed in case the launcher window is closed before the driver is loaded,
        // otherwise this does nothing
        apply_driver_paths_backup(current_alvr_dir()?)?;
    }
    Ok(())
}

fn main() {
    let args = env::args().collect::<Vec<_>>();

    match args.get(1) {
        Some(flag) if flag == "--restart-steamvr" => restart_steamvr(),
        _ => {
            show_err(window_mode()).ok();
        }
    }
}
