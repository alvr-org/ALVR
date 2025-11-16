#![allow(dead_code)]

use crate::extra_extensions::get_instance_proc;
use openxr::{
    self as xr, raw,
    sys::{self, Handle},
};
use std::{ptr, sync::LazyLock};

pub const META_BODY_TRACKING_FULL_BODY_EXTENSION_NAME: &str = "XR_META_body_tracking_full_body";
pub static BODY_JOINT_SET_FULL_BODY_META: LazyLock<xr::BodyJointSetFB> =
    LazyLock::new(|| xr::BodyJointSetFB::from_raw(1000274000));
pub const META_BODY_TRACKING_FIDELITY_EXTENSION_NAME: &str = "XR_META_body_tracking_fidelity";
pub static SYSTEM_PROPERTIES_BODY_TRACKING_FIDELITY_META: LazyLock<xr::StructureType> =
    LazyLock::new(|| xr::StructureType::from_raw(1000284001));
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

#[repr(C)]
struct SystemPropertiesBodyTrackingFidelityMETA {
    ty: xr::StructureType,
    next: *mut std::ffi::c_void,
    supports_body_tracking_fidelity: sys::Bool32,
}

#[repr(C)]
enum BodyTrackingFidelityMode {
    Low = 1,
    High = 2,
}

type RequestBodyTrackingFidelityMETA =
    unsafe extern "system" fn(sys::BodyTrackerFB, BodyTrackingFidelityMode) -> sys::Result;

impl BodyTrackerFB {
    pub fn new<G>(
        session: &xr::Session<G>,
        system: xr::SystemId,
        body_joint_set: xr::BodyJointSetFB,
        prefer_high_fidelity: bool,
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
        let body_tracking_fidelity_props = super::get_props(
            session,
            system,
            SystemPropertiesBodyTrackingFidelityMETA {
                ty: *SYSTEM_PROPERTIES_BODY_TRACKING_FIDELITY_META,
                next: ptr::null_mut(),
                supports_body_tracking_fidelity: sys::FALSE,
            },
        )?;
        let preferred_fidelity_mode: BodyTrackingFidelityMode = if prefer_high_fidelity {
            BodyTrackingFidelityMode::High
        } else {
            BodyTrackingFidelityMode::Low
        };
        unsafe {
            super::xr_res((ext_fns.create_body_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?;

            if body_tracking_fidelity_props.supports_body_tracking_fidelity == sys::TRUE {
                let request_body_tracking_fidelity: RequestBodyTrackingFidelityMETA =
                    get_instance_proc(session, "xrRequestBodyTrackingFidelityMETA")?;
                super::xr_res(request_body_tracking_fidelity(
                    handle,
                    preferred_fidelity_mode,
                ))
                .ok(); // This is very unlikely to fail as the void falls back to Low on an invalid call.
            }
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
        let mut locations: Vec<sys::BodyJointLocationFB> = Vec::with_capacity(joint_count);
        let mut location_info = sys::BodyJointLocationsFB {
            ty: sys::BodyJointLocationsFB::TYPE,
            next: ptr::null_mut(),
            is_active: sys::FALSE,
            confidence: 0.0,
            joint_count: joint_count as u32,
            joint_locations: locations.as_mut_ptr(),
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
