pub mod convert;
pub mod foveated_rendering;
pub mod slicing;

pub use ash;
pub use wgpu;
pub use wgpu_hal;

use ash::vk;
use std::{num::NonZeroU32, sync::Arc};
use wgpu::{
    Adapter, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Color,
    ColorTargetState, ColorWrites, CommandEncoder, Device, FilterMode, FragmentState, Instance,
    LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PushConstantRange, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    TextureFormat, TextureView, VertexState,
};

pub const TARGET_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
pub const QUAD_SHADER_WGSL: &str = include_str!("../resources/quad.wgsl");

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
        source: ShaderSource::Wgsl(QUAD_SHADER_WGSL.into()),
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

pub struct BindingDesc<'a> {
    pub index: u32,
    pub binding_type: BindingType,
    pub array_size: Option<usize>,
    pub resource: BindingResource<'a>,
}

// All bindings map to the bind group 0
pub fn create_default_render_pipeline(
    label: &str,
    device: &Device,
    fragment_shader: &str,
    bindings: Vec<BindingDesc>,
    push_constants_size: usize,
) -> (RenderPipeline, BindGroup) {
    let quad_shader = quad_shader(device);

    let fragment_shader = device.create_shader_module(&ShaderModuleDescriptor {
        label: Some(label),
        source: ShaderSource::Wgsl(fragment_shader.into()),
    });

    let layout_entries = bindings
        .iter()
        .map(|binding| BindGroupLayoutEntry {
            binding: binding.index,
            visibility: ShaderStages::FRAGMENT,
            ty: binding.binding_type,
            count: binding
                .array_size
                .map(|size| NonZeroU32::new(size as _).unwrap()),
        })
        .collect::<Vec<_>>();

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &layout_entries,
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..push_constants_size as _,
        }],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
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
        multiview: None,
    });

    let bind_group_entries = bindings
        .into_iter()
        .map(|binding| BindGroupEntry {
            binding: binding.index,
            resource: binding.resource,
        })
        .collect::<Vec<_>>();

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &bind_group_entries,
    });

    (pipeline, bind_group)
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
