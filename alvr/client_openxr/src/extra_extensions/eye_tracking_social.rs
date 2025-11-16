use openxr::{
    self as xr, raw,
    sys::{self, Handle},
};
use std::ptr;

pub struct EyeTrackerSocial {
    handle: sys::EyeTrackerFB,
    ext_fns: raw::EyeTrackingSocialFB,
}

impl EyeTrackerSocial {
    pub fn new<G>(session: &xr::Session<G>) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .fb_eye_tracking_social
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let mut handle = sys::EyeTrackerFB::NULL;
        let info = sys::EyeTrackerCreateInfoFB {
            ty: sys::EyeTrackerCreateInfoFB::TYPE,
            next: ptr::null(),
        };
        unsafe {
            super::xr_res((ext_fns.create_eye_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        Ok(Self { handle, ext_fns })
    }

    pub fn get_eye_gazes(
        &self,
        base: &xr::Space,
        time: xr::Time,
    ) -> xr::Result<[Option<xr::Posef>; 2]> {
        let gaze_info = sys::EyeGazesInfoFB {
            ty: sys::EyeGazesInfoFB::TYPE,
            next: ptr::null(),
            base_space: base.as_raw(),
            time,
        };

        let mut eye_gazes = sys::EyeGazesFB::out(ptr::null_mut());

        let eye_gazes = unsafe {
            super::xr_res((self.ext_fns.get_eye_gazes)(
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
