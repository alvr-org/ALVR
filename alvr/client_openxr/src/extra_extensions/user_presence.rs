use openxr::{self as xr, sys};
use std::ptr;

pub fn supports_user_presence<G>(session: &xr::Session<G>, system: xr::SystemId) -> bool {
    if session.instance().exts().ext_user_presence.is_none() {
        return false;
    }

    super::get_props(
        session,
        system,
        sys::SystemUserPresencePropertiesEXT {
            ty: sys::SystemUserPresencePropertiesEXT::TYPE,
            next: ptr::null_mut(),
            supports_user_presence: sys::FALSE,
        },
    )
    .map(|props| props.supports_user_presence.into())
    .unwrap_or(false)
}
