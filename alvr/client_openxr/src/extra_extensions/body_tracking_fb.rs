#![allow(dead_code)]

use alvr_common::{anyhow::Result, once_cell::sync::Lazy, ToAny};
use openxr::{self as xr, raw, sys};
use std::ptr;

pub const META_BODY_TRACKING_FULL_BODY_EXTENSION_NAME: &str = "XR_META_body_tracking_full_body";
static TYPE_SYSTEM_PROPERTIES_BODY_TRACKING_FULL_BODY_META: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000274000));
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

impl super::ExtraExtensions {
    pub fn supports_body_tracking_fb(&self, instance: &xr::Instance, system: xr::SystemId) -> bool {
        self.get_props(
            instance,
            system,
            sys::SystemBodyTrackingPropertiesFB {
                ty: sys::SystemBodyTrackingPropertiesFB::TYPE,
                next: ptr::null_mut(),
                supports_body_tracking: sys::FALSE,
            },
        )
        .map(|props| props.supports_body_tracking.into())
        .unwrap_or(false)
    }

    pub fn supports_full_body_tracking_meta(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(
            instance,
            system,
            SystemPropertiesBodyTrackingFullBodyMETA {
                ty: *TYPE_SYSTEM_PROPERTIES_BODY_TRACKING_FULL_BODY_META,
                next: ptr::null_mut(),
                supports_full_body_tracking: sys::FALSE,
            },
        )
        .map(|props| props.supports_full_body_tracking.into())
        .unwrap_or(false)
    }

    pub fn create_body_tracker_fb<G>(
        &self,
        session: &xr::Session<G>,
        body_joint_set: xr::BodyJointSetFB,
    ) -> Result<BodyTrackerFB> {
        let ext_fns = self.ext_functions_ptrs.fb_body_tracking.to_any()?;

        let mut handle = sys::BodyTrackerFB::NULL;
        let info = sys::BodyTrackerCreateInfoFB {
            ty: sys::BodyTrackerCreateInfoFB::TYPE,
            next: ptr::null(),
            body_joint_set,
        };
        unsafe {
            super::to_any((ext_fns.create_body_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        Ok(BodyTrackerFB { handle, ext_fns })
    }
}

pub struct BodyTrackerFB {
    handle: sys::BodyTrackerFB,
    ext_fns: raw::BodyTrackingFB,
}

impl BodyTrackerFB {
    pub fn locate_body_joints(
        &self,
        time: xr::Time,
        reference_space: &xr::Space,
        joint_count: usize,
    ) -> Result<Option<Vec<xr::BodyJointLocationFB>>> {
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
            super::to_any((self.ext_fns.locate_body_joints)(
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
