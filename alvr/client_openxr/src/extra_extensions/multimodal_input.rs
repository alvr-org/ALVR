// Code taken from:
// https://github.com/meta-quest/Meta-OpenXR-SDK/blob/main/OpenXR/meta_openxr_preview/meta_simultaneous_hands_and_controllers.h

use alvr_common::once_cell::sync::Lazy;
use openxr::{
    self as xr,
    sys::{self, pfn::VoidFunction},
};
use std::{ffi::c_void, mem, ptr};

pub const META_SIMULTANEOUS_HANDS_AND_CONTROLLERS_EXTENSION_NAME: &str =
    "XR_META_simultaneous_hands_and_controllers";
pub const META_DETACHED_CONTROLLERS_EXTENSION_NAME: &str = "XR_META_detached_controllers";

static TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532002));
static TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_PAUSE_INFO_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532003));

#[repr(C)]
pub struct SimultaneousHandsAndControllersTrackingResumeInfoMETA {
    ty: xr::StructureType,
    next: *const c_void,
}
#[repr(C)]
pub struct SimultaneousHandsAndControllersTrackingPauseInfoMETA {
    ty: xr::StructureType,
    next: *const c_void,
}

pub type ResumeSimultaneousHandsAndControllersTrackingMETA =
    unsafe extern "system" fn(
        sys::Session,
        *const SimultaneousHandsAndControllersTrackingResumeInfoMETA,
    ) -> sys::Result;
pub type PauseSimultaneousHandsAndControllersTrackingMETA =
    unsafe extern "system" fn(
        sys::Session,
        *const SimultaneousHandsAndControllersTrackingPauseInfoMETA,
    ) -> sys::Result;

pub fn resume_simultaneous_hands_and_controllers_tracking_meta<G>(
    session: &xr::Session<G>,
) -> xr::Result<()> {
    let resume_simultaneous_hands_and_controllers_tracking_meta = unsafe {
        let mut resume_simultaneous_hands_and_controllers_tracking_meta = None;
        let _ = (session.instance().fp().get_instance_proc_addr)(
            session.instance().as_raw(),
            c"xrResumeSimultaneousHandsAndControllersTrackingMETA".as_ptr(),
            &mut resume_simultaneous_hands_and_controllers_tracking_meta,
        );

        resume_simultaneous_hands_and_controllers_tracking_meta.map(|pfn| {
            mem::transmute::<VoidFunction, ResumeSimultaneousHandsAndControllersTrackingMETA>(pfn)
        })
    }
    .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

    let resume_info = SimultaneousHandsAndControllersTrackingResumeInfoMETA {
        ty: *TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META,
        next: ptr::null(),
    };
    unsafe {
        super::xr_res(resume_simultaneous_hands_and_controllers_tracking_meta(
            session.as_raw(),
            &resume_info,
        ))
    }
}

pub fn pause_simultaneous_hands_and_controllers_tracking_meta<G>(
    session: &xr::Session<G>,
) -> xr::Result<()> {
    let pause_simultaneous_hands_and_controllers_tracking_meta = unsafe {
        let mut pause_simultaneous_hands_and_controllers_tracking_meta = None;
        let _ = (session.instance().fp().get_instance_proc_addr)(
            session.instance().as_raw(),
            c"xrPauseSimultaneousHandsAndControllersTrackingMETA".as_ptr(),
            &mut pause_simultaneous_hands_and_controllers_tracking_meta,
        );

        pause_simultaneous_hands_and_controllers_tracking_meta.map(|pfn| {
            mem::transmute::<VoidFunction, PauseSimultaneousHandsAndControllersTrackingMETA>(pfn)
        })
    }
    .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

    let pause_info = SimultaneousHandsAndControllersTrackingPauseInfoMETA {
        ty: *TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_PAUSE_INFO_META,
        next: ptr::null(),
    };
    unsafe {
        super::xr_res(pause_simultaneous_hands_and_controllers_tracking_meta(
            session.as_raw(),
            &pause_info,
        ))
    }
}
