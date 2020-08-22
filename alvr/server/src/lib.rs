#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

mod connection;
mod logging_backend;
mod statistics;
mod web_server;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{data::*, logging::*, process::*, *};
use lazy_static::lazy_static;
use lazy_static_include::*;
use parking_lot::Mutex;
use std::{
    collections::{hash_map::Entry, HashSet},
    ffi::{c_void, CStr, CString},
    net::IpAddr,
    os::raw::c_char,
    sync::{Arc, Once},
    thread,
    time::SystemTime,
};
use tokio::{runtime::Runtime, sync::broadcast};

pub type AMutex<T> = tokio::sync::Mutex<T>;

lazy_static! {
    static ref MAYBE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(Runtime::new().ok());
    static ref MAYBE_SHUTDOWN_NOTIFIER: Mutex<Option<broadcast::Sender<()>>> = Mutex::new(None);
}

pub fn shutdown_runtime() {
    if let Some(notifier) = &*MAYBE_SHUTDOWN_NOTIFIER.lock() {
        notifier.send(()).ok();
    }

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
        shutdown_runtime();

        unsafe { ShutdownSteamvr() };

        // todo: launch alvr_launcher with "restart" flag
    });
}

pub enum ClientListAction {
    AddIfMissing { ip: IpAddr, certificate_pem: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub async fn update_client_list(
    session_manager: Arc<AMutex<SessionManager>>,
    hostname: String,
    action: ClientListAction,
    update_client_listeners_notifier: broadcast::Sender<()>,
) {
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let session_manager_ref = &mut session_manager.lock().await;
    let session_desc_ref =
        &mut session_manager_ref.get_mut(SERVER_SESSION_UPDATE_ID, SessionUpdateType::ClientList);

    let maybe_client_entry = session_desc_ref.last_clients.entry(hostname);

    match action {
        ClientListAction::AddIfMissing {
            ip,
            certificate_pem,
        } => match maybe_client_entry {
            Entry::Occupied(mut existing_entry) => {
                let client_connection_ref = existing_entry.get_mut();
                client_connection_ref.last_update_ms_since_epoch = now_ms as _;
                client_connection_ref.last_ip = ip;
            }
            Entry::Vacant(new_entry) => {
                let client_connection_desc = ClientConnectionDesc {
                    trusted: false,
                    last_update_ms_since_epoch: now_ms as _,
                    last_ip: ip,
                    manual_ips: HashSet::new(),
                    display_name: None,
                    certificate_pem,
                };
                new_entry.insert(client_connection_desc);
            }
        },
        ClientListAction::TrustAndMaybeAddIp(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                let client_connection_ref = entry.get_mut();
                client_connection_ref.trusted = true;
                if let Some(ip) = maybe_ip {
                    client_connection_ref.manual_ips.insert(ip);
                }
            }
            // else: never happens. The UI cannot request a new entry creation because in that case
            // it wouldn't have the certificate
        }
        ClientListAction::RemoveIpOrEntry(maybe_ip) => {
            if let Entry::Occupied(mut entry) = maybe_client_entry {
                if let Some(ip) = maybe_ip {
                    entry.get_mut().manual_ips.remove(&ip);
                } else {
                    entry.remove_entry();
                }
            }
        }
    }

    if let Err(e) = update_client_listeners_notifier.send(()) {
        warn!("Failed to notify client list update: {:?}", e);
    }
}

fn init(log_sender: broadcast::Sender<String>) -> StrResult {
    let alvr_dir = get_alvr_dir()?;

    if let Err(e) = restore_driver_paths_backup() {
        info!(
            "Failed to restore drivers paths backup (usually not an error): {}",
            e
        );
        // This is not fatal. This happens if the user did register ALVR driver through the setup
        // wizard.
    }

    if let Some(runtime) = &*MAYBE_RUNTIME.lock() {
        let session_manager = Arc::new(AMutex::new(SessionManager::new(&alvr_dir)));

        let (shutdown_notifier, mut shutdown_receiver) = broadcast::channel(1);
        let (update_client_listeners_notifier, _) = broadcast::channel(1);

        // Error: reached the type-length limit while instantiating ...
        // I need to split my future into separate .spawn()

        runtime.spawn({
            async move {
                let web_server = show_err_async(web_server::web_server(
                    session_manager.clone(),
                    log_sender,
                    update_client_listeners_notifier.clone(),
                ));

                let connection_loop = show_err_async(connection::connection_loop(
                    session_manager,
                    update_client_listeners_notifier,
                ));

                tokio::select! {
                    _ = web_server => (),
                    _ = connection_loop => (),
                    _ = shutdown_receiver.recv() => (),
                }
            }
        });

        *MAYBE_SHUTDOWN_NOTIFIER.lock() = Some(shutdown_notifier);
    }

    let alvr_dir_c_string = CString::new(alvr_dir.to_string_lossy().to_string()).unwrap();
    unsafe { g_alvrDir = alvr_dir_c_string.into_raw() };

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
    lazy_static_include_bytes!(QUAD_SHADER_CSO  => "cpp/alvr_server/QuadVertexShader.cso");
    lazy_static_include_bytes!(COMPRESS_SLICES_CSO => "cpp/alvr_server/CompressSlicesPixelShader.cso");
    lazy_static_include_bytes!(COLOR_CORRECTION_CSO => "cpp/alvr_server/ColorCorrectionPixelShader.cso");

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

    unsafe fn log(level: log::Level, string_ptr: *const c_char) {
        _log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
    }

    unsafe extern "C" fn log_error(string_ptr: *const c_char) {
        log(log::Level::Error, string_ptr);
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

    extern "C" fn _shutdown_runtime() {
        shutdown_runtime();
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    ShutdownRuntime = Some(_shutdown_runtime);

    CppEntryPoint(interface_name, return_code)
}
