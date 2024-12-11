use crate::extra_extensions::{PassthroughFB, PassthroughHTC};
use alvr_common::anyhow::{bail, Result};
use openxr::{
    self as xr,
    sys::{CompositionLayerPassthroughFB, CompositionLayerPassthroughHTC},
};
use std::{marker::PhantomData, mem, ops::Deref};

pub struct PassthroughLayer<'a> {
    handle_fb: Option<PassthroughFB>,
    handle_htc: Option<PassthroughHTC>,
    _marker: PhantomData<&'a ()>,
}

impl PassthroughLayer<'_> {
    pub fn new(session: &xr::Session<xr::OpenGlEs>) -> Result<Self> {
        let mut handle_fb = None;
        let mut handle_htc = None;

        let exts = session.instance().exts();
        if exts.fb_passthrough.is_some() {
            handle_fb = Some(PassthroughFB::new(session)?);
        } else if exts.htc_passthrough.is_some() {
            handle_htc = Some(PassthroughHTC::new(session)?);
        } else {
            bail!("No passthrough extension available");
        };

        Ok(Self {
            handle_fb,
            handle_htc,
            _marker: PhantomData,
        })
    }
}

impl<'a> Deref for PassthroughLayer<'a> {
    type Target = xr::CompositionLayerBase<'a, xr::OpenGlEs>;

    fn deref(&self) -> &Self::Target {
        if let Some(handle) = &self.handle_fb {
            unsafe {
                mem::transmute::<&CompositionLayerPassthroughFB, &Self::Target>(handle.layer())
            }
        } else if let Some(handle) = &self.handle_htc {
            unsafe {
                mem::transmute::<&CompositionLayerPassthroughHTC, &Self::Target>(handle.layer())
            }
        } else {
            panic!("No passthrough extension available");
        }
    }
}
