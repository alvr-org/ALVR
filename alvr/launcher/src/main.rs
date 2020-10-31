// #![windows_subsystem = "windows"]

use alvr_common::{commands::*, data::ALVR_SERVER_VERSION, *};
use logging::{show_e, show_err};
use serde_json as json;
use std::{env, fs::File, fs, path::PathBuf, process::Command, io, sync::{Arc, Mutex}, thread, time::{Duration, Instant}};
use fs_extra::dir::{CopyOptions, copy};


#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

// fn get_latest_version() -> Result<version_compare::Version<'static>, StrResult> {
//     let latest_url = "https://api.github.com/repos/Nexite/ALVR/releases/latest";
//     let resp = reqwest::blocking::get(latest_url).unwrap().json().unwrap();
//     Ok(Version::from(resp["tag_name"]).unwrap())
// }

fn window_mode() -> StrResult {
    let instance_mutex = trace_err!(single_instance::SingleInstance::new("alvr_launcher_mutex"))?;
    if instance_mutex.is_single() {
        struct InstanceMutex(single_instance::SingleInstance);
        unsafe impl Send for InstanceMutex {}

        let instance_mutex = Arc::new(Mutex::new(Some(InstanceMutex(instance_mutex))));

        maybe_delete_alvr_dir_storage();

        let html_content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/html/index.html"));
        let window = Arc::new(trace_err!(alcro::UIBuilder::new()
            .content(alcro::Content::Html(html_content))
            .size(200, 200)
            .custom_args(&["--disk-cache-size=1"])
            .run())?);

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

        trace_err!(window.bind("update", {
            let window = window.clone();
            move |_| {
                show_err(update()).ok();
                instance_mutex.lock().unwrap().take();
                window.close();

                // reopen alvr
                let mut command =
                    Command::new(::std::env::current_dir().unwrap().join("ALVR launcher"));
                command.spawn().ok();

                Ok(json::Value::Null)
            }
        }))?;

        // trace_err!(window.eval("init()"))?;

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

fn update() -> StrResult {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("Nexite")
        .repo_name("ALVR")
        .build().unwrap()
        .fetch().unwrap();
    // println!("found releases:");
    // println!("{:#?}\n", releases);

    // get the first available release
    let asset = releases[0].asset_for("autoupdater").unwrap();
    println!("{:#?}\n", asset);
    let tmp_dir = tempfile::Builder::new()
        .prefix("self_update")
        .tempdir_in(::std::env::temp_dir()).unwrap();

    let tmp_tarball_path = &tmp_dir.path().join(&asset.name);
    let tmp_tarball = File::create(&tmp_tarball_path).unwrap();
    let extract_dir = &tmp_dir.path().join("data");
    fs::create_dir_all(extract_dir).ok();

    self_update::Download::from_url(&asset.download_url)
        .show_progress(true)
        .set_header(reqwest::header::ACCEPT, "application/octet-stream".parse().unwrap())
        .download_to(&tmp_tarball).ok();
    let file = fs::File::open(&tmp_tarball_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        #[allow(deprecated)]
        let outpath = tmp_dir.path().join("data").join(file.sanitized_name());

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    let tmp_file = tmp_dir.path().join("replacement_tmp");
    let bin_path = extract_dir.join("ALVR launcher.exe");
    let driver_manifest_path = extract_dir.join("driver.vrdrivermanifest");

    self_update::Move::from_source(&bin_path)
        .replace_using_temp(&tmp_file)
        .to_dest(&::std::env::current_exe().unwrap()).ok();

    let tmp_file = tmp_dir.path().join("replacement_tmp");
    self_update::Move::from_source(&driver_manifest_path)
        .replace_using_temp(&tmp_file)
        .to_dest(current_alvr_dir()?.as_path()).ok();

    create_replace(extract_dir, "web_gui")?;
    create_replace(extract_dir, "bin")?;
    create_replace(extract_dir, "resources")?;
    Ok(())
}

fn create_replace(from: &PathBuf, path: &str) -> StrResult {
    fs::remove_dir_all(path);
    fs::create_dir_all(path).unwrap();
    let options = CopyOptions::new();
    copy(from.join(path), &::std::env::current_dir().unwrap(), &options).ok();
    Ok(())
}
