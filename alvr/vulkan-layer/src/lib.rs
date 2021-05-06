#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/layer_bindings.rs"));
}
use bindings::*;

use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn vkGetInstanceProcAddr(
    instance: VkInstance,
    p_name: *const c_char,
) -> PFN_vkVoidFunction {
    bindings::wsi_layer_vkGetInstanceProcAddr(instance, p_name)
}

#[no_mangle]
pub unsafe extern "C" fn vkGetDeviceProcAddr(
    instance: VkDevice,
    p_name: *const c_char,
) -> PFN_vkVoidFunction {
    bindings::wsi_layer_vkGetDeviceProcAddr(instance, p_name)
}
