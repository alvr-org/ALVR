#![allow(unused_variables)]

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
