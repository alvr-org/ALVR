#![cfg(target_os = "linux")]
#![allow(clippy::missing_safety_doc)]

use std::ffi::CString;

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::useless_transmute
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/layer_bindings.rs"));
}
use bindings::*;

#[no_mangle]
pub unsafe extern "C" fn ALVR_Negotiate(nli: *mut VkNegotiateLayerInterface) -> VkResult {
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
