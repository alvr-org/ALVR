use super::{GraphicsContext, MAX_PUSH_CONSTANTS_SIZE, SDR_FORMAT};
use alvr_common::{
    DeviceMotion, Pose, ViewParams,
    glam::{IVec2, Mat4, Quat, UVec2, Vec3},
};
use glyph_brush_layout::{
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
    ab_glyph::{Font, FontRef, ScaleFont},
};
use std::{f32::consts::FRAC_PI_2, mem, rc::Rc};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
    BlendOperation, BlendState, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    Device, Extent3d, FilterMode, FragmentState, LoadOp, Operations, Origin3d,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, PushConstantRange, RenderPass,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor, ShaderStages, StoreOp,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureSampleType,
    TextureView, TextureViewDimension, VertexState, include_wgsl,
};

const TRANSFORM_CONST_SIZE: u32 = mem::size_of::<Mat4>() as u32;
const OBJECT_TYPE_CONST_SIZE: u32 = mem::size_of::<u32>() as u32;
const FLOOR_SIDE_CONST_SIZE: u32 = mem::size_of::<f32>() as u32;
const COLOR_CONST_SIZE: u32 = mem::size_of::<u32>() as u32;

const QUAD_PUSH_CONTANTS_SIZE: u32 =
    TRANSFORM_CONST_SIZE + OBJECT_TYPE_CONST_SIZE + FLOOR_SIDE_CONST_SIZE;
const LINE_PUSH_CONTANTS_SIZE: u32 = TRANSFORM_CONST_SIZE + COLOR_CONST_SIZE;
const _: () = assert!(
    QUAD_PUSH_CONTANTS_SIZE <= MAX_PUSH_CONSTANTS_SIZE
        && LINE_PUSH_CONTANTS_SIZE <= MAX_PUSH_CONSTANTS_SIZE,
    "Push constants size exceeds the maximum size"
);

const TRANSFORM_CONST_OFFSET: u32 = 0;
const OBJECT_TYPE_CONST_OFFSET: u32 = TRANSFORM_CONST_SIZE;
const FLOOR_SIDE_CONST_OFFSET: u32 = OBJECT_TYPE_CONST_OFFSET + OBJECT_TYPE_CONST_SIZE;
const COLOR_CONST_OFFSET: u32 = TRANSFORM_CONST_SIZE;

const FLOOR_SIDE: f32 = 300.0;
const HUD_DIST: f32 = 5.0;
const HUD_SIDE: f32 = 3.5;
const HUD_TEXTURE_SIDE: usize = 1024;
const FONT_SIZE: f32 = 50.0;

const FAST_BORDER_OFFSETS: [IVec2; 8] = [
    IVec2::new(0, -3),
    IVec2::new(2, -2),
    IVec2::new(3, 0),
    IVec2::new(2, 2),
    IVec2::new(0, 3),
    IVec2::new(-2, 2),
    IVec2::new(-3, 0),
    IVec2::new(-2, -2),
];
const MAX_BORDER_OFFSET: i32 = 3;

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

const BODY_SKELETON_BONES_FB: [(usize, usize); 30] = [
    // Spine
    (1, 2),
    (2, 3),
    (3, 4),
    (4, 5),
    (5, 6),
    (6, 7),
    // Left arm
    (5, 8),
    (8, 9),
    (9, 10),
    (10, 11),
    (11, 12),
    // Right arm
    (5, 13),
    (13, 14),
    (14, 15),
    (15, 16),
    (16, 17),
    // Left leg
    (1, 70),
    (70, 71),
    (71, 72),
    (72, 73),
    (73, 74),
    (74, 75),
    (75, 76),
    // Right leg
    (1, 77),
    (77, 78),
    (78, 79),
    (79, 80),
    (80, 81),
    (81, 82),
    (82, 83),
];

