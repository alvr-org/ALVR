#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types, non_upper_case_globals)]

mod connection;
mod logging_backend;
mod web_server;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alcro::JSResult;
use alvr_common::{data::*, logging::*, *};
use lazy_static::lazy_static;
use lazy_static_include::*;
use parking_lot::Mutex;
use std::{
    collections::{hash_map::Entry, HashSet},
    ffi::{c_void, CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    path::PathBuf,
    sync::{atomic::AtomicUsize, atomic::Ordering, Arc, Once},
    thread,
    time::Duration,
};
use tokio::{
    runtime::Runtime,
    sync::{broadcast, Notify},
};

lazy_static! {
    // Since ALVR_DIR is needed to initialize logging, if error then just panic
    static ref ALVR_DIR: PathBuf = commands::get_alvr_dir().unwrap();
    static ref SESSION_MANAGER: Mutex<SessionManager> = Mutex::new(SessionManager::new(&ALVR_DIR));
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(Runtime::new().ok());
    static ref CLIENTS_UPDATED_NOTIFIER: Notify = Notify::new();
    static ref RESTART_NOTIFIER: Notify = Notify::new();
    static ref SHUTDOWN_NOTIFIER: Notify = Notify::new();
    static ref MAYBE_WINDOW: Mutex<Option<Arc<alcro::UI>>> = Mutex::new(None);
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

pub fn restart_steamvr() {
    thread::spawn(|| {
        RESTART_NOTIFIER.notify_waiters();

        // give time to the control loop to send the restart packet (not crucial)
        thread::sleep(Duration::from_millis(100));

        shutdown_runtime();

        unsafe { ShutdownSteamvr() };

        commands::restart_steamvr_with_timeout(&ALVR_DIR).ok();
    });
}

pub enum ClientListAction {
    AddIfMissing {
        device_name: String,
        ip: IpAddr,
        certificate_pem: Option<String>,
    },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub async fn update_client_list(hostname: String, action: ClientListAction) {
    let mut client_connections = SESSION_MANAGER.lock().get().client_connections.clone();

    let maybe_client_entry = client_connections.entry(hostname);

    let mut updated = false;
    match action {
        ClientListAction::AddIfMissing {
            device_name,
            ip,
            certificate_pem,
        } => match maybe_client_entry {
            Entry::Occupied(mut existing_entry) => {
                let client_connection_ref = existing_entry.get_mut();

                if client_connection_ref.last_local_ip != ip {
                    client_connection_ref.last_local_ip = ip;

                    updated = true;
                }
            }
            Entry::Vacant(new_entry) => {
                let client_connection_desc = ClientConnectionDesc {
                    trusted: false,
                    last_local_ip: ip,
                    manual_ips: HashSet::new(),
                    device_name,
                    certificate_pem,
                };
                new_entry.insert(client_connection_desc);

                updated = true;
            }
        },
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
        SESSION_MANAGER
            .lock()
            .get_mut(None, SessionUpdateType::ClientList)
            .client_connections = client_connections;

        CLIENTS_UPDATED_NOTIFIER.notify_waiters();
    }
}

// this thread gets interrupted when SteamVR closes
// todo: handle this in a better way
fn ui_thread() -> StrResult {
    let window = Arc::new(trace_err!(alcro::UIBuilder::new()
        .content(alcro::Content::Url("http://127.0.0.1:8082"))
        .size(800, 600)
        .custom_args(&["--disk-cache-size=1"])
        .run())?);

    *MAYBE_WINDOW.lock() = Some(window.clone());

    window.wait_finish();

    // prevent panic on window.close()
    *MAYBE_WINDOW.lock() = None;
    shutdown_runtime();
    unsafe { ShutdownSteamvr() };

    Ok(())
}

fn init(log_sender: broadcast::Sender<String>) -> StrResult {
    if let Some(runtime) = MAYBE_RUNTIME.lock().as_mut() {
        // Acquire and drop the session_manager lock to create session.json if not present
        // this is needed until Settings.cpp is replaced with Rust. todo: remove
        SESSION_MANAGER
            .lock()
            .get_mut(None, SessionUpdateType::Other);

        runtime.spawn(async move {
            let connections = SESSION_MANAGER.lock().get().client_connections.clone();
            for (hostname, connection) in connections {
                if !connection.trusted {
                    update_client_list(hostname, ClientListAction::RemoveIpOrEntry(None)).await;
                }
            }

            let web_server = show_err_async(web_server::web_server(log_sender));

            tokio::select! {
                _ = web_server => (),
                _ = SHUTDOWN_NOTIFIER.notified() => (),
            }
        });

        thread::spawn(|| show_err(ui_thread()));
    }

    let alvr_dir_c_string = CString::new(ALVR_DIR.to_string_lossy().to_string()).unwrap();
    unsafe { g_alvrDir = alvr_dir_c_string.into_raw() };

    // ALVR_DIR has been used (and so initialized). I don't need alvr_dir storage on disk anymore
    commands::maybe_delete_alvr_dir_storage();

    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(|| {
        let (log_sender, _) = broadcast::channel(web_server::LOG_BROADCAST_CAPACITY);
        logging_backend::init_logging(log_sender.clone());

        show_err(init(log_sender)).ok();
    });

    lazy_static_include_bytes!(FRAME_RENDER_VS_CSO => "cpp/alvr_server/FrameRenderVS.cso");
    lazy_static_include_bytes!(FRAME_RENDER_PS_CSO => "cpp/alvr_server/FrameRenderPS.cso");
    lazy_static_include_bytes!(QUAD_SHADER_CSO => "cpp/alvr_server/QuadVertexShader.cso");
    lazy_static_include_bytes!(COMPRESS_SLICES_CSO =>
        "cpp/alvr_server/CompressSlicesPixelShader.cso");
    lazy_static_include_bytes!(COLOR_CORRECTION_CSO =>
        "cpp/alvr_server/ColorCorrectionPixelShader.cso");

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
        show_e(CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe fn log(level: log::Level, string_ptr: *const c_char) {
        _log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
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

    unsafe extern "C" fn driver_ready_idle() {
        show_err(commands::apply_driver_paths_backup(ALVR_DIR.clone())).ok();

        if let Some(runtime) = &mut *MAYBE_RUNTIME.lock() {
            runtime.spawn(async move {
                // call this when inside a new tokio thread. Calling this on the parent thread will
                // crash SteamVR
                SetDefaultChaperone();

                tokio::select! {
                    Err(e) = connection::connection_lifecycle_loop() => show_e(e),
                    _ = SHUTDOWN_NOTIFIER.notified() => (),
                    else => (),
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
    ShutdownRuntime = Some(_shutdown_runtime);

    // cast to usize to allow the variables to cross thread boundaries
    let interface_name_usize = interface_name as usize;
    let return_code_usize = return_code as usize;

    lazy_static::lazy_static! {
        static ref maybe_ptr_usize: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        static ref num_trials: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    }

    thread::spawn(move || {
        num_trials.fetch_add(1, Ordering::Relaxed);
        if num_trials.load(Ordering::Relaxed) <= 1 {
            maybe_ptr_usize.store(
                CppEntryPoint(interface_name_usize as _, return_code_usize as _) as _,
                Ordering::Relaxed,
            );
        }
    })
    .join()
    .ok();

    maybe_ptr_usize.load(Ordering::Relaxed) as _
}
