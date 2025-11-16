use openxr::{
    self as xr, raw,
    sys::{self, Handle},
};
use std::ptr;

pub struct FacialTrackerHTC {
    // Keeping a reference to the session to ensure that the tracker handle remains valid
    _session: xr::Session<xr::AnyGraphics>,
    handle: sys::FacialTrackerHTC,
    ext_fns: raw::FacialTrackingHTC,
    expression_count: usize,
}

impl FacialTrackerHTC {
    pub fn new<G>(
        session: xr::Session<G>,
        system: xr::SystemId,
        facial_tracking_type: xr::FacialTrackingTypeHTC,
    ) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .htc_facial_tracking
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let props = super::get_props(
            &session,
            system,
            sys::SystemFacialTrackingPropertiesHTC {
                ty: sys::SystemFacialTrackingPropertiesHTC::TYPE,
                next: ptr::null_mut(),
                support_eye_facial_tracking: sys::FALSE,
                support_lip_facial_tracking: sys::FALSE,
            },
        )?;

        let expression_count = if facial_tracking_type == sys::FacialTrackingTypeHTC::EYE_DEFAULT
            && props.support_eye_facial_tracking.into()
        {
            sys::FACIAL_EXPRESSION_EYE_COUNT_HTC
        } else if facial_tracking_type == sys::FacialTrackingTypeHTC::LIP_DEFAULT
            && props.support_lip_facial_tracking.into()
        {
            sys::FACIAL_EXPRESSION_LIP_COUNT_HTC
        } else {
            return Err(sys::Result::ERROR_FEATURE_UNSUPPORTED);
        };

        let mut handle = sys::FacialTrackerHTC::NULL;
        let info = sys::FacialTrackerCreateInfoHTC {
            ty: sys::FacialTrackerCreateInfoHTC::TYPE,
            next: ptr::null(),
            facial_tracking_type,
        };
        unsafe {
            super::xr_res((ext_fns.create_facial_tracker)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        Ok(Self {
            _session: session.into_any_graphics(),
            handle,
            ext_fns,
            expression_count,
        })
    }

    pub fn get_facial_expressions(&self, time: xr::Time) -> xr::Result<Option<Vec<f32>>> {
        let mut weights = Vec::with_capacity(self.expression_count);

        let mut facial_expressions = sys::FacialExpressionsHTC {
            ty: sys::FacialExpressionsHTC::TYPE,
            next: ptr::null_mut(),
            is_active: sys::FALSE,
            sample_time: time,
            expression_count: self.expression_count as u32,
            expression_weightings: weights.as_mut_ptr(),
        };

        unsafe {
            super::xr_res((self.ext_fns.get_facial_expressions)(
                self.handle,
                &mut facial_expressions,
            ))?;

            if facial_expressions.is_active.into() {
                weights.set_len(self.expression_count);

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
