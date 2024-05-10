mod props;

pub use props::*;

use std::{
    ffi::{c_char, c_void},
    sync::Once,
};

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    static INIT_ONCE: Once = Once::new();
    INIT_ONCE.call_once(crate::init);

    crate::CppOpenvrEntryPoint(interface_name, return_code)
}
