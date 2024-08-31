use alvr_common::{anyhow::Result, ToAny};
use openxr::{self as xr, raw, sys};
use std::ptr;

impl super::ExtraExtensions {
    pub fn supports_fb_visual_face_tracking(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(instance, system, unsafe {
            sys::SystemFaceTrackingProperties2FB::out(ptr::null_mut()).assume_init()
        })
        .map(|props| props.supports_visual_face_tracking.into())
        .unwrap_or(false)
    }

    pub fn supports_fb_audio_face_tracking(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(instance, system, unsafe {
            sys::SystemFaceTrackingProperties2FB::out(ptr::null_mut()).assume_init()
        })
        .map(|props| props.supports_audio_face_tracking.into())
        .unwrap_or(false)
    }

    pub fn create_face_tracker2_fb<G>(
        &self,
        session: &xr::Session<G>,
        visual: bool,
        audio: bool,
    ) -> Result<FaceTracker2FB> {
        let ext_fns = self.ext_functions_ptrs.fb_face_tracking2.to_any()?;

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
            super::to_any((ext_fns.create_face_tracker2)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        Ok(FaceTracker2FB { handle, ext_fns })
    }
}

pub struct FaceTracker2FB {
    handle: sys::FaceTracker2FB,
    ext_fns: raw::FaceTracking2FB,
}

impl FaceTracker2FB {
    pub fn get_face_expression_weights(&self, time: xr::Time) -> Result<Option<Vec<f32>>> {
        let expression_info = sys::FaceExpressionInfo2FB {
            ty: sys::FaceExpressionInfo2FB::TYPE,
            next: ptr::null(),
            time,
        };

        let weights_count = xr::FaceExpression2FB::COUNT.into_raw() as usize;
        let confidence_count = xr::FaceConfidence2FB::COUNT.into_raw() as usize;

        let mut weights = Vec::with_capacity(weights_count);
        let mut confidences = vec![0.0; confidence_count];

        let mut expression_weights = sys::FaceExpressionWeights2FB {
            ty: sys::FaceExpressionWeights2FB::TYPE,
            next: ptr::null_mut(),
            weight_count: weights_count as u32,
            weights: weights.as_mut_ptr() as _,
            confidence_count: confidence_count as u32,
            confidences: confidences.as_mut_ptr() as _,
            is_valid: sys::FALSE,
            is_eye_following_blendshapes_valid: sys::FALSE,
            data_source: sys::FaceTrackingDataSource2FB::from_raw(0),
            time: xr::Time::from_nanos(0),
        };

        unsafe {
            super::to_any((self.ext_fns.get_face_expression_weights2)(
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
