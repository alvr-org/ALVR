#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types, non_upper_case_globals)]

mod connection;
mod logging_backend;
mod web_server;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{data::*, logging::*, *};
use lazy_static::lazy_static;
use lazy_static_include::*;
use parking_lot::Mutex;
use std::{
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
    path::PathBuf,
    sync::Once,
    sync::{atomic::AtomicUsize, atomic::Ordering, Arc},
    thread,
};
use tokio::{runtime::Runtime, sync::broadcast};

lazy_static! {
    // Since ALVR_DIR is needed to initialize logging, if error then just panic
    static ref ALVR_DIR: PathBuf = commands::get_alvr_dir().unwrap();
    static ref SESSION_MANAGER: Mutex<SessionManager> = Mutex::new(SessionManager::new(&ALVR_DIR));
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

fn init(log_sender: broadcast::Sender<String>) -> StrResult {
    if let Some(runtime) = MAYBE_RUNTIME.lock().as_mut() {
        // Acquire and drop the session_manager lock to create session.json if not present
        // this is needed until Settings.cpp is replaced with Rust. todo: remove
        SESSION_MANAGER
            .lock()
            .get_mut(None, SessionUpdateType::Other);

        let (shutdown_notifier, mut shutdown_receiver) = broadcast::channel(1);

        runtime.spawn(async move {
            let web_server = show_err_async(web_server::web_server(log_sender));

            tokio::select! {
                _ = web_server => (),
                _ = shutdown_receiver.recv() => (),
            }
        });

        *MAYBE_SHUTDOWN_NOTIFIER.lock() = Some(shutdown_notifier);
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

        if let (Some(runtime), Some(shutdown_notifier)) = (
            MAYBE_RUNTIME.lock().as_mut(),
            MAYBE_SHUTDOWN_NOTIFIER.lock().as_mut(),
        ) {
            let mut shutdown_receiver = shutdown_notifier.subscribe();
            runtime.spawn(async move {
                tokio::select! {
                    _ = connection::client_discovery() => (),
                    _ = shutdown_receiver.recv() => (),
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
