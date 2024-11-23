use alvr_client_core::graphics::{self, GraphicsContext};
use alvr_common::glam::UVec2;
use openxr as xr;

#[allow(unused)]
pub fn session_create_info(ctx: &GraphicsContext) -> xr::opengles::SessionCreateInfo {
    #[cfg(target_os = "android")]
    {
        xr::opengles::SessionCreateInfo::Android {
            display: ctx.egl_display.as_ptr(),
            config: ctx.egl_config.as_ptr(),
            context: ctx.egl_context.as_ptr(),
        }
    }
    #[cfg(not(target_os = "android"))]
    unimplemented!()
}

pub fn swapchain_format(
    gfx_ctx: &GraphicsContext,
    session: &xr::Session<xr::OpenGlEs>,
    enable_hdr: bool,
) -> u32 {
    gfx_ctx.make_current();

    let formats = session.enumerate_swapchain_formats().unwrap();
    graphics::choose_swapchain_format(&formats, enable_hdr)
}

#[allow(unused_variables)]
pub fn create_swapchain(
    session: &xr::Session<xr::OpenGlEs>,
    gfx_ctx: &GraphicsContext,
    resolution: UVec2,
    format: u32,
    foveation: Option<&xr::FoveationProfileFB>,
) -> xr::Swapchain<xr::OpenGlEs> {
    gfx_ctx.make_current();

    let swapchain_info = xr::SwapchainCreateInfo {
        create_flags: xr::SwapchainCreateFlags::EMPTY,
        usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT | xr::SwapchainUsageFlags::SAMPLED,
        format,
        sample_count: 1,
        width: resolution.x,
        height: resolution.y,
        face_count: 1,
        array_size: 1,
        mip_count: 1,
    };

    session.create_swapchain(&swapchain_info).unwrap()
}

pub struct ProjectionLayerAlphaConfig {
    pub premultiplied: bool,
}

// This is needed to work around lifetime limitations. Deref cannot be implemented because there are
// nested references, and in a way or the other I would get `cannot return reference to temporary
// value`
pub struct ProjectionLayerBuilder<'a> {
    reference_space: &'a xr::Space,
    layers: [xr::CompositionLayerProjectionView<'a, xr::OpenGlEs>; 2],
    alpha: Option<ProjectionLayerAlphaConfig>,
}

impl<'a> ProjectionLayerBuilder<'a> {
    pub fn new(
        reference_space: &'a xr::Space,
        layers: [xr::CompositionLayerProjectionView<'a, xr::OpenGlEs>; 2],
        alpha: Option<ProjectionLayerAlphaConfig>,
    ) -> Self {
        Self {
            reference_space,
            layers,
            alpha,
        }
    }

    pub fn build(&self) -> xr::CompositionLayerProjection<xr::OpenGlEs> {
        let mut flags = xr::CompositionLayerFlags::EMPTY;

        if let Some(alpha) = &self.alpha {
            flags |= xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA;

            if !alpha.premultiplied {
                flags |= xr::CompositionLayerFlags::UNPREMULTIPLIED_ALPHA;
            }
        }

        xr::CompositionLayerProjection::new()
            .layer_flags(flags)
            .space(self.reference_space)
            .views(&self.layers)
    }
}
