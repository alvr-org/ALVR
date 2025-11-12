use openxr::{
    self as xr, raw,
    sys::{self, Handle},
};
use std::ptr;

pub struct FaceTracker2FB {
    // Keeping a reference to the session to ensure that the tracker handle remains valid
    _session: xr::Session<xr::AnyGraphics>,
    handle: sys::FaceTracker2FB,
    ext_fns: raw::FaceTracking2FB,
}

impl FaceTracker2FB {
    pub fn new<G>(session: xr::Session<G>, visual: bool, audio: bool) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .fb_face_tracking2
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let mut requested_data_sources = vec![];
        if visual {
            requested_data_sources.push(sys::FaceTrackingDataSource2FB::VISUAL);
        }
        if audio {
            requested_data_sources.push(sys::FaceTrackingDataSource2FB::AUDIO);
        }

        let mut handle = sys::FaceTracker2FB::NULL;
        let info = sys::FaceTrackerCreateInfo2FB {
            ty: sys::FaceTrackerCreateInfo2FB::TYPE,
            next: ptr::null(),
            face_expression_set: xr::FaceExpressionSet2FB::DEFAULT,
            requested_data_source_count: requested_data_sources.len() as u32,
            requested_data_sources: requested_data_sources.as_mut_ptr(),
        };
        unsafe {
            super::xr_res((ext_fns.create_face_tracker2)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        Ok(Self {
            _session: session.into_any_graphics(),
            handle,
            ext_fns,
        })
    }

    pub fn get_face_expression_weights(&self, time: xr::Time) -> xr::Result<Option<Vec<f32>>> {
        let expression_info = sys::FaceExpressionInfo2FB {
            ty: sys::FaceExpressionInfo2FB::TYPE,
            next: ptr::null(),
            time,
        };

        let weights_count = xr::FaceExpression2FB::COUNT.into_raw() as usize;
        let confidence_count = xr::FaceConfidence2FB::COUNT.into_raw() as usize;

        let mut weights: Vec<f32> = Vec::with_capacity(weights_count);
        let mut confidences: Vec<f32> = vec![0.0; confidence_count];

        let mut expression_weights = sys::FaceExpressionWeights2FB {
            ty: sys::FaceExpressionWeights2FB::TYPE,
            next: ptr::null_mut(),
            weight_count: weights_count as u32,
            weights: weights.as_mut_ptr(),
            confidence_count: confidence_count as u32,
            confidences: confidences.as_mut_ptr(),
            is_valid: sys::FALSE,
            is_eye_following_blendshapes_valid: sys::FALSE,
            data_source: sys::FaceTrackingDataSource2FB::from_raw(0),
            time: xr::Time::from_nanos(0),
        };

        unsafe {
            super::xr_res((self.ext_fns.get_face_expression_weights2)(
                self.handle,
                &expression_info,
                &mut expression_weights,
            ))?;

            if expression_weights.is_valid.into() {
                weights.set_len(weights_count);

                Ok(Some(weights))
            } else {
                Ok(None)
            }
        }
    }
}

impl Drop for FaceTracker2FB {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_face_tracker2)(self.handle);
        }
    }
}
