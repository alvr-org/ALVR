use super::{ck, GraphicsContext};
use crate::graphics::GL_TEXTURE_EXTERNAL_OES;
use alvr_common::glam::{IVec2, UVec2};
use glow::{self as gl, HasContext};
use std::{ffi::c_void, rc::Rc};

fn create_program(
    gl: &gl::Context,
    vertex_shader_source: &str,
    fragment_shader_source: &str,
) -> gl::Program {
    unsafe {
        let vertex_shader = ck!(gl.create_shader(gl::VERTEX_SHADER).unwrap());
        ck!(gl.shader_source(vertex_shader, vertex_shader_source));
        ck!(gl.compile_shader(vertex_shader));
        if !gl.get_shader_compile_status(vertex_shader) {
            panic!(
                "Failed to compile vertex shader: {}",
                gl.get_shader_info_log(vertex_shader)
            );
        }

        let fragment_shader = ck!(gl.create_shader(gl::FRAGMENT_SHADER).unwrap());
        ck!(gl.shader_source(fragment_shader, fragment_shader_source));
        ck!(gl.compile_shader(fragment_shader));
        if !gl.get_shader_compile_status(fragment_shader) {
            panic!(
                "Failed to compile fragment shader: {}",
                gl.get_shader_info_log(fragment_shader)
            );
        }

        let program = ck!(gl.create_program().unwrap());
        ck!(gl.attach_shader(program, vertex_shader));
        ck!(gl.attach_shader(program, fragment_shader));
        ck!(gl.link_program(program));
        if !gl.get_program_link_status(program) {
            panic!(
                "Failed to link program: {}",
                gl.get_program_info_log(program)
            );
        }

        ck!(gl.delete_shader(vertex_shader));
        ck!(gl.delete_shader(fragment_shader));

        program
    }
}

pub struct StagingRenderer {
    context: Rc<GraphicsContext>,
    program: gl::Program,
    surface_texture: gl::Texture,
    framebuffer: gl::Framebuffer,
    viewport_size: IVec2,
}

impl StagingRenderer {
    pub fn new(
        context: Rc<GraphicsContext>,
        staging_texture: gl::Texture,
        resolution: UVec2,
    ) -> Self {
        let gl = &context.gl_context;
        context.make_current();

        let program = create_program(
            gl,
            include_str!("../../resources/staging_vertex.glsl"),
            include_str!("../../resources/staging_fragment.glsl"),
        );

        unsafe {
            // This is an external surface and storage should not be initialized
            let surface_texture = ck!(gl.create_texture().unwrap());

            let framebuffer = ck!(gl.create_framebuffer().unwrap());
            ck!(gl.bind_framebuffer(gl::DRAW_FRAMEBUFFER, Some(framebuffer)));
            ck!(gl.framebuffer_texture_2d(
                gl::DRAW_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                Some(staging_texture),
                0,
            ));
            ck!(gl.bind_framebuffer(gl::FRAMEBUFFER, None));

            Self {
                context,
                program,
                surface_texture,
                framebuffer,
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

                ck!(gl.bind_framebuffer(gl::DRAW_FRAMEBUFFER, Some(self.framebuffer)));

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
            ck!(gl.delete_framebuffer(self.framebuffer));
        }
    }
}
