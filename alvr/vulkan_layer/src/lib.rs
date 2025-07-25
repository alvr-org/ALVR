#![cfg(target_os = "linux")]
#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unsafe_op_in_unsafe_fn,
    unused_imports,
    clippy::missing_safety_doc,
    clippy::ptr_offset_with_cast,
    clippy::too_many_arguments,
    clippy::useless_transmute,
    clippy::pedantic,
    clippy::nursery
)]

use std::ffi::CString;

mod bindings {
    include!(concat!(env!("OUT_DIR"), "/layer_bindings.rs"));
}
use bindings::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ALVR_Negotiate(nli: *mut VkNegotiateLayerInterface) -> VkResult {
    unsafe {
        g_sessionPath = CString::new(
            alvr_filesystem::filesystem_layout_invalid()
                .session()
                .to_string_lossy()
                .to_string(),
        )
        .unwrap()
        .into_raw();

        bindings::wsi_layer_Negotiate(nli)
    }
}
