use super::{ck, GraphicsContext, RenderTarget, RenderViewInput};
use alvr_common::{
    glam::{IVec2, Mat4, UVec2, Vec3, Vec4},
    Fov, Pose,
};
use glow::{self as gl, HasContext, PixelUnpackData};
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};
use std::{f32::consts::FRAC_PI_2, num::NonZeroU32, rc::Rc};

const HUD_DIST: f32 = 5.0;
const HUD_SIDE: f32 = 3.5;
const HUD_TEXTURE_SIDE: usize = 1024;
const FONT_SIZE: f32 = 50.0;

const HAND_SKELETON_BONES: [(usize, usize); 19] = [
    // Thumb
    (2, 3),
    (3, 4),
    (4, 5),
    // Index
    (6, 7),
    (7, 8),
    (8, 9),
    (9, 10),
    // Middle
    (11, 12),
    (12, 13),
    (13, 14),
    (14, 15),
    // Ring
    (16, 17),
    (17, 18),
    (18, 19),
    (19, 20),
    // Pinky
    (21, 22),
    (22, 23),
    (23, 24),
    (24, 25),
];

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

    Mat4::from_cols(
        Vec4::new(a, 0.0, c, 0.0),
        Vec4::new(0.0, b, d, 0.0),
        Vec4::new(0.0, 0.0, -1.0, -2.0 * NEAR),
        Vec4::new(0.0, 0.0, -1.0, 0.0),
    )
    .transpose()
}

pub struct LobbyRenderer {
    context: Rc<GraphicsContext>,
    program: gl::Program,
    object_type_uloc: gl::UniformLocation,
    transform_uloc: gl::UniformLocation,
    hud_texture: gl::Texture,
    render_targets: [Vec<RenderTarget>; 2],
    viewport_size: IVec2,
}

impl LobbyRenderer {
    pub fn new(
        context: Rc<GraphicsContext>,
        view_resolution: UVec2,
        swapchain_textures: [Vec<u32>; 2],
        initial_hud_message: &str,
    ) -> Self {
        let gl = &context.gl_context;

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

        let hud_texture = super::create_texture(
            gl,
            UVec2::new(HUD_TEXTURE_SIDE as u32, HUD_TEXTURE_SIDE as u32),
            gl::RGBA8,
        );

        let program = super::create_program(
            gl,
            include_str!("../../resources/lobby_vertex.glsl"),
            include_str!("../../resources/lobby_fragment.glsl"),
        );

        let this = unsafe {
            let object_type_uloc = ck!(gl.get_uniform_location(program, "object_type").unwrap());
            let transform_uloc = ck!(gl.get_uniform_location(program, "transform").unwrap());

            Self {
                context,
                program,
                object_type_uloc,
                transform_uloc,
                hud_texture,
                render_targets,
                viewport_size: view_resolution.as_ivec2(),
            }
        };

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
                        HUD_TEXTURE_SIDE as f32 / 2_f32,
                        HUD_TEXTURE_SIDE as f32 / 2_f32,
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

        let mut buffer = vec![0; HUD_TEXTURE_SIDE * HUD_TEXTURE_SIDE * 4];

        for section_glyph in section_glyphs {
            if let Some(outlined) = scaled_font.outline_glyph(section_glyph.glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|x, y, alpha| {
                    let x = x as usize + bounds.min.x as usize;
                    let y = y as usize + bounds.min.y as usize;
                    if x < HUD_TEXTURE_SIDE && y < HUD_TEXTURE_SIDE {
                        buffer[(y * HUD_TEXTURE_SIDE + x) * 4 + 3] = (alpha * 255.0) as u8;
                    }
                });
            }
        }