const BODY_SKELETON_BONES_BD: [(usize, usize); 23] = [
    // Left leg
    (0, 1),
    (1, 4),
    (4, 7),
    (7, 10),
    // Right leg
    (0, 2),
    (2, 5),
    (5, 8),
    (8, 11),
    // Spine
    (0, 3),
    (3, 6),
    (6, 9),
    (9, 12),
    (12, 15),
    // Left arm
    (9, 13),
    (13, 16),
    (16, 18),
    (18, 20),
    (20, 22),
    // Right arm
    (9, 14),
    (14, 17),
    (17, 19),
    (19, 21),
    (21, 23),
];

pub enum BodyTrackingType {
    Meta,
    Pico,
}

fn create_pipeline(
    device: &Device,
    label: &str,
    bind_group_layouts: &[&BindGroupLayout],
    push_constants_len: u32,
    shader: ShaderModuleDescriptor,
    topology: PrimitiveTopology,
) -> RenderPipeline {
    let shader_module = device.create_shader_module(shader);
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(label),
        // Note: Layout cannot be inferred because of a bug with push constants
        layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts,
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..push_constants_len,
            }],
        })),
        vertex: VertexState {
            module: &shader_module,
            entry_point: None,
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: PrimitiveState {
            topology,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(FragmentState {
            module: &shader_module,
            entry_point: None,
            compilation_options: Default::default(),
            targets: &[Some(ColorTargetState {
                format: SDR_FORMAT,
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                }),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

pub struct LobbyViewParams {
    pub swapchain_index: u32,
    pub view_params: ViewParams,
}

pub struct LobbyRenderer {
    context: Rc<GraphicsContext>,
    quad_pipeline: RenderPipeline,
    line_pipeline: RenderPipeline,
    hud_texture: Texture,
    bind_group: BindGroup,
    render_targets: [Vec<TextureView>; 2],
}

impl LobbyRenderer {
    pub fn new(
        context: Rc<GraphicsContext>,
        view_resolution: UVec2,
        swapchain_textures: [Vec<u32>; 2],
        initial_hud_message: &str,
    ) -> Self {
        let device = &context.device;

        let hud_texture =
            super::create_texture(device, UVec2::ONE * HUD_TEXTURE_SIDE as u32, SDR_FORMAT);

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let quad_pipeline = create_pipeline(
            device,
            "lobby_quad",
            &[&bind_group_layout],
            QUAD_PUSH_CONTANTS_SIZE,
            include_wgsl!("../resources/lobby_quad.wgsl"),
            PrimitiveTopology::TriangleStrip,
        );

        let line_pipeline = create_pipeline(
            device,
            "lobby_line",
            &[],
            LINE_PUSH_CONTANTS_SIZE,
            include_wgsl!("../resources/lobby_line.wgsl"),
            PrimitiveTopology::LineList,
        );

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &hud_texture.create_view(&Default::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&device.create_sampler(
                        &SamplerDescriptor {
                            mag_filter: FilterMode::Linear,
                            min_filter: FilterMode::Linear,
                            ..Default::default()
                        },
                    )),
                },
            ],
        });

        let render_targets = [
            super::create_gl_swapchain(device, &swapchain_textures[0], view_resolution, SDR_FORMAT),
            super::create_gl_swapchain(device, &swapchain_textures[1], view_resolution, SDR_FORMAT),
        ];

        let this = Self {
            context,
            quad_pipeline,
            line_pipeline,
            hud_texture,
            bind_group,
            render_targets,
        };

        this.update_hud_message(initial_hud_message);

        this
    }

    pub fn update_hud_message(&self, message: &str) {
        let ubuntu_font =
            FontRef::try_from_slice(include_bytes!("../resources/Ubuntu-Medium.ttf")).unwrap();

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
                    let x = x as i32 + bounds.min.x as i32;
                    let y = y as i32 + bounds.min.y as i32;

                    if x >= MAX_BORDER_OFFSET
                        && y >= MAX_BORDER_OFFSET
                        && x < HUD_TEXTURE_SIDE as i32 - MAX_BORDER_OFFSET
                        && y < HUD_TEXTURE_SIDE as i32 - MAX_BORDER_OFFSET
                    {
                        let coord = (y as usize * HUD_TEXTURE_SIDE + x as usize) * 4;
                        let value = (alpha * 255.0) as u8;

                        buffer[coord] = value;
                        buffer[coord + 1] = value;
                        buffer[coord + 2] = value;

                        // Render opacity with border
                        for offset in &FAST_BORDER_OFFSETS {
                            let coord = ((y + offset.y) as usize * HUD_TEXTURE_SIDE
                                + (x + offset.x) as usize)
                                * 4;
                            buffer[coord + 3] = u8::max(buffer[coord + 3], value);
                        }
                    }
                });
            }
        }

        self.context.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.hud_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &buffer,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(HUD_TEXTURE_SIDE as u32 * 4),
                rows_per_image: Some(HUD_TEXTURE_SIDE as u32),
            },
            Extent3d {
                width: HUD_TEXTURE_SIDE as u32,
                height: HUD_TEXTURE_SIDE as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    #[expect(clippy::too_many_arguments)]
    pub fn render(
        &self,
        view_params: [LobbyViewParams; 2],
        hand_data: [(Option<DeviceMotion>, Option<[Pose; 26]>); 2],
        additional_motions: Option<Vec<DeviceMotion>>,
        body_skeleton: Option<Vec<Option<Pose>>>,
        body_tracking_type: Option<BodyTrackingType>,
        render_background: bool,
        show_velocities: bool,
    ) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("lobby_command_encoder"),
            });

        for (view_idx, view_input) in view_params.iter().enumerate() {
            let view = Mat4::from_rotation_translation(
                view_input.view_params.pose.orientation,
                view_input.view_params.pose.position,
            )
            .inverse();
            let view_proj = super::projection_from_fov(view_input.view_params.fov) * view;

            let clear_color = if render_background {
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.02,
                    a: 1.0,
                }
            } else {
                Color::TRANSPARENT
            };

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some(&format!("lobby_view_{view_idx}")),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.render_targets[view_idx][view_input.swapchain_index as usize],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            fn transform_draw(pass: &mut RenderPass, transform: Mat4, vertices_count: u32) {
                let data = transform
                    .to_cols_array()
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<u8>>();
                pass.set_push_constants(
                    ShaderStages::VERTEX_FRAGMENT,
                    TRANSFORM_CONST_OFFSET,
                    &data,
                );
                pass.draw(0..vertices_count, 0..1);
            }

            // Draw the following geometry in the correct order (depth buffer is disabled)

            // Bind quad pipeline
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);

            if render_background {
                // Render ground
                pass.set_push_constants(
                    ShaderStages::VERTEX_FRAGMENT,
                    OBJECT_TYPE_CONST_OFFSET,
                    &0_u32.to_le_bytes(),
                );
                pass.set_push_constants(
                    ShaderStages::VERTEX_FRAGMENT,
                    FLOOR_SIDE_CONST_OFFSET,
                    &FLOOR_SIDE.to_le_bytes(),
                );
                let transform = view_proj
                    * Mat4::from_rotation_x(-FRAC_PI_2)
                    * Mat4::from_scale(Vec3::ONE * FLOOR_SIDE);
                transform_draw(&mut pass, transform, 4);
            }

            // Render HUD
            pass.set_push_constants(
                ShaderStages::VERTEX_FRAGMENT,
                OBJECT_TYPE_CONST_OFFSET,
                &1_u32.to_le_bytes(),
            );
            for i in 0..4 {
                let transform = Mat4::from_rotation_y(FRAC_PI_2 * i as f32)
                    * Mat4::from_translation(Vec3::new(0.0, HUD_SIDE / 2.0, -HUD_DIST))
                    * Mat4::from_scale(Vec3::ONE * HUD_SIDE);
                transform_draw(&mut pass, view_proj * transform, 4);
            }

            fn draw_crosshair(
                pass: &mut RenderPass,
                motion: &DeviceMotion,
                view_proj: Mat4,
                show_velocities: bool,
            ) {
                let hand_transform = Mat4::from_scale_rotation_translation(
                    Vec3::ONE * 0.2,
                    motion.pose.orientation,
                    motion.pose.position,
                );

                // Draw crosshair
                let segment_rotations = [
                    Mat4::IDENTITY,
                    Mat4::from_rotation_y(FRAC_PI_2),
                    Mat4::from_rotation_x(FRAC_PI_2),
                ];
                pass.set_push_constants(
                    ShaderStages::VERTEX_FRAGMENT,
                    COLOR_CONST_OFFSET,
                    &[255, 255, 255, 255],
                );
                for rot in &segment_rotations {
                    let transform = hand_transform
                        * *rot
                        * Mat4::from_scale(Vec3::ONE * 0.5)
                        * Mat4::from_translation(Vec3::Z * 0.5);
                    transform_draw(pass, view_proj * transform, 2);
                }

                if show_velocities {
                    // Draw linear velocity
                    let transform = Mat4::from_scale_rotation_translation(
                        Vec3::ONE * motion.linear_velocity.length() * 0.2,
                        Quat::from_rotation_arc(-Vec3::Z, motion.linear_velocity.normalize()),
                        motion.pose.position,
                    );
                    pass.set_push_constants(
                        ShaderStages::VERTEX_FRAGMENT,
                        COLOR_CONST_OFFSET,
                        &[255, 0, 0, 255],
                    );
                    transform_draw(pass, view_proj * transform, 2);

                    // Draw angular velocity
                    let transform = Mat4::from_scale_rotation_translation(
                        Vec3::ONE * motion.angular_velocity.length() * 0.01,
                        Quat::from_rotation_arc(-Vec3::Z, motion.angular_velocity.normalize()),
                        motion.pose.position,
                    );
                    pass.set_push_constants(
                        ShaderStages::VERTEX_FRAGMENT,
                        COLOR_CONST_OFFSET,
                        &[0, 255, 0, 255],
                    );
                    transform_draw(pass, view_proj * transform, 2);
                }
            }

            // Render hands and body skeleton
            pass.set_pipeline(&self.line_pipeline);
            for (maybe_motion, maybe_skeleton) in &hand_data {
                if let Some(skeleton) = maybe_skeleton {
                    pass.set_push_constants(
                        ShaderStages::VERTEX_FRAGMENT,
                        COLOR_CONST_OFFSET,
                        &[255, 255, 255, 255],
                    );

                    for (joint1_idx, joint2_idx) in HAND_SKELETON_BONES {
                        let j1_pose = skeleton[joint1_idx];
                        let j2_pose = skeleton[joint2_idx];

                        let transform = Mat4::from_scale_rotation_translation(
                            Vec3::ONE * Vec3::distance(j1_pose.position, j2_pose.position),
                            j1_pose.orientation,
                            j1_pose.position,
                        );
                        transform_draw(&mut pass, view_proj * transform, 2);
                    }
                }

                if let Some(motion) = maybe_motion {
                    draw_crosshair(&mut pass, motion, view_proj, show_velocities);
                }
            }

            if let Some(motions) = &additional_motions {
                for motion in motions {
                    draw_crosshair(&mut pass, motion, view_proj, show_velocities);
                }
            }

            let body_skeleton_bones = match body_tracking_type {
                Some(BodyTrackingType::Meta) => Some(BODY_SKELETON_BONES_FB.as_slice()),
                Some(BodyTrackingType::Pico) => Some(BODY_SKELETON_BONES_BD.as_slice()),
                _ => None,
            };

            if let (Some(skeleton), Some(skeleton_bones)) = (&body_skeleton, body_skeleton_bones) {
                for (joint1_idx, joint2_idx) in skeleton_bones {
                    if let (Some(Some(j1_pose)), Some(Some(j2_pose))) =
                        (skeleton.get(*joint1_idx), skeleton.get(*joint2_idx))
                    {
                        let transform = Mat4::from_scale_rotation_translation(
                            Vec3::ONE * Vec3::distance(j1_pose.position, j2_pose.position),
                            Quat::from_rotation_arc(
                                -Vec3::Z,
                                (j2_pose.position - j1_pose.position).normalize(),
                            ),
                            j1_pose.position,
                        );
                        transform_draw(&mut pass, view_proj * transform, 2);
                    }
                }
            }
        }

        self.context.queue.submit(Some(encoder.finish()));
    }
}
