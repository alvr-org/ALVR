// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dashboard;

#[cfg(not(target_arch = "wasm32"))]
mod data_sources;
#[cfg(target_arch = "wasm32")]
mod data_sources_wasm;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(target_os = "linux")]
mod linux_checks;
#[cfg(not(target_arch = "wasm32"))]
mod logging_backend;
#[cfg(not(target_arch = "wasm32"))]
mod steamvr_launcher;

#[cfg(not(target_arch = "wasm32"))]
use data_sources::DataSources;
#[cfg(target_arch = "wasm32")]
use data_sources_wasm::DataSources;

use alvr_filesystem as afs;
use dashboard::Dashboard;

fn get_filesystem_layout() -> afs::Layout {
    afs::filesystem_layout_from_dashboard_exe(&std::env::current_exe().unwrap()).unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use alvr_common::ALVR_VERSION;
    use alvr_common::info;
    use alvr_filesystem as afs;
    use eframe::{
        NativeOptions,
        egui::{IconData, ViewportBuilder},
    };
    use ico::IconDir;
    use std::{env, ffi::OsStr, fs};
    use std::{io::Cursor, sync::mpsc};

    let (server_events_sender, server_events_receiver) = mpsc::channel();
    logging_backend::init_logging(server_events_sender.clone());

    // Kill any other dashboard instance
    let self_path = std::env::current_exe().unwrap().canonicalize().unwrap();
    for proc in sysinfo::System::new_all().processes_by_name(OsStr::new(&afs::dashboard_fname())) {
        // According to implementation notes, on linux the returned path can be empty due to
        // privileges, so canonicalize can fail
        if let Some(other_path) = proc.exe().and_then(|path| path.canonicalize().ok())
            && other_path != self_path
        {
            info!(
                "Killing other dashboard process with path {}",
                other_path.display()
            );
            proc.kill();
        }
    }

    #[cfg(target_os = "linux")]
    linux_checks::audio_check();

    data_sources::clean_session();

    if data_sources::get_read_only_local_session()
        .settings()
        .extra
        .steamvr_launcher
        .open_close_steamvr_with_dashboard
    {
        steamvr_launcher::LAUNCHER.lock().launch_steamvr()
    }

    let ico = IconDir::read(Cursor::new(include_bytes!("../resources/dashboard.ico"))).unwrap();
    let image = ico.entries().first().unwrap().decode().unwrap();

    // Workaround for the steam deck
    if fs::read_to_string("/sys/devices/virtual/dmi/id/board_vendor")
        .map(|vendor| vendor.trim() == "Valve")
        .unwrap_or(false)
    {
        unsafe { env::set_var("WINIT_X11_SCALE_FACTOR", "1") };
    }

    eframe::run_native(
        &format!("ALVR Dashboard (streamer v{})", *ALVR_VERSION),
        NativeOptions {
            viewport: ViewportBuilder::default()
                .with_app_id("alvr.dashboard")
                .with_inner_size((900.0, 600.0))
                .with_icon(IconData {
                    rgba: image.rgba_data().to_owned(),
                    width: image.width(),
                    height: image.height(),
                }),
            centered: true,
            ..Default::default()
        },
        {
            Box::new(move |creation_context| {
                let data_source = DataSources::new(
                    creation_context.egui_ctx.clone(),
                    server_events_sender,
                    server_events_receiver,
                );

                Ok(Box::new(Dashboard::new(creation_context, data_source)))
            })
        },
    )
    .unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start("dashboard_canvas", eframe::WebOptions::default(), {
                Box::new(move |creation_context| {
                    let context = creation_context.egui_ctx.clone();
                    Box::new(Dashboard::new(creation_context, DataSources::new(context)))
                })
            })
            .await
            .ok();
    });
}
