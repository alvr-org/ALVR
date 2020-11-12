#![windows_subsystem = "windows"]

use alvr_common::{*, commands::*, data::ALVR_SERVER_VERSION, logging::show_err};
use serde_json as json;
use version_compare::{CompOp, Version};
use std::{env, fs::File, fs, path::PathBuf, process::Command, io, sync::{Arc, Mutex}, thread, time::{Duration, Instant}};
use fs_extra::{dir, file};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;


fn main() -> StrResult {
    let instance_mutex = trace_err!(single_instance::SingleInstance::new("alvr_launcher_mutex"))?;
    if instance_mutex.is_single() {
        struct InstanceMutex(single_instance::SingleInstance);
        unsafe impl Send for InstanceMutex {}
        
        let instance_mutex = Arc::new(Mutex::new(Some(InstanceMutex(instance_mutex))));

        let html_content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/client_gui/html/index.html"));
        let jquery = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/client_gui/js/jquery-3.5.1.min.js"));
        let bootstrap_js = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/client_gui/js/bootstrap.min.js"));
        let bootstrap_css = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/client_gui/css/bootstrap.min.css"));
        
        let window = Arc::new(trace_err!(alcro::UIBuilder::new()
            .content(alcro::Content::Html(&html_content))
            .size(0, 0)
            .custom_args(&["--disk-cache-size=1", "--window-position=-1000,-1000"])
            .run())?);

            trace_err!(window.bind("restartServer", |_| {
                maybe_kill_web_server();
                maybe_launch_web_server(&std::env::current_dir().unwrap());
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
                    command.creation_flags(CREATE_NO_WINDOW).spawn().ok();
    
                    Ok(json::Value::Null)
                }
            }))?;
            
            trace_err!(window.bind("loadJQuery",  {
                let window = window.clone();
                move |_| {
                    trace_err!(window.eval(jquery))?;
                    trace_err!(window.eval(bootstrap_js))?;
                    trace_err!(window.load_css(bootstrap_css))?;
                    Ok(json::Value::Null)
            }}))?;
            trace_err!(window.load(alcro::Content::Html(&html_content)))?;
            
            if check_for_update() {
                trace_err!(window.eval("promptUpdate()"))?;
            } else {
                maybe_launch_web_server(&std::env::current_dir().unwrap());
                trace_err!(window.eval("init()"))?;
            }
    
            window.wait_finish();

        maybe_kill_web_server();
    }
    Ok(())
}
fn check_for_update() -> bool {
    // change Nexite to JackD83 for actual release
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("JackD83")
        .repo_name("ALVR")
        .build().unwrap()
        .fetch().unwrap();

    let latest_version = Version::from(&releases[0].version).unwrap();
    return Version::from(&ALVR_SERVER_VERSION.to_string()).unwrap().compare(&latest_version) == CompOp::Lt;
}
// change Nexite to JackD83 for actual release
fn update() -> StrResult {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("JackD83")
        .repo_name("ALVR")
        .build().unwrap()
        .fetch().unwrap();

    // get the first available release
    let asset = releases[0].asset_for("alvr_server_windows").unwrap();
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
    let extract_dir = extract_dir.join("alvr_server_windows");
    let tmp_file = tmp_dir.path().join("replacement_tmp");
    let bin_path = extract_dir.join("ALVR launcher.exe");
    let driver_manifest_path = extract_dir.join("driver.vrdrivermanifest");

    self_update::Move::from_source(&bin_path)
        .replace_using_temp(&tmp_file)
        .to_dest(&::std::env::current_exe().unwrap()).ok();

    let tmp_file = tmp_dir.path().join("replacement_tmp");
    self_update::Move::from_source(&driver_manifest_path)
        .replace_using_temp(&tmp_file)
        .to_dest(get_alvr_dir()?.as_path()).ok();

    create_replace_dir(&extract_dir, "web_gui")?;
    create_replace_dir(&extract_dir, "bin")?;
    create_replace_dir(&extract_dir, "resources")?;
    create_replace_file(&extract_dir, "driver.vrdrivermanifest")?;
    create_replace_file(&extract_dir, "alvr_web_server.exe")?;
    Ok(())
}

fn create_replace_dir(from: &PathBuf, path: &str) -> StrResult {
    fs::remove_dir_all(path);
    fs::create_dir_all(path).unwrap();
    let options = dir::CopyOptions::new();
    dir::copy(from.join(path), &::std::env::current_dir().unwrap(), &options).ok();
    Ok(())
}

fn create_replace_file(from: &PathBuf, path: &str) -> StrResult {

    fs::remove_file(PathBuf::from(path));

    let options = file::CopyOptions::new();
    file::copy(from.join(path), PathBuf::from(path), &options).ok();
    Ok(())
}
