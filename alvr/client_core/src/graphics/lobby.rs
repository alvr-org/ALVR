use super::{GraphicsContext, RenderViewInput};
use alvr_common::glam::UVec2;
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};
use std::rc::Rc;

const HUD_TEXTURE_WIDTH: usize = 1280;
const HUD_TEXTURE_HEIGHT: usize = 720;
const FONT_SIZE: f32 = 50_f32;

pub struct LobbyRenderer {
    _context: Rc<GraphicsContext>,
}

impl LobbyRenderer {
    #[allow(unused_variables)]
    pub fn new(
        context: Rc<GraphicsContext>,
        preferred_view_resolution: UVec2,
        swapchain_textures: [Vec<u32>; 2],
        enable_srgb_correction: bool,
        initial_hud_message: &str,
    ) -> Self {
        #[cfg(target_os = "android")]
        unsafe {
            let swapchain_length = swapchain_textures[0].len();
            let mut swapchain_textures = [
                swapchain_textures[0].as_ptr(),
                swapchain_textures[1].as_ptr(),
            ];

            super::opengl::prepareLobbyRoom(
                preferred_view_resolution.x as _,
                preferred_view_resolution.y as _,
                swapchain_textures.as_mut_ptr(),
                swapchain_length as _,
                enable_srgb_correction,
            );
        }

        let this = Self { _context: context };

        this.update_hud_message(initial_hud_message);

        this
    }

    pub fn update_hud_message(&self, message: &str) {
        let ubuntu_font =
            FontRef::try_from_slice(include_bytes!("../../resources/Ubuntu-Medium.ttf")).unwrap();

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
            super::opengl::updateLobbyHudTexture(buffer.as_ptr());
        }
    }

    #[allow(unused_variables)]
    pub fn render(&self, view_inputs: [RenderViewInput; 2]) {
        #[cfg(target_os = "android")]
        unsafe {
            let eye_inputs = [
                super::opengl::FfiViewInput {
                    position: view_inputs[0].pose.position.to_array(),
                    orientation: view_inputs[0].pose.orientation.to_array(),
                    fovLeft: view_inputs[0].fov.left,
                    fovRight: view_inputs[0].fov.right,
                    fovUp: view_inputs[0].fov.up,
                    fovDown: view_inputs[0].fov.down,
                    swapchainIndex: view_inputs[0].swapchain_index as _,
                },
                super::opengl::FfiViewInput {
                    position: view_inputs[1].pose.position.to_array(),
                    orientation: view_inputs[1].pose.orientation.to_array(),
                    fovLeft: view_inputs[1].fov.left,
                    fovRight: view_inputs[1].fov.right,
                    fovUp: view_inputs[1].fov.up,
                    fovDown: view_inputs[1].fov.down,
                    swapchainIndex: view_inputs[1].swapchain_index as _,
                },
            ];

            super::opengl::renderLobbyNative(eye_inputs.as_ptr());
        }
    }
}

impl Drop for LobbyRenderer {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        unsafe {
            super::opengl::destroyLobby();
        }
    }
}
