mod body_tracking_fb;
mod eye_tracking_social;
mod face_tracking2_fb;
mod facial_tracking_htc;
mod multimodal_input;

pub use body_tracking_fb::*;
pub use eye_tracking_social::*;
pub use face_tracking2_fb::*;
pub use facial_tracking_htc::*;
pub use multimodal_input::*;

use alvr_common::anyhow::{anyhow, Result};
use openxr::{self as xr, sys};
use std::{mem, ptr};

fn to_any(result: sys::Result) -> Result<()> {
    if result.into_raw() >= 0 {
        Ok(())
    } else {
        Err(anyhow!("OpenXR error: {:?}", result))
    }
}

#[derive(Clone)]
pub struct ExtraExtensions {
    base_function_ptrs: xr::raw::Instance,
    ext_functions_ptrs: xr::InstanceExtensions,
    resume_simultaneous_hands_and_controllers_tracking_meta:
        Option<ResumeSimultaneousHandsAndControllersTrackingMETA>,
}

impl ExtraExtensions {
    pub fn new(instance: &xr::Instance) -> Self {
        let resume_simultaneous_hands_and_controllers_tracking_meta = unsafe {
            let mut resume_simultaneous_hands_and_controllers_tracking_meta = None;
            let _ = (instance.fp().get_instance_proc_addr)(
                instance.as_raw(),
                c"xrResumeSimultaneousHandsAndControllersTrackingMETA".as_ptr(),
                &mut resume_simultaneous_hands_and_controllers_tracking_meta,
            );

            resume_simultaneous_hands_and_controllers_tracking_meta
                .map(|f| mem::transmute::<_, ResumeSimultaneousHandsAndControllersTrackingMETA>(f))
        };

        Self {
            base_function_ptrs: instance.fp().clone(),
            ext_functions_ptrs: *instance.exts(),
            resume_simultaneous_hands_and_controllers_tracking_meta,
        }
    }

    fn get_props<T>(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
        default_struct: T,
    ) -> Option<T> {
        let mut props = default_struct;

        let mut system_properties = sys::SystemProperties::out((&mut props as *mut T).cast());
        let result = unsafe {
            (self.base_function_ptrs.get_system_properties)(
                instance.as_raw(),
                system,
                system_properties.as_mut_ptr(),
            )
        };

        (result.into_raw() >= 0).then_some(props)
    }

    pub fn supports_eye_gaze_interaction(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(instance, system, unsafe {
            sys::SystemEyeGazeInteractionPropertiesEXT::out(ptr::null_mut()).assume_init()
        })
        .map(|props| props.supports_eye_gaze_interaction.into())
        .unwrap_or(false)
    }
}
