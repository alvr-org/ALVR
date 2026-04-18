use openxr::{self as xr, raw, sys};
use std::ptr;

pub struct MultimodalMeta {
    session: xr::Session<xr::AnyGraphics>,
    ext_fns: raw::SimultaneousHandsAndControllersMETA,
}

impl MultimodalMeta {
    pub fn new<G>(session: xr::Session<G>, system: xr::SystemId) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .meta_simultaneous_hands_and_controllers
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        if session
            .instance()
            .exts()
            .meta_detached_controllers
            .is_none()
        {
            return Err(sys::Result::ERROR_EXTENSION_NOT_PRESENT);
        }

        let props = super::get_props(
            &session,
            system,
            sys::SystemSimultaneousHandsAndControllersPropertiesMETA {
                ty: sys::SystemSimultaneousHandsAndControllersPropertiesMETA::TYPE,
                next: ptr::null_mut(),
                supports_simultaneous_hands_and_controllers: xr::sys::FALSE,
            },
        )?;

        if props.supports_simultaneous_hands_and_controllers.into() {
            Ok(Self {
                session: session.into_any_graphics(),
                ext_fns,
            })
        } else {
            Err(sys::Result::ERROR_FEATURE_UNSUPPORTED)
        }
    }

    pub fn resume(&self) -> xr::Result<()> {
        let resume_info = sys::SimultaneousHandsAndControllersTrackingResumeInfoMETA {
            ty: sys::SimultaneousHandsAndControllersTrackingResumeInfoMETA::TYPE,
            next: ptr::null(),
        };
        unsafe {
            super::xr_res((self
                .ext_fns
                .resume_simultaneous_hands_and_controllers_tracking)(
                self.session.as_raw(),
                &resume_info,
            ))
        }
    }

    pub fn pause(&self) -> xr::Result<()> {
        let pause_info = sys::SimultaneousHandsAndControllersTrackingPauseInfoMETA {
            ty: sys::SimultaneousHandsAndControllersTrackingPauseInfoMETA::TYPE,
            next: ptr::null(),
        };
        unsafe {
            super::xr_res((self
                .ext_fns
                .pause_simultaneous_hands_and_controllers_tracking)(
                self.session.as_raw(),
                &pause_info,
            ))
        }
    }
}
