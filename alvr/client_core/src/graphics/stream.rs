use super::{staging::StagingRenderer, GraphicsContext};
use alvr_common::{
    glam::{self, Mat4, Quat, UVec2, Vec3},
    Fov,
};
use alvr_session::{FoveatedEncodingConfig, PassthroughMode};
use std::{collections::HashMap, ffi::c_void, iter, rc::Rc};
use wgpu::{
    hal::{api, gles},
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, ColorTargetState, ColorWrites,
    FragmentState, LoadOp, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, PushConstantRange, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StoreOp, TextureSampleType, TextureView, TextureViewDescriptor, TextureViewDimension,
    VertexState,
};

pub struct StreamViewParams {
    pub swapchain_index: u32,
    pub reprojection_rotation: Quat,
    pub fov: Fov,
}

#[derive(Debug)]
struct ViewObjects {
    bind_group: BindGroup,
    render_target: Vec<TextureView>,
}

pub struct StreamRenderer {
    context: Rc<GraphicsContext>,
    staging_renderer: StagingRenderer,
    pipeline: RenderPipeline,
    views_objects: [ViewObjects; 2],
}

impl StreamRenderer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: Rc<GraphicsContext>,
        view_resolution: UVec2,
        swapchain_textures: [Vec<u32>; 2],
        target_format: u32,
        foveated_encoding: Option<FoveatedEncodingConfig>,
        enable_srgb_correction: bool,
        fix_limited_range: bool,
        encoding_gamma: f32,
        passthrough: Option<PassthroughMode>,
    ) -> Self {
        let device = &context.device;

        let target_format = super::gl_format_to_wgpu(target_format);

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

        let shader_module =
            device.create_shader_module(include_wgsl!("../../resources/stream.wgsl"));

        let mut constants = HashMap::new();

        constants.extend([
            (
                "ENABLE_SRGB_CORRECTION".into(),
                enable_srgb_correction.into(),
            ),
            ("FIX_LIMITED_RANGE".into(), fix_limited_range.into()),
            ("ENCODING_GAMMA".into(), encoding_gamma.into()),
        ]);

        if let Some(mode) = passthrough {
            let ps_alpha = match mode {
                PassthroughMode::AugmentedReality { brightness } => brightness,
                PassthroughMode::Blend { opacity } => opacity,
            };
            constants.extend([("COLOR_ALPHA".into(), (1. - ps_alpha).into())]);
        }

        let staging_resolution = if let Some(foveated_encoding) = foveated_encoding {
            let (staging_resolution, ffe_constants) =
                foveated_encoding_shader_constants(view_resolution, foveated_encoding);
            constants.extend(ffe_constants);

            staging_resolution
        } else {
            view_resolution
        };

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            // Note: Layout cannot be inferred because of a bug with push constants
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX_FRAGMENT,
                    range: 0..68,
                }],
            })),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vertex_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    zero_initialize_workgroup_memory: false,
                },
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    zero_initialize_workgroup_memory: false,
                },
                targets: &[Some(ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let mut view_objects = vec![];
        let mut staging_textures_gl = vec![];
        for target_swapchain in &swapchain_textures {
            let staging_texture = super::create_texture(device, staging_resolution, target_format);

            let staging_texture_gl = unsafe {
                staging_texture.as_hal::<api::Gles, _, _>(|tex| {
                    let gles::TextureInner::Texture { raw, .. } = tex.unwrap().inner else {
                        panic!("invalid texture type");
                    };
                    raw
                })
            };

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &staging_texture.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&sampler),
                    },
                ],
            });

            let render_target = super::create_gl_swapchain(
                device,
                target_swapchain,
                view_resolution,
                target_format,
            );

            view_objects.push(ViewObjects {
                bind_group,
                render_target,
            });
            staging_textures_gl.push(staging_texture_gl);
        }

        let staging_renderer = StagingRenderer::new(
            Rc::clone(&context),
            staging_textures_gl.try_into().unwrap(),
            staging_resolution,
        );

        Self {
            context,
            staging_renderer,
            pipeline,
            views_objects: view_objects.try_into().unwrap(),
        }
    }

    pub unsafe fn render(&self, hardware_buffer: *mut c_void, view_params: [StreamViewParams; 2]) {
        // if hardware_buffer is available copy stream to staging texture
        if !hardware_buffer.is_null() {
            self.staging_renderer.render(hardware_buffer);
        }

        let mut encoder = self
            .context
            .device
            .create_command_encoder(&Default::default());

        for (view_idx, view_params) in view_params.iter().enumerate() {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.views_objects[view_idx].render_target
                        [view_params.swapchain_index as usize],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            let fov = view_params.fov;

            let tanl = f32::tan(fov.left);
            let tanr = f32::tan(fov.right);
            let tanu = f32::tan(fov.up);
            let tand = f32::tan(fov.down);

            let width = tanr - tanl;
            let height = tanu - tand;

            // The image is at z = -1.0, so we use tangents for the size
            let model_mat =
                Mat4::from_translation(Vec3::new(width / 2.0 + tanl, height / 2.0 + tand, -1.0))
                    * Mat4::from_scale(Vec3::new(width, height, 1.));
            let view_mat = Mat4::from_quat(view_params.reprojection_rotation).inverse();
            let proj_mat = super::projection_from_fov(view_params.fov);

            let transform = proj_mat * view_mat * model_mat;

            let transform_bytes = transform
                .to_cols_array()
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<u8>>();

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, &transform_bytes);
            render_pass.set_push_constants(
                ShaderStages::VERTEX_FRAGMENT,
                64,
                &(view_idx as u32).to_le_bytes(),
            );
            render_pass.set_bind_group(0, &self.views_objects[view_idx].bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }

        self.context.queue.submit(iter::once(encoder.finish()));
    }
}

