use crate::{GraphicsContext, VulkanBackend};
use alvr_common::{anyhow::Result, glam::UVec2};
use std::{ffi::c_void, num::NonZeroU32};
use wgpu::*;

struct InputResource {
    texture: Texture,
    bind_group: BindGroup,
}

pub struct ClientStreamRenderer<T: Clone> {
    context: GraphicsContext<T>,
    pipeline: RenderPipeline,
    input_resource_swapchain: Vec<InputResource>,
    output_swapchain: Vec<TextureView>,
}

impl ClientStreamRenderer<VulkanBackend> {
    pub fn new(
        context: GraphicsContext<VulkanBackend>,
        input_swapchain_len: usize,
        output_swapchain: Vec<Texture>,
        output_swapchain_resolution: UVec2,
        skip_srgb_correction: bool,
    ) -> Result<Self> {
        let label = Some("stream_renderer");

        // let output_swapchain = output_swapchain_gl
        //     .iter()
        //     .map(|&texture| {
        //         convert::create_texture_from_gles(&context.device, texture, output_view_resolution)
        //     })
        //     .collect::<Vec<_>>();

        // let client_stream_pass = ClientStreamPass::new(
        //     &context.device,
        //     input_swapchain_len,
        //     output_view_resolution,
        //     &output_swapchain,
        // );

        // let staging_swapchain = (0..output_swapchain_gl.len())
        //     .map(|idx| client_stream_pass.get_input_texture(idx))
        //     .collect::<Vec<_>>();

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    }],
                });

        let pipeline =
            context
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&context.device.create_pipeline_layout(
                        &PipelineLayoutDescriptor {
                            label,
                            bind_group_layouts: &[&bind_group_layout],
                            push_constant_ranges: &[],
                        },
                    )),
                    vertex: VertexState {
                        module: &context.device.create_shader_module(ShaderModuleDescriptor {
                            label,
                            source: ShaderSource::Wgsl(
                                include_str!("../shaders/client/render_vert.wgsl").into(),
                            ),
                        }),
                        entry_point: "main",
                        buffers: &[],
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        module: &context.device.create_shader_module(ShaderModuleDescriptor {
                            label,
                            source: ShaderSource::Wgsl(
                                include_str!("../shaders/client/render_frag.wgsl").into(),
                            ),
                        }),
                        entry_point: "main",
                        targets: &[Some(ColorTargetState {
                            format: TextureFormat::Rgba8UnormSrgb,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    multiview: Some(NonZeroU32::new(2).unwrap()),
                });

        let input_resource_swapchain = (0..input_swapchain_len)
            .map(|_| {
                let input = context.device.create_texture(&TextureDescriptor {
                    label,
                    // todo: size should be derived
                    size: Extent3d {
                        width: output_swapchain_resolution.x,
                        height: output_swapchain_resolution.y,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });

                let bind_group = context.device.create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &bind_group_layout,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &input.create_view(&Default::default()),
                        ),
                    }],
                });

                InputResource {
                    texture: input,
                    bind_group,
                }
            })
            .collect();

        let output_swapchain = output_swapchain
            .iter()
            .map(|output| output.create_view(&Default::default()))
            .collect();

        Ok(Self {
            context: context.clone(),
            pipeline,
            input_resource_swapchain,
            output_swapchain,
        })
    }

    // pub fn input_swapchain(&self) -> Vec<u32> {

    //         self.input_resource_swapchain.iter()
    //             .map(|input: &InputResource| input.texture.0.get())
    //             .collect::<Vec<_>>()
    // }

    /// # Safety
    /// if buffer must either be null or point to a valid AHardwareBuffer
    pub unsafe fn render_from_android_buffer(
        &mut self,
        buffer: *const c_void,
        input_swapchain_index: usize,
        output_swapchain_index: usize,
    ) {
        // let egl_image = if !buffer.is_null() {
        //     // if let Some(pass) = &self.srgb_pass {
        //     //     let gl_input_tex = pass.get_input_texture(input_swapchain_index);

        //     //     Some(
        //     //         self.context
        //     //             .bind_ahardwarebuffer_to_gl_ext_texture(buffer, gl_input_tex),
        //     //     )
        //     // } else {
        //         None
        //     // }
        // } else {
        //     None
        // };

        self.render(
            input_swapchain_index,
            output_swapchain_index,
            buffer.is_null(),
        );

        // if let Some(image) = egl_image {
        //     unsafe { self.context.destroy_image(image) };
        // }
    }

    pub fn render(
        &mut self,
        input_swapchain_index: usize,
        output_swapchain_index: usize,
        rerender_last: bool,
    ) {
        // if !rerender_last {
        //     if let Some(pass) = self.srgb_pass.take() {
        //         pass.render(input_swapchain_index)
        //     }
        // }

        self.render_no_color_correction(input_swapchain_index, output_swapchain_index)
    }
}

impl<T: Clone> ClientStreamRenderer<T> {
    pub fn render_no_color_correction(
        &mut self,
        input_swapchain_index: usize,
        output_swapchain_index: usize,
    ) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.output_swapchain[output_swapchain_index],
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(
            0,
            &self.input_resource_swapchain[input_swapchain_index].bind_group,
            &[],
        );
        // pass.set_push_constants(ShaderStages::VERTEX, offset, data)
        pass.draw(0..4, 0..1);

        drop(pass); // apply pass

        self.context.queue.submit(Some(encoder.finish()));
    }
}
