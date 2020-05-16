mod logging_backend;

use alvr_common::{data::*, *};
use alvr_xtask::*;
use lazy_static::lazy_static;
use std::ptr;

#[no_mangle]
pub extern "C" fn init_logging() {
    logging_backend::init_logging();
}

// If settings cannot be loaded, this method shows an error and returns null.
#[no_mangle]
pub extern "C" fn settings() -> *const Settings {
    lazy_static! {
        static ref MAYBE_SETTINGS: StrResult<Settings> = {
            let maybe_settings = match get_alvr_dir_using_vrpathreg() {
                Ok(alvr_dir) => load_json(&alvr_dir.join(SETTINGS_FNAME)),
                Err(e) => Err(e.to_string()),
            };
            if let Err(e) = &maybe_settings {
                log::error!("{}", e);
            }
            maybe_settings
        };
    }

    if let Ok(settings) = &*MAYBE_SETTINGS {
        settings as _
    } else {
        ptr::null()
    }
}

#[no_mangle]
pub extern "C" fn maybe_launch_web_server() {
    match get_alvr_dir_using_vrpathreg() {
        Ok(alvr_dir) => process::maybe_launch_web_server(&alvr_dir),
        Err(e) => log::error!("{}", e),
    }
}

#[no_mangle]
pub extern "C" fn maybe_kill_web_server() {
    process::maybe_kill_web_server();
}
