#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod dashboard;
mod data_sources;
mod firewall;
mod logging_backend;
mod steamvr_launcher;
mod theme;

use alvr_common::{parking_lot::Mutex, ALVR_VERSION};
use alvr_sockets::GpuVendor;
use dashboard::Dashboard;
use data_sources::ServerEvent;
use eframe::{egui, IconData, NativeOptions};
use ico::IconDir;
use std::{
    io::Cursor,
    sync::{mpsc, Arc},
    thread,
};

fn main() {
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
                .extra
                .patches
                .linux_async_reprojection = false;
        }

        if data_manager.session().server_version != *ALVR_VERSION {
            let mut session_ref = data_manager.session_mut();
            session_ref.server_version = ALVR_VERSION.clone();
            session_ref.client_connections.clear();
        }
    }

    let ico = IconDir::read(Cursor::new(include_bytes!("../resources/dashboard.ico"))).unwrap();
    let image = ico.entries().first().unwrap().decode().unwrap();

    let data_thread = Arc::new(Mutex::new(None));

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
            let data_thread = Arc::clone(&data_thread);
            Box::new(move |creation_context| {
                let (dashboard_requests_sender, dashboard_requests_receiver) = mpsc::channel();

                let context = creation_context.egui_ctx.clone();
                *data_thread.lock() = Some(thread::spawn(|| {
                    data_sources::data_interop_thread(
                        context,
                        dashboard_requests_receiver,
                        server_events_sender,
                    )
                }));

                Box::new(Dashboard::new(
                    creation_context,
                    dashboard_requests_sender,
                    server_events_receiver,
                ))
            })
        },
    )
    .unwrap();

    data_thread.lock().take().unwrap().join().unwrap();
}
