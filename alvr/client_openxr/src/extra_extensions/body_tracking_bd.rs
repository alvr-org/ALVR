use crate::extra_extensions::get_instance_proc;
use alvr_common::once_cell::sync::Lazy;
use openxr::{self as xr, sys};
use std::ffi::{c_char, c_void, CString};
use std::ptr;

pub const BD_BODY_TRACKING_EXTENSION_NAME: &str = "XR_BD_body_tracking";
pub const BD_MOTION_TRACKING_EXTENSION_NAME: &str = "XR_BD_motion_tracking";
pub const PICO_CONFIGURATION_EXTENSION_NAME: &str = "XR_PICO_configuration";

static TYPE_BODY_TRACKER_CREATE_INFO_BD: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000385001));
static TYPE_BODY_JOINTS_LOCATE_INFO_BD: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000385002));
static TYPE_BODY_JOINT_LOCATIONS_BD: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000385003));
static TYPE_SYSTEM_BODY_TRACKING_PROPERTIES_BD: Lazy<xr::StructureType> =
    Lazy::new(|| xr::StructureType::from_raw(1000385004));

pub const BODY_JOINT_PELVIS_BD: usize = 0;
pub const BODY_JOINT_LEFT_KNEE_BD: usize = 4;
pub const BODY_JOINT_RIGHT_KNEE_BD: usize = 5;
pub const BODY_JOINT_SPINE3_BD: usize = 9;
pub const BODY_JOINT_LEFT_FOOT_BD: usize = 10;
pub const BODY_JOINT_RIGHT_FOOT_BD: usize = 11;
pub const BODY_JOINT_LEFT_ELBOW_BD: usize = 18;
pub const BODY_JOINT_RIGHT_ELBOW_BD: usize = 19;
pub const BODY_JOINT_COUNT_BD: usize = 24;

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct XrBodyTrackerBD(u64);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BodyJointSetBD(i32);
impl BodyJointSetBD {
    pub const BODY_WITHOUT_ARM: BodyJointSetBD = Self(1i32);
    pub const FULL_BODY_JOINTS: BodyJointSetBD = Self(2i32);
}

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

#[repr(C)]
struct BodyTrackerCreateInfoBD {
    ty: xr::StructureType,
    next: *const c_void,
    joint_set: BodyJointSetBD,
}

#[repr(C)]
struct BodyJointsLocateInfoBD {
    ty: xr::StructureType,
    next: *const c_void,
    base_space: sys::Space,
    time: sys::Time,
}

#[repr(C)]
pub struct BodyJointLocationBD {
    pub location_flags: sys::SpaceLocationFlags,
    pub pose: sys::Posef,
    pub radius: f32,
}

#[repr(C)]
struct BodyJointLocationsBD {
    ty: xr::StructureType,
    next: *const c_void,
    all_joint_poses_tracked: sys::Bool32,
    joint_count: u32,
    joint_locations: *mut BodyJointLocationBD,
}

#[repr(C)]
struct SystemBodyTrackingPropertiesBD {
    ty: xr::StructureType,
    next: *const c_void,
    supports_body_tracking: sys::Bool32,
}

type CreateBodyTrackerBD = unsafe extern "system" fn(
    sys::Session,
    *const BodyTrackerCreateInfoBD,
    *mut XrBodyTrackerBD,
) -> sys::Result;

type DestroyBodyTrackerBD = unsafe extern "system" fn(XrBodyTrackerBD) -> sys::Result;

type LocateBodyJointsBD = unsafe extern "system" fn(
    XrBodyTrackerBD,
    *const BodyJointsLocateInfoBD,
    *mut BodyJointLocationsBD,
) -> sys::Result;

type StartBodyTrackingCalibAppBD =
    unsafe extern "system" fn(sys::Instance, *const c_char, CalibAppFlagBD) -> sys::Result;

type GetBodyTrackingStateBD = unsafe extern "system" fn(
    sys::Instance,
    *mut BodyTrackingStatusCodeBD,
    *mut BodyTrackingErrorCodeBD,
) -> sys::Result;

#[repr(C)]
struct MotionTrackerConnectStateBD {
    tracker_count: i32,
    serials: [MotionTrackerSerialBD; 6],
}

