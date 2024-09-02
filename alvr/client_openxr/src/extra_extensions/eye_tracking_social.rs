use alvr_common::{anyhow::Result, ToAny};
use openxr::{self as xr, raw, sys};
use std::ptr;

impl super::ExtraExtensions {
    pub fn supports_social_eye_tracking(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(instance, system, unsafe {
            sys::SystemEyeTrackingPropertiesFB::out(ptr::null_mut()).assume_init()
        })
        .map(|props| props.supports_eye_tracking.into())
        .unwrap_or(false)
    }

    pub fn create_eye_tracker_social<G>(
        &self,
        session: &xr::Session<G>,
    ) -> Result<EyeTrackerSocial> {
        let ext_fns = self.ext_functions_ptrs.fb_eye_tracking_social.to_any()?;

        let mut handle = sys::EyeTrackerFB::NULL;
        let info = sys::EyeTrackerCreateInfoFB {
            ty: sys::EyeTrackerCreateInfoFB::TYPE,
            next: ptr::null(),
        };
        unsafe {
            super::to_any((ext_fns.create_eye_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        Ok(EyeTrackerSocial { handle, ext_fns })
    }
}

pub struct EyeTrackerSocial {
    handle: sys::EyeTrackerFB,
    ext_fns: raw::EyeTrackingSocialFB,
}

impl EyeTrackerSocial {
    pub fn get_eye_gazes(
        &self,
        base: &xr::Space,
        time: xr::Time,
    ) -> Result<[Option<xr::Posef>; 2]> {
        let gaze_info = sys::EyeGazesInfoFB {
            ty: sys::EyeGazesInfoFB::TYPE,
            next: ptr::null(),
            base_space: base.as_raw(),
            time,
        };

        let mut eye_gazes = sys::EyeGazesFB::out(ptr::null_mut());

        let eye_gazes = unsafe {
            super::to_any((self.ext_fns.get_eye_gazes)(
                self.handle,
                &gaze_info,
                eye_gazes.as_mut_ptr(),
            ))?;

            eye_gazes.assume_init()
        };

        let left_valid: bool = eye_gazes.gaze[0].is_valid.into();
        let right_valid: bool = eye_gazes.gaze[1].is_valid.into();

        Ok([
            left_valid.then(|| eye_gazes.gaze[0].gaze_pose),
            right_valid.then(|| eye_gazes.gaze[1].gaze_pose),
        ])
    }
}

impl Drop for EyeTrackerSocial {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_eye_tracker)(self.handle);
        }
    }
}
