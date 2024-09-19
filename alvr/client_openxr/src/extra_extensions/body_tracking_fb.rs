#![allow(dead_code)]

use alvr_common::once_cell::sync::Lazy;
use openxr::{self as xr, raw, sys};
use std::ptr;

pub const META_BODY_TRACKING_FULL_BODY_EXTENSION_NAME: &str = "XR_META_body_tracking_full_body";
pub static BODY_JOINT_SET_FULL_BODY_META: Lazy<xr::BodyJointSetFB> =
    Lazy::new(|| xr::BodyJointSetFB::from_raw(1000274000));

pub const FULL_BODY_JOINT_LEFT_UPPER_LEG_META: usize = 70;
pub const FULL_BODY_JOINT_LEFT_LOWER_LEG_META: usize = 71;
pub const FULL_BODY_JOINT_LEFT_FOOT_ANKLE_TWIST_META: usize = 72;
pub const FULL_BODY_JOINT_LEFT_FOOT_ANKLE_META: usize = 73;
pub const FULL_BODY_JOINT_LEFT_FOOT_SUBTALAR_META: usize = 74;
pub const FULL_BODY_JOINT_LEFT_FOOT_TRANSVERSE_META: usize = 75;
pub const FULL_BODY_JOINT_LEFT_FOOT_BALL_META: usize = 76;
pub const FULL_BODY_JOINT_RIGHT_UPPER_LEG_META: usize = 77;
pub const FULL_BODY_JOINT_RIGHT_LOWER_LEG_META: usize = 78;
pub const FULL_BODY_JOINT_RIGHT_FOOT_ANKLE_TWIST_META: usize = 79;
pub const FULL_BODY_JOINT_RIGHT_FOOT_ANKLE_META: usize = 80;
pub const FULL_BODY_JOINT_RIGHT_FOOT_SUBTALAR_META: usize = 81;
pub const FULL_BODY_JOINT_RIGHT_FOOT_TRANSVERSE_META: usize = 82;
pub const FULL_BODY_JOINT_RIGHT_FOOT_BALL_META: usize = 83;
pub const FULL_BODY_JOINT_COUNT_META: usize = 84;

#[repr(C)]
struct SystemPropertiesBodyTrackingFullBodyMETA {
    ty: xr::StructureType,
    next: *mut std::ffi::c_void,
    supports_full_body_tracking: sys::Bool32,
}

pub struct BodyTrackerFB {
    handle: sys::BodyTrackerFB,
    ext_fns: raw::BodyTrackingFB,
}

impl BodyTrackerFB {
    pub fn new<G>(
        session: &xr::Session<G>,
        body_joint_set: xr::BodyJointSetFB,
    ) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .fb_body_tracking
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let mut handle = sys::BodyTrackerFB::NULL;
        let info = sys::BodyTrackerCreateInfoFB {
            ty: sys::BodyTrackerCreateInfoFB::TYPE,
            next: ptr::null(),
            body_joint_set,
        };
        unsafe {
            super::xr_res((ext_fns.create_body_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?;
        };

        Ok(Self { handle, ext_fns })
    }

    pub fn locate_body_joints(
        &self,
        time: xr::Time,
        reference_space: &xr::Space,
        joint_count: usize,
    ) -> xr::Result<Option<Vec<xr::BodyJointLocationFB>>> {
        let locate_info = sys::BodyJointsLocateInfoFB {
            ty: sys::BodyJointsLocateInfoFB::TYPE,
            next: ptr::null(),
            base_space: reference_space.as_raw(),
            time,
        };
        let mut locations = Vec::with_capacity(joint_count);
        let mut location_info = sys::BodyJointLocationsFB {
            ty: sys::BodyJointLocationsFB::TYPE,
            next: ptr::null_mut(),
            is_active: sys::FALSE,
            confidence: 0.0,
            joint_count: joint_count as u32,
            joint_locations: locations.as_mut_ptr() as _,
            skeleton_changed_count: 0,
            time: xr::Time::from_nanos(0),
        };
        unsafe {
            super::xr_res((self.ext_fns.locate_body_joints)(
                self.handle,
                &locate_info,
                &mut location_info,
            ))?;

            Ok(if location_info.is_active.into() {
                locations.set_len(joint_count);

                Some(locations)
            } else {
                None
            })
        }
    }
}

impl Drop for BodyTrackerFB {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_body_tracker)(self.handle);
        }
    }
}