#[repr(C)]
#[derive(Copy)]
pub struct MotionTrackerSerialBD {
    pub serial: [u8; 24],
}
impl Clone for MotionTrackerSerialBD {
    fn clone(&self) -> Self {
        *self
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct MotionTrackerConfidenceBD(i32);

#[repr(C)]
pub struct MotionTrackerLocationBD {
    pub pose: sys::Posef,
    pub angular_velocity: sys::Vector3f,
    pub linear_velocity: sys::Vector3f,
    pub angular_acceleration: sys::Vector3f,
    pub linear_acceleration: sys::Vector3f,
}

#[repr(C)]
pub struct MotionTrackerLocationsBD {
    pub serial: MotionTrackerSerialBD,
    pub local_pose: MotionTrackerLocationBD,
    pub confidence: MotionTrackerConfidenceBD,
    pub global_pose: MotionTrackerLocationBD,
}

type GetMotionTrackerConnectStateBD =
    unsafe extern "system" fn(sys::Instance, *mut MotionTrackerConnectStateBD) -> sys::Result;

type GetMotionTrackerLocationsBD = unsafe extern "system" fn(
    sys::Instance,
    sys::Time,
    *const MotionTrackerSerialBD,
    *mut MotionTrackerLocationsBD,
) -> sys::Result;

type SetConfigPICO = unsafe extern "system" fn(sys::Session, i32, *const c_char) -> sys::Result;

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct MotionTrackerModeBD(i32);
impl MotionTrackerModeBD {
    pub const BODY_TRACKING: MotionTrackerModeBD = Self(0i32);
    pub const MOTION_TRACKING: MotionTrackerModeBD = Self(1i32);
}

type GetMotionTrackerModeBD =
    unsafe extern "system" fn(sys::Instance, *mut MotionTrackerModeBD) -> sys::Result;

pub struct BodyTrackerBD {
    handle: XrBodyTrackerBD,
    instance: sys::Instance,
    destroy_body_tracker: DestroyBodyTrackerBD,
    locate_body_joints: LocateBodyJointsBD,
    get_body_tracking_state: GetBodyTrackingStateBD,
    get_motion_tracker_connect_state: GetMotionTrackerConnectStateBD,
    get_motion_tracker_locations: GetMotionTrackerLocationsBD,
    get_motion_tracker_mode: GetMotionTrackerModeBD,
}

impl BodyTrackerBD {
    pub fn new<G>(
        session: &xr::Session<G>,
        joint_set: BodyJointSetBD,
        extra_extensions: &[String],
        system: xr::SystemId,
        prompt_calibration: bool,
    ) -> xr::Result<Self> {
        if !extra_extensions.contains(&BD_BODY_TRACKING_EXTENSION_NAME.to_owned())
            || !extra_extensions.contains(&BD_MOTION_TRACKING_EXTENSION_NAME.to_owned())
            || !extra_extensions.contains(&PICO_CONFIGURATION_EXTENSION_NAME.to_owned())
        {
            return Err(sys::Result::ERROR_EXTENSION_NOT_PRESENT);
        }

        let create_body_tracker: CreateBodyTrackerBD =
            get_instance_proc(session, "xrCreateBodyTrackerBD")?;
        let start_body_tracking_calib_app: StartBodyTrackingCalibAppBD =
            get_instance_proc(session, "xrStartBodyTrackingCalibAppBD")?;
        let get_body_tracking_state: GetBodyTrackingStateBD =
            get_instance_proc(session, "xrGetBodyTrackingStateBD")?;
        let destroy_body_tracker = get_instance_proc(session, "xrDestroyBodyTrackerBD")?;
        let locate_body_joints = get_instance_proc(session, "xrLocateBodyJointsBD")?;
        let get_motion_tracker_connect_state =
            get_instance_proc(session, "xrGetMotionTrackerConnectStateBD")?;
        let get_motion_tracker_locations =
            get_instance_proc(session, "xrGetMotionTrackerLocationsBD")?;
        let set_config: SetConfigPICO = get_instance_proc(session, "xrSetConfigPICO")?;
        let get_motion_tracker_mode: GetMotionTrackerModeBD =
            get_instance_proc(session, "xrGetMotionTrackerModeBD")?;

        let props = super::get_props(
            session,
            system,
            SystemBodyTrackingPropertiesBD {
                ty: *TYPE_SYSTEM_BODY_TRACKING_PROPERTIES_BD,
                next: ptr::null(),
                supports_body_tracking: sys::FALSE,
            },
        )?;

        if props.supports_body_tracking == sys::FALSE {
            return Err(sys::Result::ERROR_FEATURE_UNSUPPORTED);
        }

        let mut handle = XrBodyTrackerBD(0);
        let info = BodyTrackerCreateInfoBD {
            ty: *TYPE_BODY_TRACKER_CREATE_INFO_BD,
            next: ptr::null(),
            joint_set,
        };
        unsafe {
            //Floor height tracking origin
            let str = CString::new("1").unwrap();
            //Set config property for tracking origin
            super::xr_res(set_config(session.as_raw(), 1, str.as_ptr()))?;

            super::xr_res(create_body_tracker(session.as_raw(), &info, &mut handle))?;
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
            instance: session.instance().as_raw(),
            destroy_body_tracker,
            locate_body_joints,
            get_body_tracking_state,
            get_motion_tracker_connect_state,
            get_motion_tracker_locations,
            get_motion_tracker_mode,
        })
    }

    pub fn locate_motion_trackers(
        &self,
        time: xr::Time,
    ) -> xr::Result<Vec<MotionTrackerLocationsBD>> {
        let mut locations = Vec::with_capacity(6);

        unsafe {
            let mut tracker_mode = MotionTrackerModeBD::BODY_TRACKING;
            super::xr_res((self.get_motion_tracker_mode)(
                self.instance,
                &mut tracker_mode,
            ))?;

            if tracker_mode != MotionTrackerModeBD::MOTION_TRACKING {
                return Ok(locations);
            }
        }

        let mut connect_state = MotionTrackerConnectStateBD {
            tracker_count: 0,
            serials: [MotionTrackerSerialBD { serial: [0; 24] }; 6],
        };

        unsafe {
            super::xr_res((self.get_motion_tracker_connect_state)(
                self.instance,
                &mut connect_state,
            ))?;

            for i in 0..connect_state.tracker_count as usize {
                let mut location = MotionTrackerLocationsBD {
                    serial: MotionTrackerSerialBD { serial: [0; 24] },
                    local_pose: MotionTrackerLocationBD {
                        pose: xr::Posef::IDENTITY,
                        angular_velocity: Default::default(),
                        linear_velocity: Default::default(),
                        angular_acceleration: Default::default(),
                        linear_acceleration: Default::default(),
                    },
                    confidence: MotionTrackerConfidenceBD(0),
                    global_pose: MotionTrackerLocationBD {
                        pose: xr::Posef::IDENTITY,
                        angular_velocity: Default::default(),
                        linear_velocity: Default::default(),
                        angular_acceleration: Default::default(),
                        linear_acceleration: Default::default(),
                    },
                };

                super::xr_res((self.get_motion_tracker_locations)(
                    self.instance,
                    time,
                    &connect_state.serials[i],
                    &mut location,
                ))?;

                locations.push(location);
            }
        }

        Ok(locations)
    }

    pub fn locate_body_joints(
        &self,
        time: xr::Time,
        reference_space: &xr::Space,
    ) -> xr::Result<Option<Vec<BodyJointLocationBD>>> {
        unsafe {
            let mut tracker_mode = MotionTrackerModeBD::BODY_TRACKING;
            super::xr_res((self.get_motion_tracker_mode)(
                self.instance,
                &mut tracker_mode,
            ))?;

            if tracker_mode != MotionTrackerModeBD::BODY_TRACKING {
                return Ok(None);
            }
        }

        let mut status_code = BodyTrackingStatusCodeBD::INVALID;
        let mut error_code = BodyTrackingErrorCodeBD::INNER_EXCEPTION;

        unsafe {
            super::xr_res((self.get_body_tracking_state)(
                self.instance,
                &mut status_code,
                &mut error_code,
            ))?;
        }

        if status_code == BodyTrackingStatusCodeBD::INVALID {
            return Ok(None);
        }

        let locate_info = BodyJointsLocateInfoBD {
            ty: *TYPE_BODY_JOINTS_LOCATE_INFO_BD,
            next: ptr::null(),
            base_space: reference_space.as_raw(),
            time,
        };

        let joint_count = BODY_JOINT_COUNT_BD;
        let mut locations = Vec::with_capacity(joint_count);

        let mut location_info = BodyJointLocationsBD {
            ty: *TYPE_BODY_JOINT_LOCATIONS_BD,
            next: ptr::null(),
            all_joint_poses_tracked: sys::FALSE,
            joint_count: joint_count as u32,
            joint_locations: locations.as_mut_ptr() as _,
        };

        unsafe {
            super::xr_res((self.locate_body_joints)(
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
            (self.destroy_body_tracker)(self.handle);
        }
    }
}
