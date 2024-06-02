use alvr_client_core::graphics::{self, GraphicsContext};
use alvr_common::glam::UVec2;
use openxr as xr;

#[allow(unused)]
pub fn session_create_info(ctx: &GraphicsContext) -> xr::opengles::SessionCreateInfo {
    #[cfg(target_os = "android")]
    {
        xr::opengles::SessionCreateInfo::Android {
            display: ctx.display.as_ptr(),
            config: ctx.config.as_ptr(),
            context: ctx.context.as_ptr(),
        }
    }
    #[cfg(not(target_os = "android"))]
    unimplemented!()
}

pub fn create_swapchain(
    session: &xr::Session<xr::OpenGlEs>,
    resolution: UVec2,
    foveation: Option<&xr::FoveationProfileFB>,
    enable_hdr: bool,
) -> xr::Swapchain<xr::OpenGlEs> {
    let format = graphics::choose_swapchain_format(
        session.enumerate_swapchain_formats().ok().as_deref(),
        enable_hdr,
    );

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

    if let Some(foveation) = foveation {
        let swapchain = session
            .create_swapchain_with_foveation(
                &swapchain_info,
                xr::SwapchainCreateFoveationFlagsFB::SCALED_BIN,
            )
            .unwrap();

        swapchain.update_foveation(foveation).unwrap();

        swapchain
    } else {
        session.create_swapchain(&swapchain_info).unwrap()
    }
}

// This is needed to work around lifetime limitations
pub struct CompositionLayerBuilder<'a> {
    reference_space: &'a xr::Space,
    layers: [xr::CompositionLayerProjectionView<'a, xr::OpenGlEs>; 2],
}

impl<'a> CompositionLayerBuilder<'a> {
    pub fn new(
        reference_space: &'a xr::Space,
        layers: [xr::CompositionLayerProjectionView<'a, xr::OpenGlEs>; 2],
    ) -> Self {
        Self {
            reference_space,
            layers,
        }
    }

    pub fn build(&self) -> xr::CompositionLayerProjection<xr::OpenGlEs> {
        xr::CompositionLayerProjection::new()
            .space(self.reference_space)
            .views(&self.layers)
    }
}
