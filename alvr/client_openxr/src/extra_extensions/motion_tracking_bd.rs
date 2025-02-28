use crate::extra_extensions::get_instance_proc;
use openxr::{self as xr, sys, AnyGraphics};
use std::ffi::{c_char, CString};

pub const BD_MOTION_TRACKING_EXTENSION_NAME: &str = "XR_BD_motion_tracking";
pub const PICO_CONFIGURATION_EXTENSION_NAME: &str = "XR_PICO_configuration";

#[repr(C)]
struct MotionTrackerConnectStateBD {
    tracker_count: i32,
    serials: [MotionTrackerSerialBD; 6],
}

#[repr(C)]
#[derive(Copy, Ord, Eq, PartialEq, PartialOrd)]
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

pub struct MotionTrackerBD {
    session: xr::Session<AnyGraphics>,
    get_motion_tracker_connect_state: GetMotionTrackerConnectStateBD,
    get_motion_tracker_locations: GetMotionTrackerLocationsBD,
}

impl MotionTrackerBD {
    pub fn new<G>(session: xr::Session<G>, extra_extensions: &[String]) -> xr::Result<Self> {
        if !extra_extensions.contains(&BD_MOTION_TRACKING_EXTENSION_NAME.to_owned())
            || !extra_extensions.contains(&PICO_CONFIGURATION_EXTENSION_NAME.to_owned())
        {
            return Err(sys::Result::ERROR_EXTENSION_NOT_PRESENT);
        }

        let get_motion_tracker_connect_state =
            get_instance_proc(&session, "xrGetMotionTrackerConnectStateBD")?;
        let get_motion_tracker_locations =
            get_instance_proc(&session, "xrGetMotionTrackerLocationsBD")?;
        let set_config: SetConfigPICO = get_instance_proc(&session, "xrSetConfigPICO")?;

        unsafe {
            //Floor height tracking origin
            let str = CString::new("1").unwrap();
            //Set config property for tracking origin
            super::xr_res(set_config(session.as_raw(), 1, str.as_ptr()))?;
        };

        Ok(Self {
            session: session.into_any_graphics(),
            get_motion_tracker_connect_state,
            get_motion_tracker_locations,
        })
    }

    pub fn locate_motion_trackers(
        &self,
        time: xr::Time,
    ) -> xr::Result<Option<Vec<MotionTrackerLocationsBD>>> {
        let mut locations = Vec::with_capacity(3);

        let mut connect_state = MotionTrackerConnectStateBD {
            tracker_count: 0,
            serials: [MotionTrackerSerialBD { serial: [0; 24] }; 6],
        };

        unsafe {
            super::xr_res((self.get_motion_tracker_connect_state)(
                self.session.instance().as_raw(),
                &mut connect_state,
            ))?;

            // Pico doesn't provide a way to connect more than three trackers now
            if connect_state.tracker_count > 3 {
                connect_state.tracker_count = 3
            }

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
                    self.session.instance().as_raw(),
                    time,
                    &connect_state.serials[i],
                    &mut location,
                ))?;

                locations.push(location);
            }
        }

        Ok(Some(locations))
    }
}
