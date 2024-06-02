use super::GraphicsContext;
use alvr_common::glam::UVec2;
use alvr_session::FoveatedEncodingConfig;
use std::rc::Rc;

pub struct StreamRenderer {
    _context: Rc<GraphicsContext>,
}

impl StreamRenderer {
    #[allow(unused_variables)]
    pub fn new(
        context: Rc<GraphicsContext>,
        view_resolution: UVec2,
        swapchain_textures: [Vec<u32>; 2],
        foveated_encoding: Option<FoveatedEncodingConfig>,
        enable_srgb_correction: bool,
        fix_limited_range: bool,
        encoding_gamma: f32,
    ) -> Self {
        #[cfg(target_os = "android")]
        unsafe {
            let config = super::opengl::FfiStreamConfig {
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

            super::opengl::streamStartNative(config);
        }

        Self { _context: context }
    }

    #[allow(unused_variables)]
    pub fn render(&self, hardware_buffer: *mut std::ffi::c_void, swapchain_indices: [u32; 2]) {
        #[cfg(target_os = "android")]
        unsafe {
            super::opengl::renderStreamNative(hardware_buffer, swapchain_indices.as_ptr());
        }
    }
}

impl Drop for StreamRenderer {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        unsafe {
            super::opengl::destroyStream();
        }
    }
}
