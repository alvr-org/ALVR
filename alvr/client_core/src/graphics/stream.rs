use super::{staging::StagingRenderer, GraphicsContext};
use alvr_common::glam::UVec2;
use alvr_session::FoveatedEncodingConfig;
use std::{ffi::c_void, iter, rc::Rc};
use wgpu::{
    hal::{api, gles},
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, ColorTargetState, ColorWrites,
    FragmentState, LoadOp, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages, StoreOp,
    TextureSampleType, TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};

#[derive(Debug)]
struct ViewObjects {
    bind_group: BindGroup,
    render_target: Vec<TextureView>,
}

struct RenderObjects {
    staging_renderer: StagingRenderer,
    pipeline: RenderPipeline,
    views_objects: [ViewObjects; 2],
}

pub struct StreamRenderer {
    context: Rc<GraphicsContext>,
    render_objects: Option<RenderObjects>,
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
    ) -> Self {
        // if ffe is enabled, use old c++ code until it is rewritten
        #[allow(unused_variables)]
        if let Some(fe) = &foveated_encoding {
            context.make_current();

            #[cfg(all(target_os = "android", feature = "use-cpp"))]
            unsafe {
                let config = super::opengl::FfiStreamConfig {
                    viewWidth: view_resolution.x,
                    viewHeight: view_resolution.y,
                    swapchainTextures: [
                        swapchain_textures[0].as_ptr(),
                        swapchain_textures[1].as_ptr(),
                    ],
                    swapchainLength: swapchain_textures[0].len() as _,
                    enableFoveation: 1,
                    foveationCenterSizeX: fe.center_size_x,
                    foveationCenterSizeY: fe.center_size_y,
                    foveationCenterShiftX: fe.center_shift_x,
                    foveationCenterShiftY: fe.center_shift_y,
                    foveationEdgeRatioX: fe.edge_ratio_x,
                    foveationEdgeRatioY: fe.edge_ratio_y,
                    enableSrgbCorrection: enable_srgb_correction as u32,
                    fixLimitedRange: fix_limited_range as u32,
                    encodingGamma: encoding_gamma,
                };

                super::opengl::streamStartNative(config);
            }

            Self {
                context,
                render_objects: None,
            }
        } else {
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
            let constants = &[
                (
                    "ENABLE_SRGB_CORRECTION".into(),
                    enable_srgb_correction.into(),
                ),
                ("FIX_LIMITED_RANGE".into(), fix_limited_range.into()),
                ("ENCODING_GAMMA".into(), encoding_gamma.into()),
            ]
            .into_iter()
            .collect();
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                // Note: Layout cannot be inferred because of a bug with push constants
                layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                })),
                vertex: VertexState {
                    module: &shader_module,
                    entry_point: "vertex_main",
                    compilation_options: PipelineCompilationOptions {
                        constants,
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
                        constants,
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
                let staging_texture = super::create_texture(device, view_resolution, target_format);

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
                view_resolution,
            );

            Self {
                context,
                render_objects: Some(RenderObjects {
                    staging_renderer,
                    pipeline,
                    views_objects: view_objects.try_into().unwrap(),
                }),
            }
        }
    }

    #[allow(unused_variables)]
    pub unsafe fn render(&self, hardware_buffer: *mut c_void, swapchain_indices: [u32; 2]) {
        if let Some(render_objects) = &self.render_objects {
            // if hardware_buffer is available copy stream to staging texture
            if !hardware_buffer.is_null() {
                render_objects.staging_renderer.render(hardware_buffer);
            }

            let mut encoder = self
                .context
                .device
                .create_command_encoder(&Default::default());

            for (view_idx, swapchain_idx) in swapchain_indices.iter().enumerate() {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &render_objects.views_objects[view_idx].render_target
                            [*swapchain_idx as usize],
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: LoadOp::Clear(wgpu::Color::BLACK),
                            store: StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });

                render_pass.set_pipeline(&render_objects.pipeline);
                render_pass.set_bind_group(
                    0,
                    &render_objects.views_objects[view_idx].bind_group,
                    &[],
                );
                render_pass.draw(0..4, 0..1);
            }

            self.context.queue.submit(iter::once(encoder.finish()));
        } else {
            self.context.make_current();

            #[cfg(all(target_os = "android", feature = "use-cpp"))]
            super::opengl::renderStreamNative(hardware_buffer, swapchain_indices.as_ptr());
        }
    }
}

impl Drop for StreamRenderer {
    fn drop(&mut self) {
        self.context.make_current();

        #[cfg(all(target_os = "android", feature = "use-cpp"))]
        if self.render_objects.is_none() {
            unsafe { super::opengl::destroyStream() };
        }
    }
}
