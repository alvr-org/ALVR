#![allow(unused_variables)]

use alvr_common::{glam::UVec2, Fov, Pose};
use alvr_session::FoveatedEncodingConfig;
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};

#[cfg(target_os = "android")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

const HUD_TEXTURE_WIDTH: usize = 1280;
const HUD_TEXTURE_HEIGHT: usize = 720;
const FONT_SIZE: f32 = 50_f32;

pub struct RenderViewInput {
    pub pose: Pose,
    pub fov: Fov,
    pub swapchain_index: u32,
}

pub fn initialize() {
    #[cfg(target_os = "android")]
    unsafe {
        pub static LOBBY_ROOM_GLTF: &[u8] = include_bytes!("../resources/loading.gltf");
        pub static LOBBY_ROOM_BIN: &[u8] = include_bytes!("../resources/buffer.bin");

        LOBBY_ROOM_GLTF_PTR = LOBBY_ROOM_GLTF.as_ptr();
        LOBBY_ROOM_GLTF_LEN = LOBBY_ROOM_GLTF.len() as _;
        LOBBY_ROOM_BIN_PTR = LOBBY_ROOM_BIN.as_ptr();
        LOBBY_ROOM_BIN_LEN = LOBBY_ROOM_BIN.len() as _;

        initGraphicsNative();
    }
}

pub fn destroy() {
    #[cfg(target_os = "android")]
    unsafe {
        destroyGraphicsNative();
    }
}

pub fn resume(preferred_view_resolution: UVec2, swapchain_textures: [Vec<u32>; 2]) {
    #[cfg(target_os = "android")]
    unsafe {
        let swapchain_length = swapchain_textures[0].len();
        let mut swapchain_textures = [
            swapchain_textures[0].as_ptr(),
            swapchain_textures[1].as_ptr(),
        ];

        prepareLobbyRoom(
            preferred_view_resolution.x as _,
            preferred_view_resolution.y as _,
            swapchain_textures.as_mut_ptr(),
            swapchain_length as _,
        );
    }
}

pub fn pause() {
    #[cfg(target_os = "android")]
    unsafe {
        destroyRenderers();
    }
}

pub fn start_stream(
    view_resolution: UVec2,
    swapchain_textures: [Vec<u32>; 2],
    foveated_encoding: Option<FoveatedEncodingConfig>,
    enable_srgb_correction: bool,
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
        };

        streamStartNative(config);
    }
}

pub fn update_hud_message(message: &str) {
    let ubuntu_font =
        FontRef::try_from_slice(include_bytes!("../resources/Ubuntu-Medium.ttf")).unwrap();

    let section_glyphs = Layout::default()
        .h_align(HorizontalAlign::Center)
        .v_align(VerticalAlign::Center)
        .calculate_glyphs(
            &[&ubuntu_font],
            &SectionGeometry {
                screen_position: (
                    HUD_TEXTURE_WIDTH as f32 / 2_f32,
                    HUD_TEXTURE_HEIGHT as f32 / 2_f32,
                ),
                ..Default::default()
            },
            &[SectionText {
                text: message,
                scale: FONT_SIZE.into(),
                font_id: FontId(0),
            }],
        );

    let scaled_font = ubuntu_font.as_scaled(FONT_SIZE);

    let mut buffer = vec![0_u8; HUD_TEXTURE_WIDTH * HUD_TEXTURE_HEIGHT * 4];

    for section_glyph in section_glyphs {
        if let Some(outlined) = scaled_font.outline_glyph(section_glyph.glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, alpha| {
                let x = x as usize + bounds.min.x as usize;
                let y = y as usize + bounds.min.y as usize;
                buffer[(y * HUD_TEXTURE_WIDTH + x) * 4 + 3] = (alpha * 255.0) as u8;
            });
        }
    }

    #[cfg(target_os = "android")]
    unsafe {
        updateLobbyHudTexture(buffer.as_ptr());
    }
}

pub fn render_lobby(view_inputs: [RenderViewInput; 2]) {
    #[cfg(target_os = "android")]
    unsafe {
        let eye_inputs = [
            FfiViewInput {
                position: view_inputs[0].pose.position.to_array(),
                orientation: view_inputs[0].pose.orientation.to_array(),
                fovLeft: view_inputs[0].fov.left,
                fovRight: view_inputs[0].fov.right,
                fovUp: view_inputs[0].fov.up,
                fovDown: view_inputs[0].fov.down,
                swapchainIndex: view_inputs[0].swapchain_index as _,
            },
            FfiViewInput {
                position: view_inputs[1].pose.position.to_array(),
                orientation: view_inputs[1].pose.orientation.to_array(),
                fovLeft: view_inputs[1].fov.left,
                fovRight: view_inputs[1].fov.right,
                fovUp: view_inputs[1].fov.up,
                fovDown: view_inputs[1].fov.down,
                swapchainIndex: view_inputs[1].swapchain_index as _,
            },
        ];

        renderLobbyNative(eye_inputs.as_ptr());
    }
}

pub fn render_stream(hardware_buffer: *mut std::ffi::c_void, swapchain_indices: [u32; 2]) {
    #[cfg(target_os = "android")]
    unsafe {
        renderStreamNative(hardware_buffer, swapchain_indices.as_ptr());
    }
}
