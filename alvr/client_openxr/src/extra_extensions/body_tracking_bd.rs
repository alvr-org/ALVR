use crate::extra_extensions::get_instance_proc;
use openxr::{
    self as xr, AnyGraphics, raw,
    sys::{self, Handle},
};
use std::{
    ffi::{CString, c_char},
    ptr,
};

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BodyTrackingStatusCodeBD(i32);
impl BodyTrackingStatusCodeBD {
    pub const INVALID: BodyTrackingStatusCodeBD = Self(0i32);
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BodyTrackingErrorCodeBD(i32);
impl BodyTrackingErrorCodeBD {
    pub const INNER_EXCEPTION: BodyTrackingErrorCodeBD = Self(0i32);
    pub const TRACKER_NOT_CALIBRATED: BodyTrackingErrorCodeBD = Self(1i32);
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct CalibAppFlagBD(i32);
impl CalibAppFlagBD {
    pub const MOTION_TRACKER_2: CalibAppFlagBD = Self(1i32);
}

type StartBodyTrackingCalibAppBD =
    unsafe extern "system" fn(sys::Instance, *const c_char, CalibAppFlagBD) -> sys::Result;

type GetBodyTrackingStateBD = unsafe extern "system" fn(
    sys::Instance,
    *mut BodyTrackingStatusCodeBD,
    *mut BodyTrackingErrorCodeBD,
) -> sys::Result;

pub struct BodyTrackerBD {
    handle: sys::BodyTrackerBD,
    session: xr::Session<AnyGraphics>,
    ext_fns: raw::BodyTrackingBD,
    get_body_tracking_state: GetBodyTrackingStateBD,
}

impl BodyTrackerBD {
    pub fn new<G>(
        session: xr::Session<G>,
        joint_set: xr::BodyJointSetBD,
        system: xr::SystemId,
        prompt_calibration: bool,
    ) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .bd_body_tracking
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let start_body_tracking_calib_app: StartBodyTrackingCalibAppBD =
            get_instance_proc(&session, "xrStartBodyTrackingCalibAppBD")?;
        let get_body_tracking_state: GetBodyTrackingStateBD =
            get_instance_proc(&session, "xrGetBodyTrackingStateBD")?;

        let props = super::get_props(
            &session,
            system,
            sys::SystemBodyTrackingPropertiesBD {
                ty: sys::SystemBodyTrackingPropertiesBD::TYPE,
                next: ptr::null_mut(),
                supports_body_tracking: sys::FALSE,
            },
        )?;

        if props.supports_body_tracking == sys::FALSE {
            return Err(sys::Result::ERROR_FEATURE_UNSUPPORTED);
        }

        let mut handle = sys::BodyTrackerBD::NULL;
        let info = sys::BodyTrackerCreateInfoBD {
            ty: sys::BodyTrackerCreateInfoBD::TYPE,
            next: ptr::null(),
            joint_set,
        };

        unsafe {
            super::xr_res((ext_fns.create_body_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?;
        };

        let mut status_code = BodyTrackingStatusCodeBD::INVALID;
        let mut error_code = BodyTrackingErrorCodeBD::INNER_EXCEPTION;

        if prompt_calibration {
            unsafe {
                super::xr_res(get_body_tracking_state(
                    session.instance().as_raw(),
                    &mut status_code,
                    &mut error_code,
                ))?;

                // todo: include actual Android package name
                let package_name = CString::new("").unwrap();

                if status_code == BodyTrackingStatusCodeBD::INVALID
                    || error_code == BodyTrackingErrorCodeBD::TRACKER_NOT_CALIBRATED
                {
                    super::xr_res(start_body_tracking_calib_app(
                        session.instance().as_raw(),
                        package_name.as_ptr(),
                        CalibAppFlagBD::MOTION_TRACKER_2,
                    ))?;
                }
            }
        }

        Ok(Self {
            handle,
            session: session.into_any_graphics(),
            ext_fns,
            get_body_tracking_state,
        })
    }

    pub fn locate_body_joints(
        &self,
        time: xr::Time,
        reference_space: &xr::Space,
    ) -> xr::Result<Option<Vec<xr::BodyJointLocationBD>>> {
        let mut status_code = BodyTrackingStatusCodeBD::INVALID;
        let mut error_code = BodyTrackingErrorCodeBD::INNER_EXCEPTION;

        unsafe {
            super::xr_res((self.get_body_tracking_state)(
                self.session.instance().as_raw(),
                &mut status_code,
                &mut error_code,
            ))?;
        }

        if status_code == BodyTrackingStatusCodeBD::INVALID {
            return Ok(None);
        }

        let locate_info = sys::BodyJointsLocateInfoBD {
            ty: sys::BodyJointsLocateInfoBD::TYPE,
            next: ptr::null(),
            base_space: reference_space.as_raw(),
            time,
        };

        let joint_count = sys::BODY_JOINT_COUNT_BD;
        let mut locations: Vec<xr::BodyJointLocationBD> = Vec::with_capacity(joint_count);

        let mut location_info = sys::BodyJointLocationsBD {
            ty: sys::BodyJointLocationsBD::TYPE,
            next: ptr::null_mut(),
            all_joint_poses_tracked: sys::FALSE,
            joint_location_count: joint_count as u32,
            joint_locations: locations.as_mut_ptr(),
        };

        unsafe {
            super::xr_res((self.ext_fns.locate_body_joints)(
                self.handle,
                &locate_info,
                &mut location_info,
            ))?;

            Ok(if location_info.all_joint_poses_tracked.into() {
                locations.set_len(joint_count);

                Some(locations)
            } else {
                None
            })
        }
    }
}

impl Drop for BodyTrackerBD {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_body_tracker)(self.handle);
        }
    }
}
