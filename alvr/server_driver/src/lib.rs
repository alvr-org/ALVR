#![allow(clippy::missing_safety_doc)]

mod logging_backend;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::*;
use alvr_xtask::*;
use lazy_static::lazy_static;
use lazy_static_include::*;
use std::{
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
};

lazy_static_include_bytes!(FRAME_RENDER_VS_CSO, "cpp/alvr_server/FrameRenderVS.cso");
lazy_static_include_bytes!(FRAME_RENDER_PS_CSO, "cpp/alvr_server/FrameRenderPS.cso");
lazy_static_include_bytes!(QUAD_SHADER_CSO, "cpp/alvr_server/QuadVertexShader.cso");
lazy_static_include_bytes!(
    COMPRESS_SLICES_CSO,
    "cpp/alvr_server/CompressSlicesPixelShader.cso"
);
lazy_static_include_bytes!(
    COLOR_CORRECTION_CSO,
    "cpp/alvr_server/ColorCorrectionPixelShader.cso"
);

extern "C" fn maybe_kill_web_server() {
    process::maybe_kill_web_server();
}

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

#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    logging_backend::init_logging();

    match get_alvr_dir() {
        Ok(alvr_dir) => {
            // launch web server
            process::maybe_launch_web_server(&alvr_dir);

            let alvr_dir_c_string = CString::new(alvr_dir.to_string_lossy().to_string()).unwrap();
            g_alvrDir = alvr_dir_c_string.into_raw();
        }
        Err(e) => log::error!("{}", e),
    }

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

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    MaybeKillWebServer = Some(maybe_kill_web_server);

    CppEntryPoint(interface_name, return_code)
}