        let gl = &self.context.gl_context;
        unsafe {
            ck!(gl.bind_texture(gl::TEXTURE_2D, Some(self.hud_texture)));
            ck!(gl.tex_sub_image_2d(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                HUD_TEXTURE_SIDE as i32,
                HUD_TEXTURE_SIDE as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                PixelUnpackData::Slice(&buffer),
            ));
        }
    }

    pub fn render(
        &self,
        view_inputs: [RenderViewInput; 2],
        hand_poses: [(Option<Pose>, Option<[Pose; 26]>); 2],
    ) {
        let gl = &self.context.gl_context;

        unsafe {
            ck!(gl.use_program(Some(self.program)));

            ck!(gl.disable(gl::SCISSOR_TEST));
            ck!(gl.disable(gl::DEPTH_TEST));
            ck!(gl.disable(gl::CULL_FACE));
            ck!(gl.enable(gl::BLEND));
            ck!(gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));

            ck!(gl.viewport(0, 0, self.viewport_size.x, self.viewport_size.y));

            for (view_idx, view_input) in view_inputs.iter().enumerate() {
                self.render_targets[view_idx][view_input.swapchain_index as usize].bind();

                let view = Mat4::from_rotation_translation(
                    view_input.pose.orientation,
                    view_input.pose.position,
                )
                .inverse();
                let view_proj = projection_from_fov(view_input.fov) * view;

                ck!(gl.clear(gl::COLOR_BUFFER_BIT));
                ck!(gl.clear_color(0.0, 0.0, 0.02, 1.0));

                // Draw the following geometry in the correct order (depth buffer is disabled)

                // Render ground
                ck!(gl.uniform_1_i32(Some(&self.object_type_uloc), 0));
                ck!(gl.uniform_matrix_4_f32_slice(
                    Some(&self.transform_uloc),
                    false,
                    &view_proj.to_cols_array(),
                ));
                ck!(gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4));

                // Render HUD
                // todo: draw only one HUD panel and implement lazy follow
                ck!(gl.uniform_1_i32(Some(&self.object_type_uloc), 1));
                ck!(gl.active_texture(gl::TEXTURE0));
                ck!(gl.bind_texture(gl::TEXTURE_2D, Some(self.hud_texture)));
                for i in 0..4 {
                    let panel_transform = Mat4::from_rotation_y(FRAC_PI_2 * i as f32)
                        * Mat4::from_translation(Vec3::new(0.0, HUD_SIDE / 2.0, -HUD_DIST))
                        * Mat4::from_scale(Vec3::ONE * HUD_SIDE);
                    ck!(gl.uniform_matrix_4_f32_slice(
                        Some(&self.transform_uloc),
                        false,
                        &(view_proj * panel_transform).to_cols_array(),
                    ));
                    ck!(gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4));
                }

                // Render hands
                gl.uniform_1_i32(Some(&self.object_type_uloc), 2);
                for (maybe_pose, maybe_skeleton) in &hand_poses {
                    if let Some(skeleton) = maybe_skeleton {
                        for (joint1_idx, joint2_idx) in HAND_SKELETON_BONES {
                            let j1_pose = skeleton[joint1_idx];
                            let j2_pose = skeleton[joint2_idx];

                            let bone_transform = Mat4::from_scale_rotation_translation(
                                Vec3::ONE * Vec3::distance(j1_pose.position, j2_pose.position),
                                j1_pose.orientation,
                                j1_pose.position,
                            );
                            ck!(gl.uniform_matrix_4_f32_slice(
                                Some(&self.transform_uloc),
                                false,
                                &(view_proj * bone_transform).to_cols_array(),
                            ));
                            ck!(gl.draw_arrays(gl::LINES, 0, 2));
                        }
                    } else if let Some(pose) = maybe_pose {
                        let hand_transform = Mat4::from_scale_rotation_translation(
                            Vec3::ONE * 0.2,
                            pose.orientation,
                            pose.position,
                        );

                        let segment_rotations = [
                            Mat4::IDENTITY,
                            Mat4::from_rotation_y(FRAC_PI_2),
                            Mat4::from_rotation_x(FRAC_PI_2),
                        ];
                        for rot in &segment_rotations {
                            let segment_transform = hand_transform
                                * *rot
                                * Mat4::from_scale(Vec3::ONE * 0.5)
                                * Mat4::from_translation(Vec3::Z * 0.5);

                            ck!(gl.uniform_matrix_4_f32_slice(
                                Some(&self.transform_uloc),
                                false,
                                &(view_proj * segment_transform).to_cols_array(),
                            ));
                            ck!(gl.draw_arrays(gl::LINES, 0, 2));
                        }
                    }
                }
            }
        }
    }
}

impl Drop for LobbyRenderer {
    fn drop(&mut self) {
        let gl = &self.context.gl_context;

        unsafe {
            ck!(gl.delete_texture(self.hud_texture));
            ck!(gl.delete_program(self.program));
        }
    }
}
