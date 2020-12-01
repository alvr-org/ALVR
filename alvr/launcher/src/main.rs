#![windows_subsystem = "windows"]

use alvr_common::{commands::*, data::*, logging::*, *};
use fs_extra::dir as dirx;
use self_update::{backends::github::ReleaseList, update::Release};
use semver::Version;
use serde_json as json;
use std::{
    env, fs,
    fs::File,
    io,
    path::Path,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    time::Instant,
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

fn get_latest_server_release(update_channel: UpdateChannel) -> StrResult<(Release, Version)> {
    let release_list = trace_err!(ReleaseList::configure()
        .repo_owner("alvr-org")
        .repo_name(if matches!(update_channel, UpdateChannel::Nightly) {
            "ALVR-nightly"
        } else {
            ALVR_NAME
        })
        .build())?;

    let wants_prereleases = !matches!(update_channel, UpdateChannel::Stable);

    for release in trace_err!(release_list.fetch())? {
        let version = trace_err!(Version::parse(&release.version))?;
        let is_server = version
            .build
            .get(0)
            .map(|b| b.to_string() != "client")
            .unwrap_or(true);
        if is_server && (!version.is_prerelease() || wants_prereleases) {
            return Ok((release, version));
        }
    }

    Err("No server release found".into())
}

fn get_server_update(update_channel: UpdateChannel) -> Option<(Release, Version)> {
    if matches!(update_channel, UpdateChannel::NoUpdates) {
        None
    } else {
        let current_version_string = ALVR_SERVER_VERSION.to_string();
        let current_nightly_tag = if let Some(index) = current_version_string.find("nightly") {
            &current_version_string[index..]
        } else {
            ""
        };

        get_latest_server_release(update_channel)
            .ok()
            .filter(|(_, version)| {
                let new_version_string = version.to_string();
                let new_nightly_tag = if let Some(index) = new_version_string.find("nightly") {
                    &new_version_string[index..]
                } else {
                    ""
                };

                // != operator ignores build metadata (such as nightly)
                *version != *ALVR_SERVER_VERSION || new_nightly_tag != current_nightly_tag
            })
    }
}

fn update(release: &Release) -> StrResult {
    kill_steamvr();

    let current_alvr_dir = current_alvr_dir()?;
    // patch for self_update replace_using_temp
    #[cfg(windows)]
    if !current_alvr_dir.starts_with("C:\\") {
        let folder_name = trace_none!(current_alvr_dir.file_name())?.to_string_lossy();
        return Err(format!(
            "ALVR cannot self update. Please close ALVR and move this folder ({}) somewhere in the C: drive.",
            folder_name
        ));
    }

    // get the first available release
    let asset = trace_none!(release.asset_for("alvr_server_windows"))?;
    println!("{:#?}\n", asset);
    let tmp_dir = trace_err!(tempfile::Builder::new()
        .prefix("self_update")
        .tempdir_in(env::temp_dir()))?;

    let tmp_tarball_path = &tmp_dir.path().join(&asset.name);
    let tmp_tarball = trace_err!(File::create(&tmp_tarball_path))?;
    let extract_dir = &tmp_dir.path().join("data");
    trace_err!(fs::create_dir_all(extract_dir))?;

    trace_err!(self_update::Download::from_url(&asset.download_url)
        .show_progress(true)
        .set_header(
            reqwest::header::ACCEPT,
            trace_err!("application/octet-stream".parse())?,
        )
        .download_to(&tmp_tarball))?;
    let file = trace_err!(fs::File::open(&tmp_tarball_path))?;
    let mut archive = trace_err!(zip::ZipArchive::new(file))?;

    for i in 0..archive.len() {
        let mut file = trace_err!(archive.by_index(i))?;

        let outpath = tmp_dir.path().join("data").join(file.name());

        if (&*file.name()).ends_with('/') {
            trace_err!(fs::create_dir_all(&outpath))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    trace_err!(fs::create_dir_all(&p))?;
                }
            }
            let mut outfile = trace_err!(fs::File::create(&outpath))?;
            trace_err!(io::copy(&mut file, &mut outfile))?;
        }
    }
    let extract_dir = extract_dir.join("alvr_server_windows");
    let bin_path = extract_dir.join(exec_fname("ALVR Launcher"));

    trace_err!(self_update::Move::from_source(&bin_path)
        .replace_using_temp(&tmp_dir.path().join("replacement_tmp"))
        .to_dest(&env::current_exe().unwrap()))?;

    create_replace_dir(&extract_dir, "web_gui")?;
    create_replace_dir(&extract_dir, "bin")?;
    create_replace_dir(&extract_dir, "resources")?;

    fs::remove_file(current_alvr_dir.join("crash_log.txt")).ok();
    trace_err!(fs::copy(
        &extract_dir.join("driver.vrdrivermanifest"),
        current_alvr_dir.join("driver.vrdrivermanifest"),
    ))?;

    Ok(())
}

