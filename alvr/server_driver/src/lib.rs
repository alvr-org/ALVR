#![allow(clippy::missing_safety_doc)]

mod logging_backend;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use alvr_common::*;
use alvr_xtask::*;
use std::{
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
};

#[no_mangle]
pub extern "C" fn maybe_kill_web_server() {
    process::maybe_kill_web_server();
}

unsafe fn log(level: log::Level, string_ptr: *const c_char) {
    _log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
}

#[no_mangle]
pub unsafe extern "C" fn log_error(string_ptr: *const c_char) {
    log(log::Level::Error, string_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn log_warn(string_ptr: *const c_char) {
    log(log::Level::Warn, string_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn log_info(string_ptr: *const c_char) {
    log(log::Level::Info, string_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn log_debug(string_ptr: *const c_char) {
    log(log::Level::Debug, string_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    logging_backend::init_logging();
    
    // launch web server
    match get_alvr_dir_using_vrpathreg() {
        Ok(alvr_dir) => process::maybe_launch_web_server(&alvr_dir),
        Err(e) => log::error!("{}", e),
    }

    let alvr_dir_c_string = CString::new(
        get_alvr_dir_using_vrpathreg()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    )
    .unwrap();
    g_alvrDir = alvr_dir_c_string.into_raw();

    LogError = Some(log_error);
    LogWarn = Some(log_warn);
    LogInfo = Some(log_info);
    LogDebug = Some(log_debug);
    MaybeKillWebServer = Some(maybe_kill_web_server);

    CppEntryPoint(interface_name, return_code)
}
