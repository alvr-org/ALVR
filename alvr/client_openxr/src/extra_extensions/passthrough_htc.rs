use openxr::{
    self as xr, raw,
    sys::{self, Handle},
};
use std::ptr;

pub struct PassthroughHTC {
    handle: sys::PassthroughHTC,
    layer: sys::CompositionLayerPassthroughHTC,
    ext_fns: raw::PassthroughHTC,
}

impl PassthroughHTC {
    pub fn new(session: &xr::Session<xr::OpenGlEs>) -> xr::Result<Self> {
        let ext_fns = session
            .instance()
            .exts()
            .htc_passthrough
            .ok_or(sys::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let mut handle = sys::PassthroughHTC::NULL;
        let info = sys::PassthroughCreateInfoHTC {
            ty: sys::PassthroughCreateInfoHTC::TYPE,
            next: ptr::null(),
            form: sys::PassthroughFormHTC::PLANAR,
        };
        unsafe {
            super::xr_res((ext_fns.create_passthrough)(
                session.as_raw(),
                &info,
                &mut handle,
            ))?
        };

        let layer = sys::CompositionLayerPassthroughHTC {
            ty: sys::CompositionLayerPassthroughHTC::TYPE,
            next: ptr::null(),
            layer_flags: xr::CompositionLayerFlags::EMPTY,
            space: sys::Space::NULL,
            passthrough: handle,
            color: sys::PassthroughColorHTC {
                ty: sys::PassthroughColorHTC::TYPE,
                next: ptr::null(),
                alpha: 1.0,
            },
        };

        Ok(Self {
            handle,
            layer,
            ext_fns,
        })
    }

    // return reference to make sure the passthrough handle is not dropped while the layer is in use
    pub fn layer(&self) -> &sys::CompositionLayerPassthroughHTC {
        &self.layer
    }
}

impl Drop for PassthroughHTC {
    fn drop(&mut self) {
        unsafe {
            (self.ext_fns.destroy_passthrough)(self.handle);
        }
    }
}
