mod connection;
mod connection_utils;
mod logging_backend;
mod openvr;
mod web_server;

#[allow(non_camel_case_types, non_upper_case_globals, dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

use alvr_common::{
    commands,
    data::{ClientConnectionDesc, SessionManager},
    graphics, logging,
    prelude::*,
};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::{
    collections::{hash_map::Entry, HashSet},
    ffi::{c_void, CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    path::PathBuf,
    ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Once,
    },
    thread,
    time::Duration,
};
use tokio::{
    runtime::Runtime,
    sync::{broadcast, mpsc, Notify},
};

lazy_static! {
    // Since ALVR_DIR is needed to initialize logging, if error then just panic
    static ref ALVR_DIR: PathBuf = {
        commands::get_alvr_dir().unwrap()
    };
    static ref SESSION_MANAGER: Mutex<SessionManager> = Mutex::new(SessionManager::new(&ALVR_DIR));
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(Runtime::new().ok());
    static ref CLIENTS_UPDATED_NOTIFIER: Notify = Notify::new();
    static ref MAYBE_WINDOW: Mutex<Option<Arc<alcro::UI>>> = Mutex::new(None);
    static ref MAYBE_LEGACY_SENDER: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>> =
        Mutex::new(None);
    static ref RESTART_NOTIFIER: Notify = Notify::new();
    static ref SHUTDOWN_NOTIFIER: Notify = Notify::new();

    static ref FRAME_RENDER_VS_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/FrameRenderVS.cso").to_vec();
    static ref FRAME_RENDER_PS_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/FrameRenderPS.cso").to_vec();
    static ref QUAD_SHADER_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/QuadVertexShader.cso").to_vec();
    static ref COMPRESS_SLICES_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/CompressSlicesPixelShader.cso").to_vec();
    static ref COLOR_CORRECTION_CSO: Vec<u8> =
        include_bytes!("../cpp/platform/win32/ColorCorrectionPixelShader.cso").to_vec();
}

pub fn shutdown_runtime() {
    if let Some(window) = MAYBE_WINDOW.lock().take() {
        window.close();
    }

    SHUTDOWN_NOTIFIER.notify_waiters();

    if let Some(runtime) = MAYBE_RUNTIME.lock().take() {
        runtime.shutdown_background();
        // shutdown_background() is non blocking and it does not guarantee that every internal
        // thread is terminated in a timely manner. Using shutdown_background() instead of just
        // dropping the runtime has the benefit of giving SteamVR a chance to clean itself as
        // much as possible before the process is killed because of alvr_launcher timeout.
    }
}

pub fn notify_shutdown_driver() {
    thread::spawn(|| {
        RESTART_NOTIFIER.notify_waiters();

        // give time to the control loop to send the restart packet (not crucial)
        thread::sleep(Duration::from_millis(100));

        shutdown_runtime();

        unsafe { ShutdownSteamvr() };
    });
}

pub fn notify_restart_driver() {
    notify_shutdown_driver();

    commands::restart_steamvr(&ALVR_DIR).ok();
}

pub fn notify_application_update() {
    notify_shutdown_driver();

    commands::invoke_application_update(&ALVR_DIR).ok();
}

