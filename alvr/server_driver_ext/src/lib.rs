pub mod logging_backend;

use alvr_common::{data::*, *};
use alvr_xtask::*;
use lazy_static::lazy_static;
use std::ptr;

#[no_mangle]
pub extern "C" fn init_logging() {}

// If settings cannot be loaded, this method shows an error and returns null.
#[no_mangle]
pub extern "C" fn settings() -> *const Settings {
    lazy_static! {
        static ref MAYBE_SETTINGS: StrResult<Settings> = {
            match get_alvr_dir_using_vrpathreg() {
                Ok(alvr_dir) => {
                    load_settings(&alvr_dir.join("settings.json"))
                }
                Err(e) => Err(e.to_string())
            }
        };
    }

    show_err!(MAYBE_SETTINGS.as_ref()).ok();

    if let Ok(settings) = &*MAYBE_SETTINGS {
        settings as _
    } else {
        ptr::null()
    }
}
