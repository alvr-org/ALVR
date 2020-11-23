#![windows_subsystem = "windows"]

use alvr_common::{
    commands::*,
    data::{ALVR_NAME, ALVR_SERVER_VERSION},
    logging::{show_e, show_err},
    *,
};
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
};

fn alvr_dir() -> PathBuf {
    env::current_exe().unwrap().parent().unwrap().to_owned()
}

fn main() -> StrResult {
    let instance_mutex = trace_err!(single_instance::SingleInstance::new("alvr_launcher_mutex"))?;
    if instance_mutex.is_single() {
        struct InstanceMutex(single_instance::SingleInstance);
        unsafe impl Send for InstanceMutex {}

        let instance_mutex = Arc::new(Mutex::new(Some(InstanceMutex(instance_mutex))));

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

        if let Some((release, version)) = get_server_update() {
            trace_err!(window.bind("getUpdateVersion", {
                move |_| Ok(json::Value::String(format!("v{}", version.to_string())))
            }))?;

            trace_err!(window.bind("update", {
                let window = window.clone();
                move |_| {
                    show_err(update(&release)).ok();
                    instance_mutex.lock().unwrap().take();
                    window.close();

                    maybe_open_launcher(&alvr_dir());

                    Ok(json::Value::Null)
                }
            }))?;
        } else {
            trace_err!(window.bind("getUpdateVersion", |_| Ok(json::Value::Null)))?;
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

        trace_err!(window.bind("startWebServer", |_| {
            maybe_launch_web_server(&alvr_dir());
            Ok(json::Value::Null)
        }))?;

        trace_err!(window.bind("restartServer", |_| {
            maybe_kill_web_server();
            maybe_launch_web_server(&alvr_dir());
            Ok(json::Value::Null)
        }))?;

        // reload the page again, the first time the callbacks were not ready
        trace_err!(window.load(alcro::Content::Html(&html_content)))?;

        window.wait_finish();

        maybe_kill_web_server();
    }
    Ok(())
}

fn get_latest_server_release() -> StrResult<(Release, Version)> {
    let release_list = trace_err!(ReleaseList::configure()
        .repo_owner("JackD83")
        .repo_name(ALVR_NAME)
        .build())?;

    let wants_prereleases = ALVR_SERVER_VERSION.is_prerelease();

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

fn get_server_update() -> Option<(Release, Version)> {
    get_latest_server_release()
        .ok()
        .filter(|(_, version)| *version != *ALVR_SERVER_VERSION)
}

// change Nexite to JackD83 for actual release
fn update(release: &Release) -> StrResult {
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

    trace_err!(fs::copy(
        &extract_dir.join(exec_fname("alvr_web_server")),
        alvr_dir().join(exec_fname("alvr_web_server")),
    ))?;
    trace_err!(fs::copy(
        &extract_dir.join("driver.vrdrivermanifest"),
        alvr_dir().join("driver.vrdrivermanifest"),
    ))?;

    Ok(())
}

fn create_replace_dir(from: &Path, dir_name: &str) -> StrResult {
    trace_err!(fs::remove_dir_all(dir_name))?;
    trace_err!(fs::create_dir_all(dir_name))?;
    trace_err!(dirx::copy(
        from.join(dir_name),
        alvr_dir(),
        &dirx::CopyOptions::new(),
    ))?;

    Ok(())
}
