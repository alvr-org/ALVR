mod logging_backend;

use alvr_common::{data::*, logging::*, *};
use alvr_xtask::*;
use lazy_static::lazy_static;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
};

#[no_mangle]
pub extern "C" fn init_logging() {
    logging_backend::init_logging();
}

// If settings cannot be loaded, this method shows an error and returns null.
#[no_mangle]
pub extern "C" fn settings() -> *const Settings {
    lazy_static! {
        static ref MAYBE_SETTINGS: StrResult<Settings> = get_alvr_dir_using_vrpathreg()
            .map_err(|e| e.to_string())
            .and_then(|alvr_dir| load_json(&alvr_dir.join(SETTINGS_FNAME)))
            .map_err(|e| {
                error!("{}", e);
                e
            });
    }

    if let Ok(settings) = &*MAYBE_SETTINGS {
        settings as _
    } else {
        ptr::null()
    }
}

#[no_mangle]
pub extern "C" fn maybe_launch_web_server() {
    match get_alvr_dir_using_vrpathreg() {
        Ok(alvr_dir) => process::maybe_launch_web_server(&alvr_dir),
        Err(e) => log::error!("{}", e),
    }
}

#[no_mangle]
pub extern "C" fn maybe_kill_web_server() {
    process::maybe_kill_web_server();
}

/// # Safety
/// This function is safe
#[no_mangle]
pub unsafe extern "C" fn get_connected_client_packet(
    address_buf: *mut c_char,
    address_len: usize,
) -> *const ClientHandshakePacket {
    lazy_static! {
        static ref MAYBE_CLIENT_CONNECTION_DESC: StrResult<ClientConnectionDesc> =
            get_alvr_dir_using_vrpathreg()
                .map_err(|e| e.to_string())
                .and_then(|alvr_dir| load_json(&alvr_dir.join(SESSION_FNAME)))
                .and_then(|session_desc: SessionDesc| {
                    for client_connection in session_desc.last_clients {
                        if client_connection.available {
                            return Ok(client_connection);
                        }
                    }
                    Err("No client connected".into())
                })
                .map_err(|e| {
                    error!("{}", e);
                    e
                });
    }

    if let Ok(client_connection_desc) = &*MAYBE_CLIENT_CONNECTION_DESC {
        let address_cstring = CString::new(client_connection_desc.address.to_owned()).unwrap();
        ptr::copy_nonoverlapping(address_cstring.as_ptr(), address_buf, address_len);
        &client_connection_desc.handshake_packet as _
    } else {
        ptr::null()
    }
}

unsafe fn log(level: log::Level, string_ptr: *const c_char) {
    _log!(level, "{}", CStr::from_ptr(string_ptr).to_string_lossy());
}

unsafe fn log_id(level: log::Level, id: LogId, string_ptr: *const c_char) {
    if !string_ptr.is_null() {
        _log!(
            level,
            id: id,
            "{}",
            CStr::from_ptr(string_ptr).to_string_lossy()
        );
    } else {
        _log!(level, id: id);
    }
}

/// # Safety
/// string_ptr must be non null and null terminated
#[no_mangle]
pub unsafe extern "C" fn error(string_ptr: *const c_char) {
    log(log::Level::Error, string_ptr);
}

/// # Safety
/// string_ptr can be null
#[no_mangle]
pub unsafe extern "C" fn error_id(id: LogId, string_ptr: *const c_char) {
    log_id(log::Level::Error, id, string_ptr);
}

/// # Safety
/// string_ptr must be non null and null terminated
#[no_mangle]
pub unsafe extern "C" fn warn(string_ptr: *const c_char) {
    log(log::Level::Warn, string_ptr);
}

/// # Safety
/// string_ptr can be null
#[no_mangle]
pub unsafe extern "C" fn warn_id(id: LogId, string_ptr: *const c_char) {
    log_id(log::Level::Warn, id, string_ptr);
}

/// # Safety
/// string_ptr must be non null and null terminated
#[no_mangle]
pub unsafe extern "C" fn info(string_ptr: *const c_char) {
    log(log::Level::Info, string_ptr);
}

/// # Safety
/// string_ptr can be null
#[no_mangle]
pub unsafe extern "C" fn info_id(id: LogId, string_ptr: *const c_char) {
    log_id(log::Level::Info, id, string_ptr);
}

/// # Safety
/// string_ptr must be non null and null terminated
#[no_mangle]
pub unsafe extern "C" fn debug(string_ptr: *const c_char) {
    log(log::Level::Debug, string_ptr);
}

/// # Safety
/// string_ptr can be null
#[no_mangle]
pub unsafe extern "C" fn debug_id(id: LogId, string_ptr: *const c_char) {
    log_id(log::Level::Debug, id, string_ptr);
}
