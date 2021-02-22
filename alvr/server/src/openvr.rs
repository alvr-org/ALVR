use openvr_driver_sys::*;

pub fn test1() {
    unsafe { vrCleanupDriverContext() };
}

pub fn test2() {
    tracked_device_property_name_to_u32("test").ok();
}
