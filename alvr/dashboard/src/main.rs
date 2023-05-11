#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod dashboard;
mod logging_backend;
mod theme;

#[cfg(not(target_arch = "wasm32"))]
mod data_sources;
#[cfg(not(target_arch = "wasm32"))]
mod steamvr_launcher;

use alvr_common::ALVR_VERSION;
use alvr_packets::GpuVendor;
use dashboard::Dashboard;
use data_sources::DataSources;
use eframe::egui;
use ico::IconDir;
use std::{io::Cursor, sync::mpsc};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use eframe::{IconData, NativeOptions};

    let (server_events_sender, server_events_receiver) = mpsc::channel();
    logging_backend::init_logging(server_events_sender.clone());

    {
        let mut data_manager = data_sources::get_local_data_source();
        if data_manager
            .get_gpu_vendors()
            .iter()
            .any(|vendor| matches!(vendor, GpuVendor::Nvidia))
        {
            data_manager
                .session_mut()
                .session_settings
                .patches
                .linux_async_reprojection = false;
        }

        if data_manager.session().server_version != *ALVR_VERSION {
            let mut session_ref = data_manager.session_mut();
            session_ref.server_version = ALVR_VERSION.clone();
            session_ref.client_connections.clear();
        }

        if data_manager
            .settings()
            .steamvr_launcher
            .open_close_steamvr_with_dashboard
        {
            steamvr_launcher::LAUNCHER.lock().launch_steamvr()
        }
    }

    let ico = IconDir::read(Cursor::new(include_bytes!("../resources/dashboard.ico"))).unwrap();
    let image = ico.entries().first().unwrap().decode().unwrap();

    eframe::run_native(
        &format!("ALVR Dashboard (streamer v{})", *ALVR_VERSION),
        NativeOptions {
            icon_data: Some(IconData {
                rgba: image.rgba_data().to_owned(),
                width: image.width(),
                height: image.height(),
            }),
            initial_window_size: Some(egui::vec2(850.0, 600.0)),
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

                Box::new(Dashboard::new(creation_context, data_source))
            })
        },
    )
    .unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
