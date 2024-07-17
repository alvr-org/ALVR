mod lobby;
mod opengl;
mod staging;
mod stream;

pub use lobby::*;
pub use stream::*;

use alvr_common::{glam::UVec2, Fov, Pose};
use glow::{self as gl, HasContext};
use khronos_egl as egl;
use std::{ffi::c_void, mem, num::NonZeroU32, ptr, rc::Rc};
use wgpu::{
    hal::{self, api, MemoryFlags, TextureUses},
    Adapter, Device, Extent3d, Instance, InstanceDescriptor, InstanceFlags, Queue, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
};

pub const GL_TEXTURE_EXTERNAL_OES: u32 = 0x8D65;

type CreateImageFn = unsafe extern "C" fn(
    egl::EGLDisplay,
    egl::EGLContext,
    egl::Enum,
    egl::EGLClientBuffer,
    *const egl::Int,
) -> egl::EGLImage;
type DestroyImageFn = unsafe extern "C" fn(egl::EGLDisplay, egl::EGLImage) -> egl::Boolean;
type GetNativeClientBufferFn = unsafe extern "C" fn(*const c_void) -> egl::EGLClientBuffer;
type ImageTargetTexture2DFn = unsafe extern "C" fn(egl::Enum, egl::EGLImage);

pub fn check_error(gl: &gl::Context, message_context: &str) {
    let err = unsafe { gl.get_error() };
    if err != glow::NO_ERROR {
        alvr_common::error!("gl error {message_context} -> {err}");
        std::process::abort();
    }
}

macro_rules! ck {
    ($gl_ctx:ident.$($gl_cmd:tt)*) => {{
        let res = $gl_ctx.$($gl_cmd)*;

        #[cfg(debug_assertions)]
        crate::graphics::check_error(&$gl_ctx, &format!("{}:{}: {}", file!(), line!(), stringify!($($gl_cmd)*)));

        res
    }};
}
pub(crate) use ck;

pub fn choose_swapchain_format(formats: Option<&[u32]>, enable_hdr: bool) -> u32 {
    // Priority-sorted list of swapchain formats we'll accept--
    let mut app_supported_swapchain_formats = vec![
        gl::SRGB8_ALPHA8,
        gl::SRGB8,
        gl::RGBA8,
        gl::BGRA,
        gl::RGB8,
        gl::BGR,
    ];

    // float16 is required for HDR output. However, float16 swapchains
    // have a high perf cost, so only use these if HDR is enabled.
    if enable_hdr {
        app_supported_swapchain_formats.insert(0, gl::RGB16F);
        app_supported_swapchain_formats.insert(0, gl::RGBA16F);
    }

    if let Some(supported_formats) = formats {
        for format in app_supported_swapchain_formats {
            if supported_formats.contains(&format) {
                return format;
            }
        }
    }

    // If we can't enumerate, default to a required format (SRGBA8)
    gl::SRGB8_ALPHA8
}

pub fn create_texture(device: &Device, resolution: UVec2) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: resolution.x,
            height: resolution.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

// This is used to convert OpenXR swapchains to wgpu
// textures should be arrays of depth 2, RGBA8UnormSrgb
pub fn create_texture_from_gles(device: &Device, texture: u32, resolution: UVec2) -> Texture {
    let size = Extent3d {
        width: resolution.x,
        height: resolution.y,
        depth_or_array_layers: 1,
    };

    unsafe {
        let hal_texture = device
            .as_hal::<api::Gles, _, _>(|device| {
                device.unwrap().texture_from_raw(
                    NonZeroU32::new(texture).unwrap(),
                    &hal::TextureDescriptor {
                        label: None,
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8Unorm,
                        usage: TextureUses::COLOR_TARGET,
                        memory_flags: MemoryFlags::empty(),
                        view_formats: vec![],
                    },
                    Some(Box::new(())),
                )
            })
            .unwrap();

        device.create_texture_from_hal::<api::Gles>(
            hal_texture,
            &TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        )
    }
}

pub fn create_gl_swapchain(
    device: &Device,
    gl_textures: &[u32],
    resolution: UVec2,
) -> Vec<TextureView> {
    gl_textures
        .iter()
        .map(|gl_tex| {
            create_texture_from_gles(device, *gl_tex, resolution).create_view(&Default::default())
        })
        .collect()
}

fn create_gl_texture(gl: &gl::Context, resolution: UVec2, internal_format: u32) -> gl::Texture {
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
            Some(&vec![0; 4 * (resolution.x * resolution.y) as usize]),
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
    _instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    pub egl_display: egl::Display,
    pub egl_config: egl::Config,
    pub egl_context: egl::Context,
    pub gl_context: gl::Context,
    dummy_surface: egl::Surface,
    create_image: CreateImageFn,
    destroy_image: DestroyImageFn,
    get_native_client_buffer: GetNativeClientBufferFn,
    image_target_texture_2d: ImageTargetTexture2DFn,
}

