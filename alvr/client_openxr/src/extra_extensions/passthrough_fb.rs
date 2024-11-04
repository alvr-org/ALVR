use openxr::{self as xr, raw, sys};
use std::ptr;

pub struct PassthroughFB {
    handle: sys::PassthroughFB,
    layer_handle: sys::PassthroughLayerFB,
    layer: sys::CompositionLayerPassthroughFB,
    ext_fns: raw::PassthroughFB,
}

impl PassthroughFB {
    pub fn new(session: &xr::Session<xr::OpenGlEs>) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .fb_passthrough
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let mut handle = sys::PassthroughFB::NULL;
        let info = sys::PassthroughCreateInfoFB {
            ty: sys::PassthroughCreateInfoFB::TYPE,
            next: ptr::null(),
            flags: sys::PassthroughFlagsFB::IS_RUNNING_AT_CREATION,
        };
        unsafe {
            super::xr_res((ext_fns.create_passthrough)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        let mut layer_handle = sys::PassthroughLayerFB::NULL;
        let info = sys::PassthroughLayerCreateInfoFB {
            ty: sys::PassthroughLayerCreateInfoFB::TYPE,
            next: ptr::null(),
            passthrough: handle,
            flags: sys::PassthroughFlagsFB::IS_RUNNING_AT_CREATION,
            purpose: sys::PassthroughLayerPurposeFB::RECONSTRUCTION,
        };
        unsafe {
            super::xr_res((ext_fns.create_passthrough_layer)(
                session.as_raw(),
                &info,
                &mut layer_handle,
            ))?
        };

        let layer = sys::CompositionLayerPassthroughFB {
            ty: sys::CompositionLayerPassthroughFB::TYPE,
            next: ptr::null(),
            flags: xr::CompositionLayerFlags::EMPTY,
            space: sys::Space::NULL,
            layer_handle,
        };

        Ok(Self {
            handle,
            layer_handle,
            layer,
            ext_fns,
        })
    }

    // return reference to make sure the passthrough handle is not dropped while the layer is in use
    pub fn layer(&self) -> &sys::CompositionLayerPassthroughFB {
        &self.layer
    }
}

impl Drop for PassthroughFB {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_passthrough_layer)(self.layer_handle);
            (self.ext_fns.destroy_passthrough)(self.handle);
        }
    }
}
