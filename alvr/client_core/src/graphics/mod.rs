mod lobby;
mod opengl;
mod stream;

pub use lobby::*;
pub use opengl::choose_swapchain_format;
pub use stream::*;

use alvr_common::{Fov, Pose};
use khronos_egl::{self as egl, EGL1_4};

pub struct RenderViewInput {
    pub pose: Pose,
    pub fov: Fov,
    pub swapchain_index: u32,
}

pub struct GraphicsContext {
    _instance: egl::DynamicInstance<EGL1_4>,
    pub display: egl::Display,
    pub config: egl::Config,
    pub context: egl::Context,
    _dummy_surface: egl::Surface,
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

        #[cfg(target_os = "android")]
        unsafe {
            pub static LOBBY_ROOM_GLTF: &[u8] = include_bytes!("../../resources/loading.gltf");
            pub static LOBBY_ROOM_BIN: &[u8] = include_bytes!("../../resources/buffer.bin");

            opengl::LOBBY_ROOM_GLTF_PTR = LOBBY_ROOM_GLTF.as_ptr();
            opengl::LOBBY_ROOM_GLTF_LEN = LOBBY_ROOM_GLTF.len() as _;
            opengl::LOBBY_ROOM_BIN_PTR = LOBBY_ROOM_BIN.as_ptr();
            opengl::LOBBY_ROOM_BIN_LEN = LOBBY_ROOM_BIN.len() as _;

            opengl::initGraphicsNative();
        }

        Self {
            _instance: instance,
            display,
            config,
            context,
            _dummy_surface: dummy_surface,
        }
    }
}

impl Default for GraphicsContext {
    fn default() -> Self {
        Self::new()
    }
}
