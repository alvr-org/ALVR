mod body_tracking_bd;
mod body_tracking_fb;
mod eye_gaze_interaction;
mod eye_tracking_social;
mod face_tracking2_fb;
mod face_tracking_pico;
mod facial_tracking_htc;
mod multimodal_input;
mod passthrough_fb;
mod passthrough_htc;

pub use body_tracking_bd::*;
pub use body_tracking_fb::*;
pub use eye_gaze_interaction::*;
pub use eye_tracking_social::*;
pub use face_tracking2_fb::*;
pub use face_tracking_pico::*;
pub use facial_tracking_htc::*;
pub use multimodal_input::*;
pub use passthrough_fb::*;
pub use passthrough_htc::*;
use std::ffi::CString;
use std::mem;

use openxr::{self as xr, sys};

fn xr_res(result: sys::Result) -> xr::Result<()> {
    if result.into_raw() >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}

fn get_props<G, T>(
    session: &xr::Session<G>,
    system: xr::SystemId,
    default_struct: T,
) -> xr::Result<T> {
    let instance = session.instance();

    let mut props = default_struct;
    let mut system_properties = sys::SystemProperties::out((&mut props as *mut T).cast());
    let result = unsafe {
        (instance.fp().get_system_properties)(
            instance.as_raw(),
            system,
            system_properties.as_mut_ptr(),
        )
    };

    xr_res(result).map(|_| props)
}

fn get_instance_proc<G, FnTy>(session: &xr::Session<G>, method_name: &str) -> xr::Result<FnTy> {
    unsafe {
        let method_name = CString::new(method_name).unwrap();
        let mut function_handle = None;

        xr_res((session.instance().fp().get_instance_proc_addr)(
            session.instance().as_raw(),
            method_name.as_ptr(),
            &mut function_handle,
        ))?;

        function_handle
            .map(|pfn| mem::transmute_copy(&pfn))
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)
    }
}
