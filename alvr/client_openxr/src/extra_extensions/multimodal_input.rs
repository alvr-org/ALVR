// Code taken from:
// https://github.com/meta-quest/Meta-OpenXR-SDK/blob/main/OpenXR/meta_openxr_preview/meta_simultaneous_hands_and_controllers.h

use alvr_common::{anyhow::Result, once_cell::sync::Lazy, ToAny};
use openxr::{self as xr, sys};
use std::{ffi::c_void, mem, ptr};

pub const META_SIMULTANEOUS_HANDS_AND_CONTROLLERS_EXTENSION_NAME: &str =
    "XR_META_simultaneous_hands_and_controllers";
pub const META_DETACHED_CONTROLLERS_EXTENSION_NAME: &str = "XR_META_detached_controllers";

static TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532002));

#[repr(C)]
pub struct SimultaneousHandsAndControllersTrackingResumeInfoMETA {
    ty: xr::StructureType,
    next: *const c_void,
}

pub type ResumeSimultaneousHandsAndControllersTrackingMETA =
    unsafe extern "system" fn(
        sys::Session,
        *const SimultaneousHandsAndControllersTrackingResumeInfoMETA,
    ) -> sys::Result;

pub fn resume_simultaneous_hands_and_controllers_tracking<G>(
    session: &xr::Session<G>,
) -> Result<()> {
    let resume_simultaneous_hands_and_controllers_tracking_meta = unsafe {
        let mut resume_simultaneous_hands_and_controllers_tracking_meta = None;
        let _ = (session.instance().fp().get_instance_proc_addr)(
            session.instance().as_raw(),
            c"xrResumeSimultaneousHandsAndControllersTrackingMETA".as_ptr(),
            &mut resume_simultaneous_hands_and_controllers_tracking_meta,
        );

        mem::transmute::<_, ResumeSimultaneousHandsAndControllersTrackingMETA>(
            resume_simultaneous_hands_and_controllers_tracking_meta.to_any()?,
        )
    };

    let resume_info = SimultaneousHandsAndControllersTrackingResumeInfoMETA {
        ty: *TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META,
        next: ptr::null(),
    };

    unsafe {
        super::xr_to_any(resume_simultaneous_hands_and_controllers_tracking_meta(
            session.as_raw(),
            &resume_info,
        ))?;
    }

    Ok(())
}
