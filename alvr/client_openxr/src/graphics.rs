use alvr_common::glam::UVec2;
use khronos_egl::{self as egl, EGL1_4};
use openxr as xr;

#[allow(unused)]
pub struct EglContext {
    instance: egl::DynamicInstance<EGL1_4>,
    display: egl::Display,
    config: egl::Config,
    context: egl::Context,
    dummy_surface: egl::Surface,
}

impl EglContext {
    pub fn session_create_info(&self) -> xr::opengles::SessionCreateInfo {
        #[cfg(target_os = "android")]
        {
            xr::opengles::SessionCreateInfo::Android {
                display: self.display.as_ptr(),
                config: self.config.as_ptr(),
                context: self.context.as_ptr(),
            }
        }
        #[cfg(not(target_os = "android"))]
        unimplemented!()
    }
}

#[allow(unused_variables)]
pub fn init_egl() -> EglContext {
    let instance = unsafe { egl::DynamicInstance::<EGL1_4>::load_required().unwrap() };

    let display = unsafe { instance.get_display(egl::DEFAULT_DISPLAY).unwrap() };

    let version = instance.initialize(display).unwrap();

    let mut configs = Vec::with_capacity(instance.get_config_count(display).unwrap());
    instance.get_configs(display, &mut configs).unwrap();

    const CONFIG_ATTRIBS: [i32; 19] = [
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::ALPHA_SIZE,
        8,
        egl::DEPTH_SIZE,
        0,
        egl::STENCIL_SIZE,
        0,
        egl::SAMPLES,
        0,
        egl::SURFACE_TYPE,
        egl::PBUFFER_BIT,
        egl::RENDERABLE_TYPE,
        egl::OPENGL_ES3_BIT,
        egl::NONE,
    ];
    let config = instance
        .choose_first_config(display, &CONFIG_ATTRIBS)
        .unwrap()
        .unwrap();

    instance.bind_api(egl::OPENGL_ES_API).unwrap();

    const CONTEXT_ATTRIBS: [i32; 3] = [egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE];
    let context = instance
        .create_context(display, config, None, &CONTEXT_ATTRIBS)
        .unwrap();

    const PBUFFER_ATTRIBS: [i32; 5] = [egl::WIDTH, 16, egl::HEIGHT, 16, egl::NONE];
    let dummy_surface = instance
        .create_pbuffer_surface(display, config, &PBUFFER_ATTRIBS)
        .unwrap();

    instance
        .make_current(
            display,
            Some(dummy_surface),
            Some(dummy_surface),
            Some(context),
        )
        .unwrap();

    EglContext {
        instance,
        display,
        config,
        context,
        dummy_surface,
    }
}

pub fn create_swapchain(
    session: &xr::Session<xr::OpenGlEs>,
    resolution: UVec2,
    foveation: Option<&xr::FoveationProfileFB>,
    enable_hdr: bool,
) -> xr::Swapchain<xr::OpenGlEs> {
    // Priority-sorted list of swapchain formats we'll accept--
    let mut app_supported_swapchain_formats = vec![
        glow::SRGB8_ALPHA8,
        glow::SRGB8,
        glow::RGBA8,
        glow::BGRA,
        glow::RGB8,
        glow::BGR,
    ];

    // float16 is required for HDR output. However, float16 swapchains
    // have a high perf cost, so only use these if HDR is enabled.
    if enable_hdr {
        app_supported_swapchain_formats.insert(0, glow::RGB16F);
        app_supported_swapchain_formats.insert(0, glow::RGBA16F);
    }

    // If we can't enumerate, default to a required format (SRGBA8)
    let mut swapchain_format = glow::SRGB8_ALPHA8;
    if let Ok(supported_formats) = session.enumerate_swapchain_formats() {
        for f in app_supported_swapchain_formats {
            if supported_formats.contains(&f) {
                swapchain_format = f;
                break;
            }
        }
    }

    let swapchain_info = xr::SwapchainCreateInfo {
        create_flags: xr::SwapchainCreateFlags::EMPTY,
        usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT | xr::SwapchainUsageFlags::SAMPLED,
        format: swapchain_format,
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