fn create_replace_dir(from: &Path, dir_name: &str) -> StrResult {
    trace_err!(fs::remove_dir_all(dir_name))?;
    trace_err!(fs::create_dir_all(dir_name))?;
    trace_err!(dirx::copy(
        from.join(dir_name),
        current_alvr_dir()?,
        &dirx::CopyOptions::new(),
    ))?;

    Ok(())
}

fn window_mode() -> StrResult {
    let instance_mutex = trace_err!(single_instance::SingleInstance::new("alvr_launcher_mutex"))?;
    if instance_mutex.is_single() {
        struct InstanceMutex(single_instance::SingleInstance);
        unsafe impl Send for InstanceMutex {}

        let current_alvr_dir = current_alvr_dir()?;

        let instance_mutex = Arc::new(Mutex::new(Some(InstanceMutex(instance_mutex))));

        let session_manager = SessionManager::new(&current_alvr_dir);
        let settings = session_manager.get().to_settings();

        let html_content =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/gui/html/index.html"));
        let jquery = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/gui/js/jquery-3.5.1.min.js"
        ));
        let bootstrap_js = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/gui/js/bootstrap.min.js"
        ));
        let bootstrap_css = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/gui/css/bootstrap.min.css"
        ));

        let initial_window_position_flag = if cfg!(debug_assertions) {
            // Inbounds, to see the devtool window
            "--window-position=0,0"
        } else {
            // Out of bounds. No negative x for people with dual screen setup
            "--window-position=0,-1000"
        };

        let window = Arc::new(trace_err!(alcro::UIBuilder::new()
            .content(alcro::Content::Html(&html_content))
            .size(800, 600)
            .custom_args(&["--disk-cache-size=1", initial_window_position_flag])
            .run())?);

        trace_err!(window.bind("loadResources", {
            let window = window.clone();
            move |_| {
                trace_err!(window.eval(jquery))?;
                trace_err!(window.eval(bootstrap_js))?;
                trace_err!(window.load_css(bootstrap_css))?;

                trace_err!(window.eval("init()"))?;

                Ok(json::Value::Null)
            }
        }))?;

        if let Some((release, version)) = get_server_update(settings.extra.update_channel) {
            let prompt_before_update = settings.extra.prompt_before_update;
            trace_err!(window.bind("getUpdateInfo", move |_| Ok(json::json!({
                "version": version.to_string(),
                "prompt": prompt_before_update,
            }))))?;

            trace_err!(window.bind("update", {
                let window = window.clone();
                let current_alvr_dir = current_alvr_dir.clone();
                move |_| {
                    show_err_blocking(update(&release)).ok();
                    instance_mutex.lock().unwrap().take();
                    window.close();

                    maybe_open_launcher(&current_alvr_dir);

                    Ok(json::Value::Null)
                }
            }))?;
        } else {
            trace_err!(window.bind("getUpdateInfo", |_| Ok(json::json!({
                "version": null,
                "prompt": false,
            }))))?;
        }

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

        trace_err!(window.bind("startDriver", |_| {
            if !is_steamvr_running() && show_err(maybe_register_alvr_driver()).is_ok() {
                maybe_launch_steamvr();
            }
            Ok(json::Value::Null)
        }))?;

        trace_err!(window.bind("restartSteamvr", {
            let current_alvr_dir = current_alvr_dir.clone();
            move |_| {
                kill_steamvr();

                // If ALVR driver does not start use a more destructive approach: remove all drivers
                apply_driver_paths_backup(current_alvr_dir.clone())?;
                if let Ok(paths) = get_registered_drivers() {
                    driver_registration(&paths, false)?;
                }

                // register ALVR driver and start SteamVR again
                restart_steamvr();
                Ok(json::Value::Null)
            }
        }))?;

        // reload the page again, the first time the callbacks were not ready
        trace_err!(window.load(alcro::Content::Html(&html_content)))?;

        window.wait_finish();

        // This is needed in case the launcher window is closed before the driver is loaded,
        // otherwise this does nothing
        apply_driver_paths_backup(current_alvr_dir)?;
    }
    Ok(())
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    match args.get(1) {
        Some(flag) if flag == "--restart-steamvr" => restart_steamvr(),
        _ => {
            show_err_blocking(window_mode()).ok();
        }
    }
}
