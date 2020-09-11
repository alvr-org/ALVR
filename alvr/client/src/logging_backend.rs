use crate::*;
use alvr_common::logging::*;

pub fn init_logging() {
    #[cfg(target_os = "android")]
    android_logger::init_once(android_logger::Config::default());

    crate::logging::set_panic_hook();
}
