use super::{ck, staging::StagingRenderer, GraphicsContext, RenderTarget};
use alvr_common::glam::{IVec2, UVec2};
use alvr_session::FoveatedEncodingConfig;
use glow::{self as gl, HasContext};
use std::{ffi::c_void, num::NonZeroU32, rc::Rc};

struct GlObjects {
    staging_renderer: StagingRenderer,
    staging_texture: gl::Texture,
    render_targets: [Vec<RenderTarget>; 2],
    program: gl::Program,
    view_idx_uloc: gl::UniformLocation,
    viewport_size: IVec2,
}

pub struct StreamRenderer {
    context: Rc<GraphicsContext>,
    gl_objects: Option<GlObjects>,
}

impl StreamRenderer {
    pub fn new(
        context: Rc<GraphicsContext>,
        view_resolution: UVec2,
        swapchain_textures: [Vec<u32>; 2],
        foveated_encoding: Option<FoveatedEncodingConfig>,
        enable_srgb_correction: bool,
        fix_limited_range: bool,
        encoding_gamma: f32,
    ) -> Self {
        // if ffe is enabled, use old c++ code until it is rewritten
        #[allow(unused_variables)]
        if let Some(fe) = &foveated_encoding {
            #[cfg(all(target_os = "android", feature = "use-cpp"))]
            unsafe {
                let config = super::opengl::FfiStreamConfig {
                    viewWidth: view_resolution.x,
                    viewHeight: view_resolution.y,
                    swapchainTextures: [
                        swapchain_textures[0].as_ptr(),
                        swapchain_textures[1].as_ptr(),
                    ],
                    swapchainLength: swapchain_textures[0].len() as _,
                    enableFoveation: 1,
                    foveationCenterSizeX: fe.center_size_x,
                    foveationCenterSizeY: fe.center_size_y,
                    foveationCenterShiftX: fe.center_shift_x,
                    foveationCenterShiftY: fe.center_shift_y,
                    foveationEdgeRatioX: fe.edge_ratio_x,
                    foveationEdgeRatioY: fe.edge_ratio_y,
                    enableSrgbCorrection: enable_srgb_correction as u32,
                    fixLimitedRange: fix_limited_range as u32,
                    encodingGamma: encoding_gamma,
                };

                super::opengl::streamStartNative(config);
            }

            Self {
                context,
                gl_objects: None,
            }
        } else {
            let gl = &context.gl_context;

            let staging_texture = super::create_texture(gl, view_resolution, gl::RGBA8);

            let render_targets = [
                swapchain_textures[0]
                    .iter()
                    .map(|tex| {
                        RenderTarget::new(
                            Rc::clone(&context),
                            gl::NativeTexture(NonZeroU32::new(*tex).unwrap()),
                        )
                    })
                    .collect(),
                swapchain_textures[1]
                    .iter()
                    .map(|tex| {
                        RenderTarget::new(
                            Rc::clone(&context),
                            gl::NativeTexture(NonZeroU32::new(*tex).unwrap()),
                        )
                    })
                    .collect(),
            ];

            let program = super::create_program(
                gl,
                include_str!("../../resources/stream_vertex.glsl"),
                include_str!("../../resources/stream_fragment.glsl"),
            );

            let staging_renderer =
                StagingRenderer::new(Rc::clone(&context), staging_texture, view_resolution);

            unsafe {
                let view_idx_uloc = ck!(gl.get_uniform_location(program, "view_idx").unwrap());

                let enable_srgb_correction_uloc = ck!(gl
                    .get_uniform_location(program, "enable_srgb_correction")
                    .unwrap());
                let fix_limited_range_uloc = ck!(gl
                    .get_uniform_location(program, "fix_limited_range")
                    .unwrap());
                let encoding_gamma_uloc =
                    ck!(gl.get_uniform_location(program, "encoding_gamma").unwrap());

                ck!(gl.use_program(Some(program)));
                ck!(gl.uniform_1_i32(
                    Some(&enable_srgb_correction_uloc),
                    enable_srgb_correction as i32
                ));
                ck!(gl.uniform_1_i32(Some(&fix_limited_range_uloc), fix_limited_range as i32));
                ck!(gl.uniform_1_f32(Some(&encoding_gamma_uloc), encoding_gamma));

                Self {
                    context,
                    gl_objects: Some(GlObjects {
                        staging_renderer,
                        staging_texture,
                        render_targets,
                        program,
                        view_idx_uloc,
                        viewport_size: view_resolution.as_ivec2(),
                    }),
                }
            }
        }
    }

    pub fn staging_texture(&self) -> Option<gl::Texture> {
        self.gl_objects
            .as_ref()
            .map(|gl_objects| gl_objects.staging_texture)
    }

    #[allow(unused_variables)]
    pub unsafe fn render(&self, hardware_buffer: *mut c_void, swapchain_indices: [u32; 2]) {
        if let Some(gl_objects) = &self.gl_objects {
            let gl = &self.context.gl_context;

            // if hardware_buffer is available copy stream to staging texture
            if !hardware_buffer.is_null() {
                gl_objects.staging_renderer.render(hardware_buffer);
            }

            unsafe {
                ck!(gl.use_program(Some(gl_objects.program)));

                ck!(gl.disable(gl::SCISSOR_TEST));
                ck!(gl.disable(gl::DEPTH_TEST));
                ck!(gl.disable(gl::CULL_FACE));
                ck!(gl.disable(gl::BLEND));

                ck!(gl.viewport(0, 0, gl_objects.viewport_size.x, gl_objects.viewport_size.y));

                for (view_idx, swapchain_idx) in swapchain_indices.iter().enumerate() {
                    gl_objects.render_targets[view_idx][*swapchain_idx as usize].bind();

                    ck!(gl.clear(gl::COLOR_BUFFER_BIT));
                    ck!(gl.clear_color(0.0, 0.0, 0.0, 1.0));

                    ck!(gl.uniform_1_i32(Some(&gl_objects.view_idx_uloc), view_idx as i32));
                    ck!(gl.active_texture(gl::TEXTURE0));
                    ck!(gl.bind_texture(gl::TEXTURE_2D, Some(gl_objects.staging_texture)));
                    ck!(gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4));
                }
            }
        } else {
            #[cfg(all(target_os = "android", feature = "use-cpp"))]
            super::opengl::renderStreamNative(hardware_buffer, swapchain_indices.as_ptr());
        }
    }
}

impl Drop for StreamRenderer {
    fn drop(&mut self) {
        if let Some(gl_objects) = &self.gl_objects {
            unsafe {
                let gl = &self.context.gl_context;

                ck!(gl.delete_program(gl_objects.program));
                ck!(gl.delete_texture(gl_objects.staging_texture));
            }
        } else {
            #[cfg(all(target_os = "android", feature = "use-cpp"))]
            unsafe {
                super::opengl::destroyStream();
            }
        }
    }
}
