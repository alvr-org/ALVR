use graphics::TARGET_FORMAT;
use wgpu::{
    BindGroup, BlendState, Color, ColorTargetState, ColorWrites, CommandEncoder, Device,
    FragmentState, LoadOp, MultisampleState, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Texture, TextureView,
    TextureViewDescriptor, VertexState,
};

pub struct Layer<'a> {
    pub bind_group: &'a BindGroup,
    pub rect: openxr_sys::Rect2Di,
}

// Crop and render layers on top of each other, in the specified order
// todo: the compositor should support reprojection, in case layers are submitted with different
// poses
pub struct CompositingPass {
    inner: RenderPipeline,
    sampler: Sampler,
}

impl CompositingPass {
    pub fn new(device: &Device) -> Self {
        let quad_shader = graphics::quad_shader(device);

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
        });

        let sampler = graphics::create_default_sampler(device);

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

        graphics::create_default_bind_group_with_sampler(device, &self.inner, &view, &self.sampler)
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
                layer.rect.offset.x as f32,
                layer.rect.offset.y as f32,
                layer.rect.extent.width as f32,
                layer.rect.extent.height as f32,
            ];
            pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::cast_slice(&rect_f32));

            pass.draw(0..4, 0..1);
        }
    }
}
