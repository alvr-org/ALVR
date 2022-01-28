use std::{ffi::c_void, os::raw::c_char};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Entry point. The entry point must live on the Rust side, since C symbols are not exported
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    root::entry_point(interface_name, return_code)
}
