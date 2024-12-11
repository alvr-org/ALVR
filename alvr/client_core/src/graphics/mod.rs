mod lobby;
mod staging;
mod stream;

pub use lobby::*;
pub use stream::*;

use alvr_common::{
    glam::{Mat4, UVec2, Vec4},
    Fov,
};
use glow::{self as gl, HasContext};
use khronos_egl as egl;
use std::{ffi::c_void, num::NonZeroU32, ptr};
use wgpu::{
    hal::{self, api, MemoryFlags, TextureUses},
    Adapter, Device, Extent3d, Instance, Queue, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView,
};

pub const SDR_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
pub const SDR_FORMAT_GL: u32 = gl::RGBA8;
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

fn projection_from_fov(fov: Fov) -> Mat4 {
    const NEAR: f32 = 0.1;

    let tanl = f32::tan(fov.left);
    let tanr = f32::tan(fov.right);
    let tanu = f32::tan(fov.up);
    let tand = f32::tan(fov.down);
    let a = 2.0 / (tanr - tanl);
    let b = 2.0 / (tanu - tand);
    let c = (tanr + tanl) / (tanr - tanl);
    let d = (tanu + tand) / (tanu - tand);

    // note: for wgpu compatibility, the b and d components should be flipped. Maybe a bug in the
    // viewport handling in wgpu?
    Mat4::from_cols(
        Vec4::new(a, 0.0, c, 0.0),
        Vec4::new(0.0, -b, -d, 0.0),
        Vec4::new(0.0, 0.0, -1.0, -NEAR),
        Vec4::new(0.0, 0.0, -1.0, 0.0),
    )
    .transpose()
}

pub fn choose_swapchain_format(supported_formats: &[u32], enable_hdr: bool) -> u32 {
    // Priority-sorted list of swapchain formats we'll accept--
    let mut app_supported_swapchain_formats = vec![gl::SRGB8_ALPHA8, gl::RGBA8];

    // float16 is required for HDR output. However, float16 swapchains
    // have a high perf cost, so only use these if HDR is enabled.
    if enable_hdr {
        app_supported_swapchain_formats.insert(0, gl::RGBA16F);
    }

    for format in app_supported_swapchain_formats {
        if supported_formats.contains(&format) {
            return format;
        }
    }

    // If we can't enumerate, default to a required format
    gl::RGBA8
}

pub fn gl_format_to_wgpu(format: u32) -> TextureFormat {
    match format {
        gl::SRGB8_ALPHA8 => TextureFormat::Rgba8UnormSrgb,
        gl::RGBA8 => TextureFormat::Rgba8Unorm,
        gl::RGBA16F => TextureFormat::Rgba16Float,
        _ => panic!("Unsupported GL format: {}", format),
    }
}

pub fn create_texture(device: &Device, resolution: UVec2, format: TextureFormat) -> Texture {
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
        format,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn create_texture_from_gles(
    device: &Device,
    texture: u32,
    resolution: UVec2,
    format: TextureFormat,
) -> Texture {
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
                        format,
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
                format,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        )
    }
}

// This is used to convert OpenXR swapchains to wgpu
pub fn create_gl_swapchain(
    device: &Device,
    gl_textures: &[u32],
    resolution: UVec2,
    format: TextureFormat,
) -> Vec<TextureView> {
    gl_textures
        .iter()
        .map(|gl_tex| {
            create_texture_from_gles(device, *gl_tex, resolution, format)
                .create_view(&Default::default())
        })
        .collect()
}

pub struct GraphicsContext {
    _instance: Instance,

    #[cfg_attr(windows, expect(dead_code))]
    adapter: Adapter,

    device: Device,
    queue: Queue,
    pub egl_display: egl::Display,
    pub egl_config: egl::Config,
    pub egl_context: egl::Context,
    pub gl_context: gl::Context,

    #[cfg_attr(windows, expect(dead_code))]
    dummy_surface: egl::Surface,

    create_image: CreateImageFn,
    destroy_image: DestroyImageFn,
    get_native_client_buffer: GetNativeClientBufferFn,
    image_target_texture_2d: ImageTargetTexture2DFn,
}

impl GraphicsContext {
    #[cfg(not(windows))]
    pub fn new_gl() -> Self {
        use std::mem;
        use wgpu::{
            Backends, DeviceDescriptor, Features, InstanceDescriptor, InstanceFlags, Limits,
        };

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
                    ..adapter.limits()
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
