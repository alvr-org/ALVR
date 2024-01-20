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

use dashboard::Dashboard;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use alvr_common::ALVR_VERSION;
    use eframe::{
        egui::{IconData, ViewportBuilder},
        NativeOptions,
    };
    use ico::IconDir;
    use std::{env, fs};
    use std::{io::Cursor, sync::mpsc};

    let (server_events_sender, server_events_receiver) = mpsc::channel();
    logging_backend::init_logging(server_events_sender.clone());

    {
        let mut data_manager = data_sources::get_local_data_source();

        data_manager.clean_client_list();

        #[cfg(target_os = "linux")]
        {
            let has_nvidia = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN,
                flags: wgpu::InstanceFlags::empty(),
                dx12_shader_compiler: Default::default(),
                gles_minor_version: Default::default(),
            })
            .enumerate_adapters(wgpu::Backends::VULKAN)
            .iter()
            .any(|adapter| adapter.get_info().vendor == 0x10de);

            if has_nvidia {
                data_manager
                    .session_mut()
                    .session_settings
                    .patches
                    .linux_async_reprojection = false;
            }
        }

        if data_manager.session().server_version != *ALVR_VERSION {
            let mut session_ref = data_manager.session_mut();
            session_ref.server_version = ALVR_VERSION.clone();
            session_ref.client_connections.clear();
            session_ref.session_settings.open_setup_wizard = true;
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
                .with_inner_size((870.0, 600.0))
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

                Box::new(Dashboard::new(creation_context, data_source))
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
