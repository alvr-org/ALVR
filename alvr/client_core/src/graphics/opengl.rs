#![allow(unused_variables)]

use alvr_common::glam::UVec2;
use alvr_session::FoveatedEncodingConfig;

#[cfg(target_os = "android")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub fn choose_swapchain_format(formats: Option<&[u32]>, enable_hdr: bool) -> u32 {
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

    if let Some(supported_formats) = formats {
        for format in app_supported_swapchain_formats {
            if supported_formats.contains(&format) {
                return format;
            }
        }
    }

    // If we can't enumerate, default to a required format (SRGBA8)
    glow::SRGB8_ALPHA8
}

pub fn destroy_stream() {
    #[cfg(target_os = "android")]
    unsafe {
        destroyStream();
    }
}

pub fn start_stream(
    view_resolution: UVec2,
    swapchain_textures: [Vec<u32>; 2],
    foveated_encoding: Option<FoveatedEncodingConfig>,
    enable_srgb_correction: bool,
    fix_limited_range: bool,
    encoding_gamma: f32,
) {
    #[cfg(target_os = "android")]
    unsafe {
        let config = FfiStreamConfig {
            viewWidth: view_resolution.x,
            viewHeight: view_resolution.y,
            swapchainTextures: [
                swapchain_textures[0].as_ptr(),
                swapchain_textures[1].as_ptr(),
            ],
            swapchainLength: swapchain_textures[0].len() as _,
            enableFoveation: foveated_encoding.is_some().into(),
            foveationCenterSizeX: foveated_encoding
                .as_ref()
                .map(|f| f.center_size_x)
                .unwrap_or_default(),
            foveationCenterSizeY: foveated_encoding
                .as_ref()
                .map(|f| f.center_size_y)
                .unwrap_or_default(),
            foveationCenterShiftX: foveated_encoding
                .as_ref()
                .map(|f| f.center_shift_x)
                .unwrap_or_default(),
            foveationCenterShiftY: foveated_encoding
                .as_ref()
                .map(|f| f.center_shift_y)
                .unwrap_or_default(),
            foveationEdgeRatioX: foveated_encoding
                .as_ref()
                .map(|f| f.edge_ratio_x)
                .unwrap_or_default(),
            foveationEdgeRatioY: foveated_encoding
                .as_ref()
                .map(|f| f.edge_ratio_y)
                .unwrap_or_default(),
            enableSrgbCorrection: enable_srgb_correction as u32,
            fixLimitedRange: fix_limited_range as u32,
            encodingGamma: encoding_gamma,
        };

        streamStartNative(config);
    }
}

pub fn render_stream(hardware_buffer: *mut std::ffi::c_void, swapchain_indices: [u32; 2]) {
    #[cfg(target_os = "android")]
    unsafe {
        renderStreamNative(hardware_buffer, swapchain_indices.as_ptr());
    }
}
