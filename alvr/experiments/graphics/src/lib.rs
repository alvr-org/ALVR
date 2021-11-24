pub mod convert;
pub mod foveated_rendering;
pub mod slicing;

use ash::vk;
use std::sync::Arc;
use wgpu::{
    Adapter, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Color,
    ColorTargetState, ColorWrites, CommandEncoder, Device, FilterMode, FragmentState, Instance,
    LoadOp, MultisampleState, Operations, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureFormat, TextureView, VertexState,
};

pub const TARGET_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

pub struct GraphicsContext {
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub raw_instance: ash::Instance,
    pub raw_physical_device: vk::PhysicalDevice,
    pub raw_device: ash::Device,
    pub queue_family_index: u32,
    pub queue_index: u32,
}

pub fn quad_shader(device: &Device) -> ShaderModule {
    device.create_shader_module(&ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(include_str!("../resources/quad.wgsl").into()),
    })
}

pub fn create_default_render_pipeline(device: &Device, fragment_shader: &str) -> RenderPipeline {
    let quad_shader = quad_shader(device);

    let fragment_shader = device.create_shader_module(&ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(fragment_shader.into()),
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
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
                blend: None,
                write_mask: ColorWrites::ALL,
            }],
        }),
    })
}

pub fn create_default_sampler(device: &Device) -> Sampler {
    device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..Default::default()
    })
}

pub fn create_default_bind_group_with_sampler(
    device: &Device,
    pipeline: &RenderPipeline,
    texture_view: &TextureView,
    sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(sampler),
            },
        ],
    })
}

pub fn create_default_bind_group(
    device: &Device,
    pipeline: &RenderPipeline,
    texture_view: &TextureView,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(texture_view),
        }],
    })
}

pub fn execute_default_pass(
    encoder: &mut CommandEncoder,
    pipeline: &RenderPipeline,
    bind_group: &BindGroup,
    push_constants: &[u8],
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

    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.set_push_constants(ShaderStages::FRAGMENT, 0, push_constants);

    pass.draw(0..4, 0..1);

    // here the pass is dropped and applied to the command encoder
}
