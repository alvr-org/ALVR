use alvr_common::glam::UVec2;
use alvr_graphics::{
    wgpu::{
        AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BlendState,
        Color, ColorTargetState, ColorWrites, CommandEncoder, Device, FilterMode, FragmentState,
        LoadOp, MultisampleState, Operations, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerDescriptor,
        ShaderModuleDescriptor, ShaderSource, ShaderStages, Texture, TextureView,
        TextureViewDescriptor, VertexState,
    },
    TARGET_FORMAT,
};
use std::sync::Arc;

pub struct Layer<'a> {
    pub bind_group: &'a BindGroup,
    pub rect_offset: UVec2,
    pub rect_size: UVec2,
}

// Crop and render layers on top of each other, in the specified order
// todo: the compositor should support reprojection, in case layers are submitted with different
// poses
pub struct CompositingPass {
    device: Arc<Device>,
    inner: RenderPipeline,
    sampler: Sampler,
}

impl CompositingPass {
    pub fn new(device: Arc<Device>) -> Self {
        let quad_shader = alvr_graphics::quad_shader(&device); // quad shader should be replaced with rotating shader

        let fragment_shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("../resources/compositing.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: VertexState {
                module: &quad_shader,
                entry_point: "main",
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format: TARGET_FORMAT,
                    blend: Some(BlendState::ALPHA_BLENDING), // todo: check if correct
                    write_mask: ColorWrites::ALL,
                }],
            }),
            multiview: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
            device,
            inner: pipeline,
            sampler,
        }
    }

    pub fn create_bind_group(
        &self,
        device: &Device,
        texture: &Texture,
        array_index: u32,
    ) -> BindGroup {
        let view = texture.create_view(&TextureViewDescriptor {
            base_array_layer: array_index,
            ..Default::default()
        });

        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.inner.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub fn draw<'a>(
        &self,
        encoder: &mut CommandEncoder,
        layers: impl Iterator<Item = Layer<'a>>,
        output: &TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            }],
            ..Default::default()
        });

        pass.set_pipeline(&self.inner);

        for layer in layers {
            pass.set_bind_group(0, layer.bind_group, &[]);

            let rect_f32 = [
                layer.rect_offset.x as f32,
                layer.rect_offset.y as f32,
                layer.rect_size.x as f32,
                layer.rect_size.y as f32,
            ];
            pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::cast_slice(&rect_f32));

            pass.draw(0..4, 0..1);
        }
    }
}