pub fn foveated_encoding_shader_constants(
    expanded_view_resolution: UVec2,
    config: FoveatedEncodingConfig,
) -> (UVec2, HashMap<String, f64>) {
    let view_resolution = expanded_view_resolution.as_vec2();

    let center_size = glam::vec2(config.center_size_x, config.center_size_y);
    let center_shift = glam::vec2(config.center_shift_x, config.center_shift_y);
    let edge_ratio = glam::vec2(config.edge_ratio_x, config.edge_ratio_y);

    let edge_size = view_resolution - center_size * view_resolution;
    let center_size_aligned =
        1. - (edge_size / (edge_ratio * 2.)).ceil() * (edge_ratio * 2.) / view_resolution;

    let edge_size_aligned = view_resolution - center_size_aligned * view_resolution;
    let center_shift_aligned = (center_shift * edge_size_aligned / (edge_ratio * 2.)).ceil()
        * (edge_ratio * 2.)
        / edge_size_aligned;

    let foveation_scale = center_size_aligned + (1. - center_size_aligned) / edge_ratio;

    let optimized_view_resolution = foveation_scale * view_resolution;

    let optimized_view_resolution_aligned =
        optimized_view_resolution.map(|v| (v / 32.).ceil() * 32.);

    let view_ratio_aligned = optimized_view_resolution / optimized_view_resolution_aligned;

    let c0 = (1. - center_size_aligned) * 0.5;
    let c1 = (edge_ratio - 1.) * c0 * (center_shift_aligned + 1.) / edge_ratio;
    let c2 = (edge_ratio - 1.) * center_size_aligned + 1.;

    let lo_bound = c0 * (center_shift_aligned + 1.);
    let hi_bound = c0 * (center_shift_aligned - 1.) + 1.;
    let lo_bound_c = c0 * (center_shift_aligned + 1.) / c2;
    let hi_bound_c = c0 * (center_shift_aligned - 1.) / c2 + 1.;

    let a_left = c2 * (1. - edge_ratio) / (edge_ratio * lo_bound_c);
    let b_left = (c1 + c2 * lo_bound_c) / lo_bound_c;

    let a_right = c2 * (edge_ratio - 1.) / (edge_ratio * (1. - hi_bound_c));
    let b_right = (c2 - edge_ratio * c1 - 2. * edge_ratio * c2
        + c2 * edge_ratio * (1. - hi_bound_c)
        + edge_ratio)
        / (edge_ratio * (1. - hi_bound_c));
    let c_right = (c2 * edge_ratio - c2) * (c1 - hi_bound_c + c2 * hi_bound_c)
        / (edge_ratio * (1. - hi_bound_c) * (1. - hi_bound_c));

    let constants = [
        ("ENABLE_FFE", 1.),
        ("VIEW_WIDTH_RATIO", view_ratio_aligned.x),
        ("VIEW_HEIGHT_RATIO", view_ratio_aligned.y),
        ("EDGE_X_RATIO", edge_ratio.x),
        ("EDGE_Y_RATIO", edge_ratio.y),
        ("C1_X", c1.x),
        ("C1_Y", c1.y),
        ("C2_X", c2.x),
        ("C2_Y", c2.y),
        ("LO_BOUND_X", lo_bound.x),
        ("LO_BOUND_Y", lo_bound.y),
        ("HI_BOUND_X", hi_bound.x),
        ("HI_BOUND_Y", hi_bound.y),
        ("A_LEFT_X", a_left.x),
        ("A_LEFT_Y", a_left.y),
        ("B_LEFT_X", b_left.x),
        ("B_LEFT_Y", b_left.y),
        ("A_RIGHT_X", a_right.x),
        ("A_RIGHT_Y", a_right.y),
        ("B_RIGHT_X", b_right.x),
        ("B_RIGHT_Y", b_right.y),
        ("C_RIGHT_X", c_right.x),
        ("C_RIGHT_Y", c_right.y),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), *v as f64))
    .collect();

    (optimized_view_resolution_aligned.as_uvec2(), constants)
}
