use crate::compositor::TARGET_FORMAT;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BlendState, Color,
    ColorTargetState, ColorWrites, CommandEncoder, Device, FilterMode, FragmentState, LoadOp,
    MultisampleState, Operations, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, Texture, TextureView, TextureViewDescriptor, VertexState,
};

pub struct Layer<'a> {
    pub bind_group: &'a BindGroup,
    pub rect: openxr_sys::Rect2Di,
}

// Crop and render frames on top of each other, in the specified order
// todo: the compositor should support reprojection, in case layers are submitted with different
// poses
pub struct CompositingPipeline {
    inner: RenderPipeline,
    sampler: Sampler,
}

impl CompositingPipeline {
    pub fn new(device: &Device, quad_vertex_state: VertexState) -> Self {
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("../../resources/compositing.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: quad_vertex_state,
            primitive: Default::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format: TARGET_FORMAT,
                    blend: Some(BlendState::ALPHA_BLENDING), // todo: check if correct
                    write_mask: ColorWrites::ALL,
                }],
            }),
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
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
                    binding: 0,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    // Crop and render frames on top of each other, in the specified order
    pub fn draw<'a>(
        &self,
        encoder: &mut CommandEncoder,
        layers: impl Iterator<Item = Layer<'a>>,
        output: &TextureView,
    ) {
        for layer in layers {
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
            pass.set_bind_group(0, layer.bind_group, &[]);

            let rect_f32 = [
                layer.rect.offset.x as f32,
                layer.rect.offset.y as f32,
                layer.rect.extent.width as f32,
                layer.rect.extent.height as f32,
            ];
            pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::cast_slice(&rect_f32));

            super::draw_quad(pass);
        }
    }
}
