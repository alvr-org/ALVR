mod props;
pub use props::*;

use crate::{logging_backend, ServerCoreContext};
use std::{
    ffi::{c_char, c_void},
    sync::OnceLock,
    thread,
};

static SERVER_CORE_CONTEXT: OnceLock<ServerCoreContext> = OnceLock::new();

pub extern "C" fn driver_ready_idle(set_default_chap: bool) {
    if set_default_chap {
        thread::spawn(move || {
            // call this when inside a new thread. Calling this on the parent thread will crash
            // SteamVR
            unsafe {
                crate::InitOpenvrClient();
                crate::SetChaperoneArea(2.0, 2.0);
                crate::ShutdownOpenvrClient();
            }
        });
    }

    SERVER_CORE_CONTEXT.get().unwrap().start_connection();
}

/// This is the SteamVR/OpenVR entry point
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    SERVER_CORE_CONTEXT.get_or_init(|| {
        logging_backend::init_logging();

        ServerCoreContext::new()
    });

    crate::DriverReadyIdle = Some(driver_ready_idle);

    crate::CppOpenvrEntryPoint(interface_name, return_code)
}
