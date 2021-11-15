#![windows_subsystem = "windows"] // hide terminal window

mod commands;
mod connection;
mod connection_utils;
mod dashboard;
mod driver_interop;
mod graphics_info;
mod logging_backend;
mod openvr;
mod web_server;

#[allow(non_camel_case_types, non_upper_case_globals, dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use alvr_ipc::IpcSseSender;
use bindings::*;

use alvr_common::prelude::*;
use alvr_filesystem::{self as afs, Layout};
use alvr_session::{ClientConnectionDesc, ServerEvent, SessionManager};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::fs;
use std::{
    collections::{hash_map::Entry, HashSet},
    ffi::{CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    ptr,
    sync::Arc,
};
use tokio::{
    runtime::Runtime,
    sync::{broadcast, mpsc, Notify},
};

lazy_static! {
    // Since ALVR_DIR is needed to initialize logging, if error then just panic
    static ref FILESYSTEM_LAYOUT: Layout =
        afs::filesystem_layout_from_launcher_exe(&std::env::current_exe().unwrap());
    static ref SESSION_MANAGER: Mutex<SessionManager> =
        Mutex::new(SessionManager::new(&FILESYSTEM_LAYOUT.session()));

    // Some of these globals can be removed by rewriting C++ code or refactoring
    static ref RUNTIME: Mutex<Option<Runtime>> = Mutex::new(Runtime::new().ok());
    static ref CHROME_DASHBOARD: Mutex<Option<Arc<alcro::UI>>> = Mutex::new(None);
    static ref NEW_DASHBOARD: Mutex<Option<Arc<alvr_gui::Dashboard>>> = Mutex::new(None);
    static ref LEGACY_SENDER: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>> = Mutex::new(None);
    static ref DRIVER_SENDER: Mutex<Option<IpcSseSender>> = Mutex::new(None);

    static ref CLIENTS_UPDATED_NOTIFIER: Notify = Notify::new();
    static ref RESTART_NOTIFIER: Notify = Notify::new();
    static ref SHUTDOWN_NOTIFIER: Notify = Notify::new();

    static ref FRAME_RENDER_VS_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/FrameRenderVS.cso").to_vec();
    static ref FRAME_RENDER_PS_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/FrameRenderPS.cso").to_vec();
    static ref QUAD_SHADER_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/QuadVertexShader.cso").to_vec();
    static ref COMPRESS_AXIS_ALIGNED_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/CompressAxisAlignedPixelShader.cso").to_vec();
    static ref COLOR_CORRECTION_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/ColorCorrectionPixelShader.cso").to_vec();
}

pub fn shutdown_runtime() {
    alvr_session::log_event(ServerEvent::ServerQuitting);

    if let Some(window) = CHROME_DASHBOARD.lock().take() {
        window.close();
    }

    SHUTDOWN_NOTIFIER.notify_waiters();

    if let Some(runtime) = RUNTIME.lock().take() {
        runtime.shutdown_background();
        // shutdown_background() is non blocking and it does not guarantee that every internal
        // thread is terminated in a timely manner. Using shutdown_background() instead of just
        // dropping the runtime has the benefit of giving SteamVR a chance to clean itself as
        // much as possible before the process is killed because of alvr_launcher timeout.
    }
}

pub enum ClientListAction {
    AddIfMissing { display_name: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub fn update_client_list(hostname: String, action: ClientListAction) {
    let mut client_connections = SESSION_MANAGER.lock().get().client_connections.clone();

    let maybe_client_entry = client_connections.entry(hostname);

    let mut updated = false;
    match action {
        ClientListAction::AddIfMissing { display_name } => {
            if let Entry::Vacant(new_entry) = maybe_client_entry {
                let client_connection_desc = ClientConnectionDesc {
                    trusted: false,
                    manual_ips: HashSet::new(),
                    display_name,
                };
                new_entry.insert(client_connection_desc);

                updated = true;
            }
        }
        ClientListAction::TrustAndMaybeAddIp(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                let client_connection_ref = entry.get_mut();
                client_connection_ref.trusted = true;
                if let Some(ip) = maybe_ip {
                    client_connection_ref.manual_ips.insert(ip);
                }

                updated = true;
            }
            // else: never happens. The function must be called with AddIfMissing{} first
        }
        ClientListAction::RemoveIpOrEntry(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                if let Some(ip) = maybe_ip {
                    entry.get_mut().manual_ips.remove(&ip);
                } else {
                    entry.remove_entry();
                }

                updated = true;
            }
        }
    }

    if updated {
        SESSION_MANAGER.lock().get_mut().client_connections = client_connections;

        CLIENTS_UPDATED_NOTIFIER.notify_waiters();
    }
}

fn main() -> StrResult {
    let (log_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    let (events_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    logging_backend::init_logging(log_sender.clone(), events_sender.clone());

    // Acquire and drop the session_manager lock to create session.json if not present
    // this is needed until Settings.cpp is replaced with Rust.
    SESSION_MANAGER.lock().get_mut();

    RUNTIME.lock().as_mut().unwrap().spawn(async move {
        let connections = SESSION_MANAGER.lock().get().client_connections.clone();
        for (hostname, connection) in connections {
            if !connection.trusted {
                update_client_list(hostname, ClientListAction::RemoveIpOrEntry(None));
            }
        }

        let web_server =
            alvr_common::show_err_async(web_server::web_server(log_sender, events_sender.clone()));

        tokio::select! {
            _ = web_server => (),
            // _ = dashboard::event_listener(events_sender) => (), for new dashboard
            _ = connection::connection_lifecycle_loop() => (),
            _ = driver_interop::driver_lifecycle_loop() => (),
            _ = SHUTDOWN_NOTIFIER.notified() => (),
        }
    });

    unsafe {
        g_sessionPath = CString::new(FILESYSTEM_LAYOUT.session().to_string_lossy().to_string())
            .unwrap()
            .into_raw();
        g_driverRootDir = CString::new(
            FILESYSTEM_LAYOUT
                .openvr_driver_root_dir
                .to_string_lossy()
                .to_string(),
        )
        .unwrap()
        .into_raw();
    };

    unsafe {
        FRAME_RENDER_VS_CSO_PTR = FRAME_RENDER_VS_CSO.as_ptr();
        FRAME_RENDER_VS_CSO_LEN = FRAME_RENDER_VS_CSO.len() as _;
        FRAME_RENDER_PS_CSO_PTR = FRAME_RENDER_PS_CSO.as_ptr();
        FRAME_RENDER_PS_CSO_LEN = FRAME_RENDER_PS_CSO.len() as _;
        QUAD_SHADER_CSO_PTR = QUAD_SHADER_CSO.as_ptr();
        QUAD_SHADER_CSO_LEN = QUAD_SHADER_CSO.len() as _;
        COMPRESS_AXIS_ALIGNED_CSO_PTR = COMPRESS_AXIS_ALIGNED_CSO.as_ptr();
        COMPRESS_AXIS_ALIGNED_CSO_LEN = COMPRESS_AXIS_ALIGNED_CSO.len() as _;
        COLOR_CORRECTION_CSO_PTR = COLOR_CORRECTION_CSO.as_ptr();
        COLOR_CORRECTION_CSO_LEN = COLOR_CORRECTION_CSO.len() as _;
    }

    unsafe extern "C" fn log_error(string_ptr: *const c_char) {
        alvr_common::show_e(CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe fn log(level: log::Level, string_ptr: *const c_char) {
        log::log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe extern "C" fn log_warn(string_ptr: *const c_char) {
        log(log::Level::Warn, string_ptr);
    }

    unsafe extern "C" fn log_info(string_ptr: *const c_char) {
        log(log::Level::Info, string_ptr);
    }

    unsafe extern "C" fn log_debug(string_ptr: *const c_char) {
        log(log::Level::Debug, string_ptr);
    }

    extern "C" fn legacy_send(buffer_ptr: *mut u8, len: i32) {
        if let Some(sender) = &*LEGACY_SENDER.lock() {
            let mut vec_buffer = vec![0; len as _];

            // use copy_nonoverlapping (aka memcpy) to avoid freeing memory allocated by C++
            unsafe {
                ptr::copy_nonoverlapping(buffer_ptr, vec_buffer.as_mut_ptr(), len as _);
            }

            sender.send(vec_buffer).ok();
        }
    }

    unsafe {
        LogError = Some(log_error);
        LogWarn = Some(log_warn);
        LogInfo = Some(log_info);
        LogDebug = Some(log_debug);
        LegacySend = Some(legacy_send);
    }

    unsafe { InitializeCpp() };

    // Dashboard window:

    const WINDOW_WIDTH: u32 = 800;
    const WINDOW_HEIGHT: u32 = 600;

    let (pos_left, pos_top) =
        if let Ok((screen_width, screen_height)) = graphics_info::get_screen_size() {
            (
                (screen_width - WINDOW_WIDTH) / 2,
                (screen_height - WINDOW_HEIGHT) / 2,
            )
        } else {
            (0, 0)
        };

    let temp_dir = trace_err!(tempfile::TempDir::new())?;
    let user_data_dir = temp_dir.path();
    trace_err!(fs::File::create(
        temp_dir.path().join("FirstLaunchAfterInstallation")
    ))?;

    let window = Arc::new(trace_err!(alcro::UIBuilder::new()
        .content(alcro::Content::Url("http://127.0.0.1:8082"))
        .user_data_dir(user_data_dir)
        .size(WINDOW_WIDTH as _, WINDOW_HEIGHT as _)
        .custom_args(&[
            "--disk-cache-size=1",
            &format!("--window-position={},{}", pos_left, pos_top)
        ])
        .run())?);

    *CHROME_DASHBOARD.lock() = Some(Arc::clone(&window));

    window.wait_finish();

    // prevent panic on window.close()
    *CHROME_DASHBOARD.lock() = None;
    shutdown_runtime();

    // unsafe { ShutdownSteamvr() };

    Ok(())
}
