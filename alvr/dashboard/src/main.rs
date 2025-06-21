// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dashboard;

#[cfg(not(target_arch = "wasm32"))]
mod data_sources;
#[cfg(target_arch = "wasm32")]
mod data_sources_wasm;
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
    use alvr_filesystem as afs;
    use eframe::{
        egui::{IconData, ViewportBuilder},
        NativeOptions,
    };
    use ico::IconDir;
    use std::{env, ffi::OsStr, fs};
    use std::{io::Cursor, sync::mpsc};

    #[cfg(target_os = "windows")]
    {
        use std::process;
        // don't launch dashboard if another instance is already running
        let process_id = process::id();
        //todo: focus already open dashboard before exiting
        for proc in
            sysinfo::System::new_all().processes_by_name(OsStr::new(&afs::dashboard_fname()))
        {
            if proc.pid().as_u32() != process_id {
                return;
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Kill any other dashboard instance
        let self_path = std::env::current_exe().unwrap();
        for proc in
            sysinfo::System::new_all().processes_by_name(OsStr::new(&afs::dashboard_fname()))
        {
            if let Some(other_path) = proc.exe() {
                if other_path != self_path {
                    proc.kill();
                }
            }
        }
    }

    let (server_events_sender, server_events_receiver) = mpsc::channel();
    logging_backend::init_logging(server_events_sender.clone());

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
        env::set_var("WINIT_X11_SCALE_FACTOR", "1");
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
