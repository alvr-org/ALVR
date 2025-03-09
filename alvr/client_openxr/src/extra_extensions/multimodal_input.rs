// Code taken from:
// https://github.com/meta-quest/Meta-OpenXR-SDK/blob/main/OpenXR/meta_openxr_preview/meta_simultaneous_hands_and_controllers.h

use crate::extra_extensions::get_instance_proc;
use alvr_common::once_cell::sync::Lazy;
use openxr::{
    self as xr,
    sys::{self},
};
use std::{ffi::c_void, ptr};

pub const META_SIMULTANEOUS_HANDS_AND_CONTROLLERS_EXTENSION_NAME: &str =
    "XR_META_simultaneous_hands_and_controllers";
pub const META_DETACHED_CONTROLLERS_EXTENSION_NAME: &str = "XR_META_detached_controllers";

static TYPE_SYSTEM_SIMULTANEOUS_HANDS_AND_CONTROLLERS_PROPERTIES_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532001));
static TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532002));
static TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_PAUSE_INFO_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532003));

#[repr(C)]
struct SystemSymultaneousHandsAndControllersPropertiesMETA {
    ty: xr::StructureType,
    next: *const c_void,
    supports_simultaneous_hands_and_controllers: sys::Bool32,
}

#[repr(C)]
struct SimultaneousHandsAndControllersTrackingResumeInfoMETA {
    ty: xr::StructureType,
    next: *const c_void,
}
#[repr(C)]
struct SimultaneousHandsAndControllersTrackingPauseInfoMETA {
    ty: xr::StructureType,
    next: *const c_void,
}

type ResumeSimultaneousHandsAndControllersTrackingMETA = unsafe extern "system" fn(
    sys::Session,
    *const SimultaneousHandsAndControllersTrackingResumeInfoMETA,
) -> sys::Result;
type PauseSimultaneousHandsAndControllersTrackingMETA = unsafe extern "system" fn(
    sys::Session,
    *const SimultaneousHandsAndControllersTrackingPauseInfoMETA,
) -> sys::Result;

pub struct MultimodalMeta {
    session: xr::Session<xr::AnyGraphics>,
    resume_simultaneous_hands_and_controllers_tracking_meta:
        ResumeSimultaneousHandsAndControllersTrackingMETA,
    pause_simultaneous_hands_and_controllers_tracking_meta:
        PauseSimultaneousHandsAndControllersTrackingMETA,
}

impl MultimodalMeta {
    pub fn new<G>(
        session: xr::Session<G>,
        extra_extensions: &[String],
        system: xr::SystemId,
    ) -> xr::Result<Self> {
        if !extra_extensions
            .contains(&META_SIMULTANEOUS_HANDS_AND_CONTROLLERS_EXTENSION_NAME.to_owned())
            || !extra_extensions.contains(&META_DETACHED_CONTROLLERS_EXTENSION_NAME.to_owned())
        {
            return Err(sys::Result::ERROR_EXTENSION_NOT_PRESENT);
        }

        let resume_simultaneous_hands_and_controllers_tracking_meta = get_instance_proc(
            &session,
            "xrResumeSimultaneousHandsAndControllersTrackingMETA",
        )?;
        let pause_simultaneous_hands_and_controllers_tracking_meta = get_instance_proc(
            &session,
            "xrPauseSimultaneousHandsAndControllersTrackingMETA",
        )?;

        let props = super::get_props(
            &session,
            system,
            SystemSymultaneousHandsAndControllersPropertiesMETA {
                ty: *TYPE_SYSTEM_SIMULTANEOUS_HANDS_AND_CONTROLLERS_PROPERTIES_META,
                next: ptr::null(),
                supports_simultaneous_hands_and_controllers: xr::sys::FALSE,
            },
        )?;

        if props.supports_simultaneous_hands_and_controllers.into() {
            Ok(Self {
                session: session.into_any_graphics(),
                resume_simultaneous_hands_and_controllers_tracking_meta,
                pause_simultaneous_hands_and_controllers_tracking_meta,
            })
        } else {
            Err(sys::Result::ERROR_FEATURE_UNSUPPORTED)
        }
    }

    pub fn resume(&self) -> xr::Result<()> {
        let resume_info = SimultaneousHandsAndControllersTrackingResumeInfoMETA {
            ty: *TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META,
            next: ptr::null(),
        };
        unsafe {
            super::xr_res((self
                .resume_simultaneous_hands_and_controllers_tracking_meta)(
                self.session.as_raw(),
                &resume_info,
            ))
        }
    }

    pub fn pause(&self) -> xr::Result<()> {
        let pause_info = SimultaneousHandsAndControllersTrackingPauseInfoMETA {
            ty: *TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_PAUSE_INFO_META,
            next: ptr::null(),
        };
        unsafe {
            super::xr_res((self
                .pause_simultaneous_hands_and_controllers_tracking_meta)(
                self.session.as_raw(),
                &pause_info,
            ))
        }
    }
}
