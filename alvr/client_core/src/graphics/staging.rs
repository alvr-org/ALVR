use super::{ck, GraphicsContext, RenderTarget};
use crate::graphics::GL_TEXTURE_EXTERNAL_OES;
use alvr_common::glam::{IVec2, UVec2};
use glow::{self as gl, HasContext};
use std::{ffi::c_void, rc::Rc};

pub struct StagingRenderer {
    context: Rc<GraphicsContext>,
    program: gl::Program,
    surface_texture: gl::Texture,
    render_target: RenderTarget,
    viewport_size: IVec2,
}

impl StagingRenderer {
    pub fn new(
        context: Rc<GraphicsContext>,
        staging_texture: gl::Texture,
        resolution: UVec2,
    ) -> Self {
        let gl = &context.gl_context;

        let render_target = RenderTarget::new(Rc::clone(&context), staging_texture);

        let program = super::create_program(
            gl,
            include_str!("../../resources/staging_vertex.glsl"),
            include_str!("../../resources/staging_fragment.glsl"),
        );

        unsafe {
            // This is an external surface and storage should not be initialized
            let surface_texture = ck!(gl.create_texture().unwrap());

            Self {
                context,
                program,
                surface_texture,
                render_target,
                viewport_size: resolution.as_ivec2(),
            }
        }
    }

    #[allow(unused_variables)]
    pub unsafe fn render(&self, hardware_buffer: *mut c_void) {
        let gl = &self.context.gl_context;

        self.context.render_ahardwarebuffer_using_texture(
            hardware_buffer,
            self.surface_texture,
            || unsafe {
                ck!(gl.use_program(Some(self.program)));

                ck!(gl.viewport(0, 0, self.viewport_size.x, self.viewport_size.y));

                self.render_target.bind();

                ck!(gl.active_texture(gl::TEXTURE0));
                ck!(gl.bind_texture(GL_TEXTURE_EXTERNAL_OES, Some(self.surface_texture)));
                ck!(gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4));
            },
        );
    }
}

impl Drop for StagingRenderer {
    fn drop(&mut self) {
        let gl = &self.context.gl_context;
        unsafe {
            ck!(gl.delete_program(self.program));
            ck!(gl.delete_texture(self.surface_texture));
        }
    }
}
