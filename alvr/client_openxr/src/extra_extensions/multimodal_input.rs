// Code taken from:
// https://github.com/meta-quest/Meta-OpenXR-SDK/blob/main/OpenXR/meta_openxr_preview/meta_simultaneous_hands_and_controllers.h

use alvr_common::{anyhow::Result, once_cell::sync::Lazy, ToAny};
use openxr::{self as xr, sys};
use std::ffi::c_void;

pub const META_SIMULTANEOUS_HANDS_AND_CONTROLLERS_EXTENSION_NAME: &str =
    "XR_META_simultaneous_hands_and_controllers";
pub const META_DETACHED_CONTROLLERS_EXTENSION_NAME: &str = "XR_META_detached_controllers";

static TYPE_SYSTEM_SIMULTANEOUS_HANDS_AND_CONTROLLERS_PROPERTIES_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532001));
static TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000532002));

#[repr(C)]
pub struct SystemSymultaneousHandsAndControllersPropertiesMETA {
    ty: xr::StructureType,
    next: *const c_void,
    supports_simultaneous_hands_and_controllers: sys::Bool32,
}

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

impl super::ExtraExtensions {
    pub fn supports_simultaneous_hands_and_controllers(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(
            instance,
            system,
            SystemSymultaneousHandsAndControllersPropertiesMETA {
                ty: *TYPE_SYSTEM_SIMULTANEOUS_HANDS_AND_CONTROLLERS_PROPERTIES_META,
                next: std::ptr::null(),
                supports_simultaneous_hands_and_controllers: xr::sys::FALSE,
            },
        )
        .map(|props| props.supports_simultaneous_hands_and_controllers.into())
        .unwrap_or(false)
    }

    pub fn resume_simultaneous_hands_and_controllers_tracking(
        &self,
        session: &xr::Session<xr::OpenGlEs>,
    ) -> Result<()> {
        let resume_info = SimultaneousHandsAndControllersTrackingResumeInfoMETA {
            ty: *TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META,
            next: std::ptr::null(),
        };

        unsafe {
            super::to_any((self
                .resume_simultaneous_hands_and_controllers_tracking_meta
                .to_any()?)(session.as_raw(), &resume_info))?;
        }

        Ok(())
    }
}
