#![windows_subsystem = "windows"]

use alvr_common::{commands::*, *};
use logging::{show_e, show_err};
use parking_lot::Mutex;
use serde_json as json;
use std::{
    env,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

const RETRY_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

fn current_alvr_dir() -> StrResult<PathBuf> {
    let current_path = trace_err!(env::current_exe())?;
    Ok(trace_none!(current_path.parent())?.to_owned())
}

// Return a backup of the registered drivers if ALVR driver wasn't registered, otherwise return none
fn maybe_register_alvr_driver() -> StrResult<Option<Vec<PathBuf>>> {
    let current_alvr_dir = current_alvr_dir()?;

    let driver_registered = get_alvr_dir_from_registered_drivers()
        .ok()
        .filter(|dir| *dir == current_alvr_dir.clone())
        .is_some();
    if !driver_registered {
        let paths_backup = match get_registered_drivers() {
            Ok(paths) => {
                driver_registration(&paths, false)?;
                paths
            }
            Err(_) => return trace_str!("Please install SteamVR, run it once, then close it."),
        };

        store_alvr_dir(&current_alvr_dir)?;
        driver_registration(&[current_alvr_dir], true)?;

        Ok(Some(paths_backup))
    } else {
        Ok(None)
    }
}

// this does nothing if called a second time
fn apply_drivers_backup(drivers_backup: Arc<Mutex<Option<Vec<PathBuf>>>>) -> StrResult {
    if let Some(paths) = drivers_backup.lock().take() {
        driver_registration(&[current_alvr_dir()?], false)?;

        driver_registration(&paths, true).ok();
    }
    // else: ALVR driver was registered manually, nothing to do

    Ok(())
}

fn start_driver(drivers_backup: Arc<Mutex<Option<Vec<PathBuf>>>>) {
    if let Ok(maybe_driver_paths) = show_err(maybe_register_alvr_driver()) {
        *drivers_backup.lock() = maybe_driver_paths;

        maybe_launch_steamvr();

        let mut time_start = Instant::now();
        while !ureq::get("http://127.0.0.1:8082")
            .timeout(RETRY_TIMEOUT)
            .call()
            .ok()
        {
            if Instant::now() > time_start + RETRY_TIMEOUT {
                maybe_launch_steamvr();
                time_start = Instant::now();
            }
        }

        apply_drivers_backup(drivers_backup).ok();
    }
}

fn restart_steamvr() {

    let start_time = Instant::now();
    while start_time.elapsed() < SHUTDOWN_TIMEOUT && is_steamvr_running() {
        thread::sleep(Duration::from_millis(500));
    }

    // Note: if SteamVR already shutdown cleanly, this does nothing
    kill_steamvr();

    thread::sleep(Duration::from_secs(1));

    start_driver(Arc::default()); // argument is not needed
}

fn window_mode(drivers_backup: Arc<Mutex<Option<Vec<PathBuf>>>>) -> StrResult {
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
            start_driver(drivers_backup.clone());
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
    let args = env::args().collect::<Vec<_>>();

    match args.get(1) {
        Some(flag) if flag == "--restart-steamvr" => restart_steamvr(),
        _ => {
            let drivers_backup = Arc::new(Mutex::new(None));
            show_err(window_mode(drivers_backup.clone())).ok();

            // fallback if the window has been closed before loading the dashboard
            apply_drivers_backup(drivers_backup).ok();
        }
    }
}
