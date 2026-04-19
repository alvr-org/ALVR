use openxr::{
    self as xr, raw,
    sys::{self, Handle},
};
use std::ptr;

pub struct FaceTrackerBD {
    _session: xr::Session<xr::AnyGraphics>,
    handle: sys::FaceTrackerBD,
    ext_fns: raw::FacialSimulationBD,
}

impl FaceTrackerBD {
    pub fn new<G>(session: xr::Session<G>, system: xr::SystemId) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .bd_facial_simulation
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let props = super::get_props(
            &session,
            system,
            sys::SystemFacialSimulationPropertiesBD {
                ty: sys::SystemFacialSimulationPropertiesBD::TYPE,
                next: ptr::null_mut(),
                supports_face_tracking: sys::FALSE,
            },
        )?;

        if props.supports_face_tracking == sys::FALSE {
            return Err(sys::Result::ERROR_FEATURE_UNSUPPORTED);
        }

        let mut modes_count = 0;
        unsafe {
            super::xr_res((ext_fns.enumerate_facial_simulation_modes)(
                session.as_raw(),
                0,
                &mut modes_count,
                ptr::null_mut(),
            ))?;
        }

        let mut modes = vec![xr::FacialSimulationModeBD::default(); modes_count as usize];
        unsafe {
            super::xr_res((ext_fns.enumerate_facial_simulation_modes)(
                session.as_raw(),
                modes_count,
                &mut modes_count,
                modes.as_mut_ptr(),
            ))?;
        }

        // Prefer combined (with audio), fall back to default (no audio). This can fail if no visual
        // tracking is supported.
        let mode = [
            xr::FacialSimulationModeBD::COMBINED_AUDIO,
            xr::FacialSimulationModeBD::DEFAULT,
        ]
        .into_iter()
        .find(|mode| modes.contains(mode))
        .ok_or(sys::Result::ERROR_FEATURE_UNSUPPORTED)?;

        let mut handle = sys::FaceTrackerBD::NULL;
        let info = sys::FaceTrackerCreateInfoBD {
            ty: sys::FaceTrackerCreateInfoBD::TYPE,
            next: ptr::null(),
            mode,
        };
        unsafe {
            super::xr_res((ext_fns.create_face_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?;
        };

        Ok(Self {
            _session: session.into_any_graphics(),
            handle,
            ext_fns,
        })
    }

    pub fn get_facial_simulation_data(&self, time: xr::Time) -> xr::Result<Option<Vec<f32>>> {
        let info = sys::FacialSimulationDataGetInfoBD {
            ty: sys::FacialSimulationDataGetInfoBD::TYPE,
            next: ptr::null(),
            time,
        };

        let mut weights = vec![0.0; sys::FACE_EXPRESSION_COUNT_BD];
        let mut facial_simulation_data = sys::FacialSimulationDataBD {
            ty: sys::FacialSimulationDataBD::TYPE,
            next: ptr::null_mut(),
            face_expression_weight_count: sys::FACE_EXPRESSION_COUNT_BD as u32,
            face_expression_weights: weights.as_mut_ptr(),
            is_upper_face_data_valid: sys::FALSE,
            is_lower_face_data_valid: sys::FALSE,
            time,
        };

        unsafe {
            super::xr_res((self.ext_fns.get_facial_simulation_data)(
                self.handle,
                &info,
                &mut facial_simulation_data,
            ))?;
        }

        if facial_simulation_data.is_upper_face_data_valid == sys::TRUE
            || facial_simulation_data.is_lower_face_data_valid == sys::TRUE
        {
            Ok(Some(weights))
        } else {
            Ok(None)
        }
    }
}

impl Drop for FaceTrackerBD {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_face_tracker)(self.handle);
        }
    }
}
