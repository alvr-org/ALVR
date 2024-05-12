mod props;
use alvr_common::{once_cell::sync::Lazy, parking_lot::Mutex, OptLazy};
pub use props::*;

use crate::{logging_backend, ServerCoreContext, ServerCoreEvent};
use std::{
    ffi::{c_char, c_void},
    thread,
    time::Duration,
};

static SERVER_CORE_CONTEXT: OptLazy<ServerCoreContext> = Lazy::new(|| {
    logging_backend::init_logging();

    Mutex::new(Some(ServerCoreContext::new()))
});

pub extern "C" fn driver_ready_idle(set_default_chap: bool) {
    thread::spawn(move || {
        if set_default_chap {
            // call this when inside a new thread. Calling this on the parent thread will crash
            // SteamVR
            unsafe {
                crate::InitOpenvrClient();
                crate::SetChaperoneArea(2.0, 2.0);
                crate::ShutdownOpenvrClient();
            }
        }

        loop {
            let event = if let Some(context) = &*SERVER_CORE_CONTEXT.lock() {
                match context.poll_event() {
                    Some(event) => event,
                    None => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                }
            } else {
                break;
            };

            match event {
                ServerCoreEvent::ShutdownPending => {
                    SERVER_CORE_CONTEXT.lock().take();

                    unsafe { crate::ShutdownSteamvr() };
                }
                ServerCoreEvent::RestartPending => {
                    if let Some(context) = SERVER_CORE_CONTEXT.lock().take() {
                        context.restart();
                    }

                    unsafe { crate::ShutdownSteamvr() };
                }
            }
        }
    });

    if let Some(context) = &*SERVER_CORE_CONTEXT.lock() {
        context.start_connection();
    }
}

pub extern "C" fn shutdown_driver() {
    SERVER_CORE_CONTEXT.lock().take();
}

/// This is the SteamVR/OpenVR entry point
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    // Make sure the context is initialized, and initialize logging
    SERVER_CORE_CONTEXT.lock().as_ref();

    crate::DriverReadyIdle = Some(driver_ready_idle);
    crate::ShutdownRuntime = Some(shutdown_driver);

    crate::CppOpenvrEntryPoint(interface_name, return_code)
}
