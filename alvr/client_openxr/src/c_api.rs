#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn alvr_entry_point(java_vm: *mut std::ffi::c_void, context: *mut std::ffi::c_void) {
    unsafe { ndk_context::initialize_android_context(java_vm, context) };

    crate::entry_point();
}
