#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types, non_upper_case_globals)]

mod logging_backend;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::{process::*, *};
use data::SessionManager;
use lazy_static::lazy_static;
use lazy_static_include::*;
use logging::show_err;
use sockets::StreamManager;
use std::{
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
    sync::{
        atomic::{AtomicBool, Ordering},
        Once,
    },
    thread,
};

lazy_static! {
    static ref running: AtomicBool = AtomicBool::new(true);
}

async fn connection_loop() -> StrResult {
    let session_manager = SessionManager::new(&get_alvr_dir()?);
    let session = session_manager.get();

    let ip = trace_none!(
        session.connected_client_ip,
        "No client found. Please trust a client first with ALVR.exe."
    )?;
    let settings = session.to_settings();

    let stream_manager = StreamManager::new(
        ip,
        settings.connection.stream_port,
        settings.connection.stream_socket_config,
    ).await?;

    // todo: receive loop

    Ok(())
}

fn begin_client_connection() -> StrResult {
    let runtime_loop = || {
        let mut runtime = trace_err!(tokio::runtime::Runtime::new())?;
        runtime.block_on(connection_loop())
    };

    trace_err!(thread::Builder::new()
        .name("Connection loop".into())
        .spawn(move || show_err(runtime_loop()).ok()))?;

    Ok(())
}

pub fn init() -> StrResult {
    let alvr_dir = get_alvr_dir()?;
    process::maybe_launch_web_server(&alvr_dir);

    begin_client_connection()?;

    let alvr_dir_c_string = CString::new(alvr_dir.to_string_lossy().to_string()).unwrap();
    unsafe { g_alvrDir = alvr_dir_c_string.into_raw() };

    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    logging_backend::init_logging();

    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(|| {
        show_err(init()).ok();
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

    extern "C" fn maybe_kill_web_server() {
        process::maybe_kill_web_server();
    }

    unsafe extern "C" fn set_running(value: bool) {
        running.store(value, Ordering::Relaxed)
    }

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    MaybeKillWebServer = Some(maybe_kill_web_server);
    SetRunning = Some(set_running);

    CppEntryPoint(interface_name, return_code)
}
