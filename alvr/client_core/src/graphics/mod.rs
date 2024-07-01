mod lobby;
mod opengl;
mod stream;

use std::rc::Rc;

pub use lobby::*;
pub use opengl::choose_swapchain_format;
pub use stream::*;

use alvr_common::{glam::UVec2, Fov, Pose};
use glow::{self as gl, HasContext};
use khronos_egl::{self as egl, EGL1_4};

macro_rules! ck {
    ($gl_ctx:ident.$($gl_cmd:tt)*) => {{
        let res = $gl_ctx.$($gl_cmd)*;

        #[cfg(debug_assertions)]
        {
            let err = $gl_ctx.get_error();
            if err != glow::NO_ERROR {
                alvr_common::error!("gl error at {}:{}: {} -> {err}", file!(), line!(), stringify!($($gl_cmd)*));
                std::process::abort();
            }
        }

        res
    }};
}
pub(crate) use ck;

fn create_texture(gl: &gl::Context, resolution: UVec2, internal_format: u32) -> gl::Texture {
    unsafe {
        let texture = gl.create_texture().unwrap();
        ck!(gl.bind_texture(gl::TEXTURE_2D, Some(texture)));

        ck!(gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            internal_format as i32,
            resolution.x as i32,
            resolution.y as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            Some(&vec![255; 4 * (resolution.x * resolution.y) as usize]),
        ));
        ck!(gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32));
        ck!(gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32));
        ck!(gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32));
        ck!(gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32));

        texture
    }
}

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

struct RenderTarget {
    graphics_context: Rc<GraphicsContext>,
    framebuffer: gl::Framebuffer,
}

impl RenderTarget {
    fn new(context: Rc<GraphicsContext>, texture: gl::Texture) -> Self {
        let gl = &context.gl_context;
        unsafe {
            let framebuffer = ck!(gl.create_framebuffer().unwrap());
            ck!(gl.bind_framebuffer(gl::DRAW_FRAMEBUFFER, Some(framebuffer)));
            ck!(gl.framebuffer_texture_2d(
                gl::DRAW_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                Some(texture),
                0,
            ));
            ck!(gl.bind_framebuffer(gl::FRAMEBUFFER, None));

            Self {
                graphics_context: context,
                framebuffer,
            }
        }
    }

    fn bind(&self) {
        unsafe {
            self.graphics_context
                .gl_context
                .bind_framebuffer(gl::DRAW_FRAMEBUFFER, Some(self.framebuffer));
        }
    }
}

impl Drop for RenderTarget {
    fn drop(&mut self) {
        unsafe {
            self.graphics_context
                .gl_context
                .delete_framebuffer(self.framebuffer);
        }
    }
}

pub struct RenderViewInput {
    pub pose: Pose,
    pub fov: Fov,
    pub swapchain_index: u32,
}

pub struct GraphicsContext {
    _instance: egl::DynamicInstance<EGL1_4>,
    pub egl_display: egl::Display,
    pub egl_config: egl::Config,
    pub egl_context: egl::Context,
    _dummy_surface: egl::Surface,
    pub gl_context: gl::Context,
}

impl GraphicsContext {
    pub fn new() -> Self {
        let instance = unsafe { egl::DynamicInstance::<EGL1_4>::load_required().unwrap() };

        let display = unsafe { instance.get_display(egl::DEFAULT_DISPLAY).unwrap() };

        let _ = instance.initialize(display).unwrap();

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
        let egl_context = instance
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
                Some(egl_context),
            )
            .unwrap();

        #[cfg(target_os = "android")]
        unsafe {
            opengl::initGraphicsNative();
        }

        let gl_context = unsafe {
            gl::Context::from_loader_function(|s| {
                instance
                    .get_proc_address(s)
                    .map(|f| f as *const _)
                    .unwrap_or(std::ptr::null())
            })
        };

        Self {
            _instance: instance,
            egl_display: display,
            egl_config: config,
            egl_context,
            _dummy_surface: dummy_surface,
            gl_context,
        }
    }
}

impl Default for GraphicsContext {
    fn default() -> Self {
        Self::new()
    }
}
