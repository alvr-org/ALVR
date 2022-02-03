use alvr_server::capi::*;
use std::{ptr, thread, time::Duration};

fn main() {
    let graphics_handles = AlvrGraphicsContext {
        vk_get_device_proc_addr: ptr::null_mut(),
        vk_instance: 0,
        vk_physical_device: 0,
        vk_device: 0,
        vk_queue_family_index: 0,
        vk_queue_index: 0,
    };

    if unsafe { !alvr_initialize(graphics_handles) } {
        return;
    }

    thread::sleep(Duration::from_secs(10000));

    alvr_shutdown();
}
