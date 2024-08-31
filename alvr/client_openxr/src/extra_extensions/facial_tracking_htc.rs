use alvr_common::{anyhow::Result, ToAny};
use openxr::{self as xr, raw, sys};
use std::ptr;

impl super::ExtraExtensions {
    pub fn supports_htc_eye_facial_tracking(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(instance, system, unsafe {
            sys::SystemFacialTrackingPropertiesHTC::out(ptr::null_mut()).assume_init()
        })
        .map(|props| props.support_eye_facial_tracking.into())
        .unwrap_or(false)
    }

    pub fn supports_htc_lip_facial_tracking(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> bool {
        self.get_props(instance, system, unsafe {
            sys::SystemFacialTrackingPropertiesHTC::out(ptr::null_mut()).assume_init()
        })
        .map(|props| props.support_lip_facial_tracking.into())
        .unwrap_or(false)
    }

    pub fn create_facial_tracker_htc<G>(
        &self,
        session: &xr::Session<G>,
        facial_tracking_type: xr::FacialTrackingTypeHTC,
    ) -> Result<FacialTrackerHTC> {
        let ext_fns = self.ext_functions_ptrs.htc_facial_tracking.to_any()?;

        let mut handle = sys::FacialTrackerHTC::NULL;
        let info = sys::FacialTrackerCreateInfoHTC {
            ty: sys::FacialTrackerCreateInfoHTC::TYPE,
            next: ptr::null(),
            facial_tracking_type,
        };
        unsafe {
            super::to_any((ext_fns.create_facial_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        let expression_count = if facial_tracking_type == sys::FacialTrackingTypeHTC::EYE_DEFAULT {
            sys::FACIAL_EXPRESSION_EYE_COUNT_HTC
        } else {
            sys::FACIAL_EXPRESSION_LIP_COUNT_HTC
        };

        Ok(FacialTrackerHTC {
            handle,
            ext_fns,
            expression_count,
        })
    }
}

pub struct FacialTrackerHTC {
    handle: sys::FacialTrackerHTC,
    ext_fns: raw::FacialTrackingHTC,
    expression_count: usize,
}

impl FacialTrackerHTC {
    pub fn get_facial_expressions(&self) -> Result<Option<Vec<f32>>> {
        let mut weights = Vec::with_capacity(self.expression_count);

        let mut facial_expressions = sys::FacialExpressionsHTC {
            ty: sys::FacialExpressionsHTC::TYPE,
            next: ptr::null_mut(),
            is_active: sys::FALSE,
            sample_time: xr::Time::from_nanos(0),
            expression_count: self.expression_count as u32,
            expression_weightings: weights.as_mut_ptr(),
        };

        unsafe {
            super::to_any((self.ext_fns.get_facial_expressions)(
                self.handle,
                &mut facial_expressions,
            ))?;

            if facial_expressions.is_active.into() {
                Ok(Some(weights))
            } else {
                Ok(None)
            }
        }
    }
}

impl Drop for FacialTrackerHTC {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_facial_tracker)(self.handle);
        }
    }
}
