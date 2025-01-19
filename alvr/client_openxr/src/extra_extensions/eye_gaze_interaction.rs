use openxr::{self as xr, sys};
use std::ptr;

pub fn supports_eye_gaze_interaction<G>(session: &xr::Session<G>, system: xr::SystemId) -> bool {
    if session.instance().exts().ext_eye_gaze_interaction.is_none() {
        return false;
    }

    super::get_props(
        session,
        system,
        sys::SystemEyeGazeInteractionPropertiesEXT {
            ty: sys::SystemEyeGazeInteractionPropertiesEXT::TYPE,
            next: ptr::null_mut(),
            supports_eye_gaze_interaction: sys::FALSE,
        },
    )
    .map(|props| props.supports_eye_gaze_interaction.into())
    .unwrap_or(false)
}
