use openxr::{self as xr, sys};
use std::ptr;

fn get_props<G, T>(session: &xr::Session<G>, system: xr::SystemId, default_struct: T) -> Option<T> {
    let instance = session.instance();

    let mut props = default_struct;
    let mut system_properties = sys::SystemProperties::out((&mut props as *mut T).cast());
    let result = unsafe {
        (instance.fp().get_system_properties)(
            instance.as_raw(),
            system,
            system_properties.as_mut_ptr(),
        )
    };
    (result.into_raw() >= 0).then_some(props)
}

pub fn supports_eye_gaze_interaction<G>(session: &xr::Session<G>, system: xr::SystemId) -> bool {
    if session.instance().exts().ext_eye_gaze_interaction.is_none() {
        return false;
    }

    get_props(
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
