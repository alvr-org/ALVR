//! Renders decoded video frames.
//!
//! Uploads the three YUV planes as single-channel textures and converts to RGB in a shader. Keeping
//! the conversion on the GPU means the per-frame CPU cost is just the upload, and it is also where a
//! future zero-copy decoder would plug in: only [`VideoRenderer::upload`] would change, not the
//! drawing.

use crate::{
    camera::Eye,
    decoder::{ColorRange, DecodedFrame},
};
use bytemuck::{Pod, Zeroable};
use std::time::Duration;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites,
    Device, Extent3d, FilterMode, FragmentState, MipmapFilterMode, MultisampleState, Origin3d,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue, RenderPipeline,
    RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    /// `(offset_x, offset_y, scale_x, scale_y)` in UV space.
    region: [f32; 4],
    /// 1.0 when the samples use the full 0-255 range, 0.0 for the limited broadcast range.
    full_range: f32,
    _padding: [f32; 3],
}

/// Dynamic offset stride, matching the alignment requirement used elsewhere.
const UNIFORM_STRIDE: u64 = 256;

/// How the stream's frame is laid out.
///
/// ALVR packs both eyes into one encoded frame. Which way round depends on the aspect ratio, so it
/// is inferred from the decoded frame rather than assumed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrameLayout {
    /// Left eye in the left half, right eye in the right half.
    SideBySide,
    /// A single view covering the whole frame.
    Single,
}

/// The planes of one decoded frame, resident on the GPU.
struct Planes {
    y: Texture,
    u: Texture,
    v: Texture,
    width: u32,
    height: u32,
}

pub struct VideoRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    uniform_buffer: Buffer,
    sampler: wgpu::Sampler,
    planes: Option<Planes>,
    bind_group: Option<BindGroup>,
    layout: FrameLayout,
    range: ColorRange,
    /// Timestamp of the frame currently uploaded, for the statistics reports.
    current_timestamp: Duration,
    frames_shown: u64,
}

impl VideoRenderer {
    pub fn new(device: &Device, output_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("video.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("video bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    // The vertex stage reads the region, the fragment stage the colour range.
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniforms>() as u64
                        ),
                    },
                    count: None,
                },
                // The three planes share one layout entry shape, differing only in binding index.
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("video pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("video pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                // The quad is generated from the vertex index, so there is no vertex buffer.
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: output_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            // Video needs no depth testing, but the pass it draws into has a depth attachment (the
            // scene pipeline requires one), and a pipeline whose depth state does not match the
            // pass fails validation. So the format is declared while testing and writing stay off.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("video uniforms"),
            size: UNIFORM_STRIDE * 2,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("video sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            sampler,
            planes: None,
            bind_group: None,
            layout: FrameLayout::Single,
            range: ColorRange::Limited,
            current_timestamp: Duration::ZERO,
            frames_shown: 0,
        }
    }

    /// Whether a frame has ever been uploaded, so the caller can keep showing the scene until the
    /// first one arrives.
    pub fn has_frame(&self) -> bool {
        self.planes.is_some()
    }

    pub fn current_timestamp(&self) -> Duration {
        self.current_timestamp
    }

    pub fn frames_shown(&self) -> u64 {
        self.frames_shown
    }

    pub fn layout(&self) -> FrameLayout {
        self.layout
    }

    /// Uploads a decoded frame, replacing whatever was shown before.
    pub fn upload(&mut self, device: &Device, queue: &Queue, frame: &DecodedFrame) {
        let DecodedFrame::Yuv420 {
            timestamp,
            width,
            height,
            range,
            y,
            y_stride,
            u,
            v,
            uv_stride,
        } = frame;

        // Chroma is half resolution in both axes. Rounded up so odd dimensions do not truncate.
        let chroma_width = width.div_ceil(2);
        let chroma_height = height.div_ceil(2);

        // Textures are only recreated when the frame size changes, which in practice means once.
        let needs_alloc = self
            .planes
            .as_ref()
            .is_none_or(|planes| planes.width != *width || planes.height != *height);

        if needs_alloc {
            self.planes = Some(Planes {
                y: create_plane(device, *width, *height, "video Y"),
                u: create_plane(device, chroma_width, chroma_height, "video U"),
                v: create_plane(device, chroma_width, chroma_height, "video V"),
                width: *width,
                height: *height,
            });
            self.bind_group = None;

            // Both eyes side by side gives a frame twice as wide as it is tall, relative to the
            // per-eye aspect. Anything squarer is a single view.
            self.layout = if *width >= height * 2 {
                FrameLayout::SideBySide
            } else {
                FrameLayout::Single
            };

            alvr_common::info!(
                "Video frame is {width}x{height}, treated as {:?}",
                self.layout
            );
        }

        let planes = self.planes.as_ref().unwrap();

        write_plane(queue, &planes.y, *width, *height, y, *y_stride);
        write_plane(queue, &planes.u, chroma_width, chroma_height, u, *uv_stride);
        write_plane(queue, &planes.v, chroma_width, chroma_height, v, *uv_stride);

        if self.bind_group.is_none() {
            let view = |texture: &Texture| texture.create_view(&TextureViewDescriptor::default());

            self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
                label: Some("video bind group"),
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.uniform_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(std::mem::size_of::<Uniforms>() as u64),
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&view(&planes.y)),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&view(&planes.u)),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&view(&planes.v)),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));
        }

        self.current_timestamp = *timestamp;
        self.range = *range;
        self.frames_shown += 1;
    }

    /// Writes the sub-rectangle each eye should sample. Call before beginning the render pass.
    pub fn set_regions(&self, queue: &Queue) {
        for (index, eye) in [Eye::Left, Eye::Right].into_iter().enumerate() {
            let region = match (self.layout, eye) {
                (FrameLayout::SideBySide, Eye::Left) => [0.0, 0.0, 0.5, 1.0],
                (FrameLayout::SideBySide, Eye::Right) => [0.5, 0.0, 0.5, 1.0],
                // A single view is shown to both eyes.
                (FrameLayout::Single, _) => [0.0, 0.0, 1.0, 1.0],
            };

            queue.write_buffer(
                &self.uniform_buffer,
                index as u64 * UNIFORM_STRIDE,
                bytemuck::bytes_of(&Uniforms {
                    region,
                    full_range: (self.range == ColorRange::Full) as u32 as f32,
                    _padding: [0.0; 3],
                }),
            );
        }
    }

    /// Draws one eye's view of the current frame. Does nothing until a frame has been uploaded.
    pub fn draw<'pass>(&self, pass: &mut wgpu::RenderPass<'pass>, eye: Eye) {
        let Some(bind_group) = &self.bind_group else {
            return;
        };

        let offset = match eye {
            Eye::Left => 0,
            Eye::Right => UNIFORM_STRIDE as u32,
        };

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[offset]);
        pass.draw(0..6, 0..1);
    }
}

fn create_plane(device: &Device, width: u32, height: u32, label: &str) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        // Single channel: the shader combines the three planes itself.
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

/// Uploads one plane, honouring the decoder's row stride.
fn write_plane(queue: &Queue, texture: &Texture, width: u32, height: u32, data: &[u8], stride: u32) {
    // ffmpeg pads rows for alignment, so the source stride is usually larger than the width. Passing
    // it through avoids a CPU repack; a mismatch here shows up as a skewed image.
    let expected = (stride * height) as usize;
    let data = if data.len() >= expected {
        &data[..expected]
    } else {
        // Short buffer: upload nothing rather than read out of bounds.
        return;
    };

    queue.write_texture(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        data,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride),
            rows_per_image: Some(height),
        },
        Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
    );
}
