use super::{GraphicsContext, SDR_FORMAT};
use alvr_common::{
    glam::{Mat4, Quat, UVec2, Vec3, Vec4},
    Fov, Pose,
};
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};
use std::{f32::consts::FRAC_PI_2, rc::Rc};
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendFactor, BlendOperation, BlendState, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, Device, Extent3d, FilterMode, FragmentState, ImageCopyTexture,
    ImageDataLayout, LoadOp, Operations, Origin3d, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, PushConstantRange, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderStages, StoreOp, Texture, TextureAspect,
    TextureSampleType, TextureView, TextureViewDimension, VertexState,
};

const FLOOR_SIDE: f32 = 300.0;
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
            entry_point: "vertex_main",
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
            entry_point: "fragment_main",
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
    })
}

pub struct RenderViewInput {
    pub pose: Pose,
    pub fov: Fov,
    pub swapchain_index: u32,
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
            72,
            include_wgsl!("../../resources/lobby_quad.wgsl"),
            PrimitiveTopology::TriangleStrip,
        );

        let line_pipeline = create_pipeline(
            device,
            "lobby_line",
            &[],
            64,
            include_wgsl!("../../resources/lobby_line.wgsl"),
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

        self.context.queue.write_texture(
            ImageCopyTexture {
                texture: &self.hud_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &buffer,
            ImageDataLayout {
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

    pub fn render(
        &self,
        view_inputs: [RenderViewInput; 2],
        hand_data: [(Option<Pose>, Option<[Pose; 26]>); 2],
        body_skeleton_fb: Option<Vec<Option<Pose>>>,
    ) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("lobby_command_encoder"),
            });

        for (view_idx, view_input) in view_inputs.iter().enumerate() {
            let view = Mat4::from_rotation_translation(
                view_input.pose.orientation,
                view_input.pose.position,
            )
            .inverse();
            let view_proj = projection_from_fov(view_input.fov) * view;

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some(&format!("lobby_view_{}", view_idx)),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.render_targets[view_idx][view_input.swapchain_index as usize],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.02,
                            a: 1.0,
                        }),
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
                pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, &data);
                pass.draw(0..vertices_count, 0..1);
            }

            // Draw the following geometry in the correct order (depth buffer is disabled)

            // Bind quad pipeline
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);

            // Render ground
            pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 64, &0_u32.to_le_bytes());
            pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 68, &FLOOR_SIDE.to_le_bytes());
            let transform = view_proj
                * Mat4::from_rotation_x(-FRAC_PI_2)
                * Mat4::from_scale(Vec3::ONE * FLOOR_SIDE);
            transform_draw(&mut pass, transform, 4);

            // Render HUD
            pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 64, &1_u32.to_le_bytes());
            for i in 0..4 {
                let transform = Mat4::from_rotation_y(FRAC_PI_2 * i as f32)
                    * Mat4::from_translation(Vec3::new(0.0, HUD_SIDE / 2.0, -HUD_DIST))
                    * Mat4::from_scale(Vec3::ONE * HUD_SIDE);
                transform_draw(&mut pass, view_proj * transform, 4);
            }

            // Bind line pipeline and render hands
            pass.set_pipeline(&self.line_pipeline);
            for (maybe_pose, maybe_skeleton) in &hand_data {
                if let Some(skeleton) = maybe_skeleton {
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

                if let Some(pose) = maybe_pose {
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
                        let transform = hand_transform
                            * *rot
                            * Mat4::from_scale(Vec3::ONE * 0.5)
                            * Mat4::from_translation(Vec3::Z * 0.5);
                        transform_draw(&mut pass, view_proj * transform, 2);
                    }
                }
            }
            if let Some(skeleton) = &body_skeleton_fb {
                for (joint1_idx, joint2_idx) in BODY_SKELETON_BONES_FB {
                    if let (Some(Some(j1_pose)), Some(Some(j2_pose))) =
                        (skeleton.get(joint1_idx), skeleton.get(joint2_idx))
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