impl GraphicsContext {
    #[cfg(not(windows))]
    pub fn new_gl() -> Self {
        use wgpu::{Backends, DeviceDescriptor, Features, Limits};

        const CREATE_IMAGE_FN_STR: &str = "eglCreateImageKHR";
        const DESTROY_IMAGE_FN_STR: &str = "eglDestroyImageKHR";
        const GET_NATIVE_CLIENT_BUFFER_FN_STR: &str = "eglGetNativeClientBufferANDROID";
        const IMAGE_TARGET_TEXTURE_2D_FN_STR: &str = "glEGLImageTargetTexture2DOES";

        let flags = if cfg!(debug_assertions) {
            InstanceFlags::DEBUG | InstanceFlags::VALIDATION
        } else {
            InstanceFlags::empty()
        };

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::GL,
            flags,
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
        });

        let adapter = instance.enumerate_adapters(Backends::GL).remove(0);
        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::PUSH_CONSTANTS,
                required_limits: Limits {
                    max_push_constant_size: 72,
                    ..Default::default()
                },
            },
            None,
        ))
        .unwrap();

        let raw_instance = unsafe { instance.as_hal::<api::Gles>() }.unwrap();

        let egl_display = raw_instance.raw_display();
        let egl_config = raw_instance.egl_config();

        let (
            egl_context,
            gl_context,
            dummy_surface,
            create_image,
            destroy_image,
            get_native_client_buffer,
            image_target_texture_2d,
        ) = unsafe {
            adapter.as_hal::<api::Gles, _, _>(|raw_adapter| {
                let adapter_context = raw_adapter.unwrap().adapter_context();
                let egl_instance = adapter_context.egl_instance().unwrap();

                let egl_context = egl::Context::from_ptr(adapter_context.raw_context());

                const PBUFFER_ATTRIBS: [i32; 5] = [egl::WIDTH, 16, egl::HEIGHT, 16, egl::NONE];
                let dummy_surface = egl_instance
                    .create_pbuffer_surface(egl_display, egl_config, &PBUFFER_ATTRIBS)
                    .unwrap();

                egl_instance
                    .make_current(
                        egl_display,
                        Some(dummy_surface),
                        Some(dummy_surface),
                        Some(egl_context),
                    )
                    .unwrap();

                let gl_context = gl::Context::from_loader_function(|fn_name| {
                    egl_instance
                        .get_proc_address(fn_name)
                        .map(|f| f as *const c_void)
                        .unwrap_or(ptr::null())
                });

                let get_fn_ptr = |fn_name| {
                    egl_instance
                        .get_proc_address(fn_name)
                        .map(|f| f as *const c_void)
                        .unwrap_or(ptr::null())
                };

                let create_image: CreateImageFn = mem::transmute(get_fn_ptr(CREATE_IMAGE_FN_STR));
                let destroy_image: DestroyImageFn =
                    mem::transmute(get_fn_ptr(DESTROY_IMAGE_FN_STR));
                let get_native_client_buffer: GetNativeClientBufferFn =
                    mem::transmute(get_fn_ptr(GET_NATIVE_CLIENT_BUFFER_FN_STR));
                let image_target_texture_2d: ImageTargetTexture2DFn =
                    mem::transmute(get_fn_ptr(IMAGE_TARGET_TEXTURE_2D_FN_STR));

                (
                    egl_context,
                    gl_context,
                    dummy_surface,
                    create_image,
                    destroy_image,
                    get_native_client_buffer,
                    image_target_texture_2d,
                )
            })
        };

        #[cfg(all(target_os = "android", feature = "use-cpp"))]
        unsafe {
            opengl::initGraphicsNative();
        }

        Self {
            _instance: instance,
            adapter,
            device,
            queue,
            egl_display,
            egl_config,
            egl_context,
            gl_context,
            dummy_surface,
            create_image,
            destroy_image,
            get_native_client_buffer,
            image_target_texture_2d,
        }
    }

    #[cfg(windows)]
    pub fn new_gl() -> Self {
        unimplemented!()
    }

    pub fn make_current(&self) {
        #[cfg(not(windows))]
        unsafe {
            self.adapter.as_hal::<api::Gles, _, _>(|raw_adapter| {
                let egl_instance = raw_adapter
                    .unwrap()
                    .adapter_context()
                    .egl_instance()
                    .unwrap();

                egl_instance
                    .make_current(
                        self.egl_display,
                        Some(self.dummy_surface),
                        Some(self.dummy_surface),
                        Some(self.egl_context),
                    )
                    .unwrap();
            })
        };
    }

    /// # Safety
    /// `buffer` must be a valid AHardwareBuffer.
    /// `texture` must be a valid GL texture.
    pub unsafe fn render_ahardwarebuffer_using_texture(
        &self,
        buffer: *const c_void,
        texture: gl::Texture,
        render_cb: impl FnOnce(),
    ) {
        const EGL_NATIVE_BUFFER_ANDROID: u32 = 0x3140;

        if !buffer.is_null() {
            let client_buffer = (self.get_native_client_buffer)(buffer);
            check_error(&self.gl_context, "get_native_client_buffer");

            let image = (self.create_image)(
                self.egl_display.as_ptr(),
                egl::NO_CONTEXT,
                EGL_NATIVE_BUFFER_ANDROID,
                client_buffer,
                ptr::null(),
            );
            check_error(&self.gl_context, "create_image");

            self.gl_context
                .bind_texture(GL_TEXTURE_EXTERNAL_OES, Some(texture));
            check_error(&self.gl_context, "bind texture OES");

            (self.image_target_texture_2d)(GL_TEXTURE_EXTERNAL_OES, image);
            check_error(&self.gl_context, "image_target_texture_2d");

            render_cb();

            (self.destroy_image)(self.egl_display.as_ptr(), image);
            check_error(&self.gl_context, "destroy_image");
        }
    }
}

#[cfg(not(windows))]
impl Default for GraphicsContext {
    fn default() -> Self {
        Self::new_gl()
    }
}