pub enum ClientListAction {
    AddIfMissing { display_name: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub async fn update_client_list(hostname: String, action: ClientListAction) {
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

// this thread gets interrupted when SteamVR closes
// todo: handle this in a better way
fn ui_thread() -> StrResult {
    const WINDOW_WIDTH: u32 = 800;
    const WINDOW_HEIGHT: u32 = 600;

    let (pos_left, pos_top) = if let Ok((screen_width, screen_height)) = graphics::get_screen_size()
    {
        (
            (screen_width - WINDOW_WIDTH) / 2,
            (screen_height - WINDOW_HEIGHT) / 2,
        )
    } else {
        (0, 0)
    };

    let window = Arc::new(trace_err!(alcro::UIBuilder::new()
        .content(alcro::Content::Url("http://127.0.0.1:8082"))
        .size(WINDOW_WIDTH as _, WINDOW_HEIGHT as _)
        .custom_args(&[
            "--disk-cache-size=1",
            &format!("--window-position={},{}", pos_left, pos_top)
        ])
        .run())?);

    *MAYBE_WINDOW.lock() = Some(Arc::clone(&window));

    window.wait_finish();

    // prevent panic on window.close()
    *MAYBE_WINDOW.lock() = None;
    shutdown_runtime();

    unsafe { ShutdownSteamvr() };

    Ok(())
}

fn init() {
    let (log_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    let (events_sender, _) = broadcast::channel(web_server::WS_BROADCAST_CAPACITY);
    logging_backend::init_logging(log_sender.clone(), events_sender.clone());

    if let Some(runtime) = MAYBE_RUNTIME.lock().as_mut() {
        // Acquire and drop the session_manager lock to create session.json if not present
        // this is needed until Settings.cpp is replaced with Rust. todo: remove
        SESSION_MANAGER.lock().get_mut();

        runtime.spawn(async move {
            let connections = SESSION_MANAGER.lock().get().client_connections.clone();
            for (hostname, connection) in connections {
                if !connection.trusted {
                    update_client_list(hostname, ClientListAction::RemoveIpOrEntry(None)).await;
                }
            }

            let web_server =
                logging::show_err_async(web_server::web_server(log_sender, events_sender));

            tokio::select! {
                _ = web_server => (),
                _ = SHUTDOWN_NOTIFIER.notified() => (),
            }
        });

        thread::spawn(|| logging::show_err(ui_thread()));
    }

    let alvr_dir_c_string = CString::new(ALVR_DIR.to_string_lossy().to_string()).unwrap();
    unsafe { g_alvrDir = alvr_dir_c_string.into_raw() };
}

#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(init);

    FRAME_RENDER_VS_CSO_PTR = FRAME_RENDER_VS_CSO.as_ptr();
    FRAME_RENDER_VS_CSO_LEN = FRAME_RENDER_VS_CSO.len() as _;
    FRAME_RENDER_PS_CSO_PTR = FRAME_RENDER_PS_CSO.as_ptr();
    FRAME_RENDER_PS_CSO_LEN = FRAME_RENDER_PS_CSO.len() as _;
    QUAD_SHADER_CSO_PTR = QUAD_SHADER_CSO.as_ptr();
    QUAD_SHADER_CSO_LEN = QUAD_SHADER_CSO.len() as _;
    COMPRESS_SLICES_CSO_PTR = COMPRESS_SLICES_CSO.as_ptr();
    COMPRESS_SLICES_CSO_LEN = COMPRESS_SLICES_CSO.len() as _;
    COLOR_CORRECTION_CSO_PTR = COLOR_CORRECTION_CSO.as_ptr();
    COLOR_CORRECTION_CSO_LEN = COLOR_CORRECTION_CSO.len() as _;

    unsafe extern "C" fn log_error(string_ptr: *const c_char) {
        logging::show_e(CStr::from_ptr(string_ptr).to_string_lossy());
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
        if let Some(sender) = &*MAYBE_LEGACY_SENDER.lock() {
            let mut vec_buffer = vec![0; len as _];

            // use copy_nonoverlapping (aka memcpy) to avoid freeing memory allocated by C++
            unsafe {
                ptr::copy_nonoverlapping(buffer_ptr, vec_buffer.as_mut_ptr(), len as _);
            }

            sender.send(vec_buffer).ok();
        }
    }

    pub extern "C" fn driver_ready_idle(set_default_chap: bool) {
        logging::show_err(commands::apply_driver_paths_backup(ALVR_DIR.clone()));

        if let Some(runtime) = &mut *MAYBE_RUNTIME.lock() {
            runtime.spawn(async move {
                if set_default_chap {
                    // call this when inside a new tokio thread. Calling this on the parent thread will
                    // crash SteamVR
                    unsafe { SetDefaultChaperone() };
                }
                tokio::select! {
                    _ = connection::connection_lifecycle_loop() => (),
                    _ = SHUTDOWN_NOTIFIER.notified() => (),
                }
            });
        }
    }

    extern "C" fn _shutdown_runtime() {
        shutdown_runtime();
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    DriverReadyIdle = Some(driver_ready_idle);
    LegacySend = Some(legacy_send);
    ShutdownRuntime = Some(_shutdown_runtime);

    // cast to usize to allow the variables to cross thread boundaries
    let interface_name_usize = interface_name as usize;
    let return_code_usize = return_code as usize;

    lazy_static::lazy_static! {
        static ref MAYBE_PTR_USIZE: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        static ref NUM_TRIALS: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    }

    thread::spawn(move || {
        NUM_TRIALS.fetch_add(1, Ordering::Relaxed);
        if NUM_TRIALS.load(Ordering::Relaxed) <= 1 {
            MAYBE_PTR_USIZE.store(
                CppEntryPoint(interface_name_usize as _, return_code_usize as _) as _,
                Ordering::Relaxed,
            );
        }
    })
    .join()
    .ok();

    MAYBE_PTR_USIZE.load(Ordering::Relaxed) as _
}
