//! wgpu rendering of the scene, both to the on-screen view and to offscreen targets that back the
//! HTTP capture endpoints.
//!
//! The renderer borrows eframe's wgpu device rather than creating its own, so there is no
//! cross-device or cross-API texture sharing to get wrong.

use crate::{
    camera::{Camera, Eye},
    scene::Scene,
};
use alvr_common::glam::Mat4;
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU32;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompareFunction, DepthStencilState, Device, Extent3d, FilterMode,
    FragmentState, FrontFace, IndexFormat, LoadOp, MipmapFilterMode, MultisampleState, Operations,
    Origin3d,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StoreOp, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// wgpu requires buffer copy rows to be aligned to 256 bytes.
const COPY_ALIGNMENT: u32 = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

/// Stride between the two eyes' uniforms in the shared buffer.
///
/// Dynamic offsets must be a multiple of `min_uniform_buffer_offset_alignment`, whose maximum
/// permitted value across backends is 256, so using 256 is portable without querying limits.
const UNIFORM_STRIDE: u64 = 256;

/// GPU resources for one loaded primitive.
struct GpuPrimitive {
    index_buffer: Buffer,
    index_count: u32,
    /// Bind group carrying this primitive's texture. The uniform buffer is shared between
    /// primitives and between eyes, selected with a dynamic offset at draw time.
    bind_group: BindGroup,
}

/// The unlit textured-mesh pipeline and the shared resources every mesh's bind groups reference.
///
/// The scene and the controller models render identically, so they share this one definition; only
/// the geometry and the matrices differ.
struct MeshPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// A 1x1 white texture standing in for materials without a base colour texture, so every
    /// primitive can share one bind group layout.
    placeholder_view: TextureView,
}

impl MeshPipeline {
    fn new(device: &Device, queue: &Queue, output_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("mesh bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        // The eye is selected by dynamic offset, so one bind group serves both.
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniforms>() as u64
                        ),
                    },
                    count: None,
                },
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
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("mesh pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("mesh pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<crate::scene::Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 12,
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 20,
                            shader_location: 2,
                        },
                    ],
                }],
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
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                // Room scans are frequently single-sided or inconsistently wound, and back-face
                // culling would punch holes in the walls when viewed from inside.
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("base colour sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        let placeholder_view = create_placeholder_texture(device, queue);

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            placeholder_view,
        }
    }
}

/// GPU form of a [`Scene`]: vertex and index buffers plus one uniform slot per eye.
struct Mesh {
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    primitives: Vec<GpuPrimitive>,
}

impl Mesh {
    fn new(device: &Device, queue: &Queue, pipeline: &MeshPipeline, scene: &Scene) -> Self {
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("mesh vertices"),
            size: (scene.vertices.len() * std::mem::size_of::<crate::scene::Vertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&scene.vertices));

        // Two slots, one per eye, so a stereo pass can draw both without rewriting between them.
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("mesh uniforms"),
            size: UNIFORM_STRIDE * 2,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut primitives = Vec::new();
        for primitive in &scene.primitives {
            let index_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("mesh indices"),
                size: (primitive.indices.len() * std::mem::size_of::<u32>()) as u64,
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&primitive.indices));

            let texture_view = primitive
                .texture
                .as_ref()
                .map(|texture| upload_texture(device, queue, texture));

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("mesh bind group"),
                layout: &pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        // Sized to one eye's uniforms; the dynamic offset picks which.
                        resource: BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &uniform_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(std::mem::size_of::<Uniforms>() as u64),
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(
                            texture_view.as_ref().unwrap_or(&pipeline.placeholder_view),
                        ),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&pipeline.sampler),
                    },
                ],
            });

            primitives.push(GpuPrimitive {
                index_buffer,
                index_count: primitive.indices.len() as u32,
                bind_group,
            });
        }

        Self {
            vertex_buffer,
            uniform_buffer,
            primitives,
        }
    }

    /// Uploads a combined matrix into the given eye's uniform slot.
    ///
    /// Both eyes can be written before a pass begins, which is what lets a stereo pass draw them
    /// both without an intervening buffer write.
    fn set_view(&self, queue: &Queue, eye: Eye, matrix: Mat4) {
        queue.write_buffer(
            &self.uniform_buffer,
            eye_offset(eye),
            bytemuck::bytes_of(&Uniforms {
                view_proj: matrix.to_cols_array_2d(),
            }),
        );
    }

    /// Records draw commands for one eye into an existing render pass. The pipeline must already
    /// be set.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, eye: Eye) {
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        let offset = eye_offset(eye) as u32;

        for primitive in &self.primitives {
            pass.set_bind_group(0, &primitive.bind_group, &[offset]);
            pass.set_index_buffer(primitive.index_buffer.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..primitive.index_count, 0, 0..1);
        }
    }
}

pub struct SceneRenderer {
    pipeline: MeshPipeline,
    mesh: Mesh,
    /// Colour format this pipeline was built for. Offscreen capture targets must use the same
    /// format, or the pipeline fails render pass validation.
    color_format: TextureFormat,
}

impl SceneRenderer {
    pub fn new(device: &Device, queue: &Queue, scene: &Scene, output_format: TextureFormat) -> Self {
        let pipeline = MeshPipeline::new(device, queue, output_format);
        let mesh = Mesh::new(device, queue, &pipeline, scene);

        Self {
            pipeline,
            mesh,
            color_format: output_format,
        }
    }

    /// Records draw commands for one eye into an existing render pass.
    ///
    /// Untextured primitives bind a 1x1 white placeholder rather than taking a separate code path.
    /// Because the material colour factor is baked into the vertex colours, multiplying by white
    /// leaves it unchanged, so a single uniform pipeline handles both cases correctly.
    /// The pass lifetime is deliberately independent of `&self`: wgpu 29's pass commands take
    /// ref-counted resource handles rather than borrows, so tying the two together would force
    /// egui's `CallbackResources` borrow to outlive the paint callback.
    pub fn draw<'pass>(&self, pass: &mut wgpu::RenderPass<'pass>, eye: Eye) {
        pass.set_pipeline(&self.pipeline.pipeline);
        self.mesh.draw(pass, eye);
    }

    /// Uploads the view-projection matrix into the given eye's uniform slot.
    pub fn set_view(&self, queue: &Queue, camera: &Camera, eye: Eye, aspect_ratio: f32) {
        let view_proj = Camera::projection_matrix(aspect_ratio) * camera.view_matrix(eye);
        self.mesh.set_view(queue, eye, view_proj);
    }
}

/// Renders the emulated controllers' 3D models into the scene view.
///
/// Independent of [`SceneRenderer`] so controllers can be shown even while the environment failed
/// to load. Each hand has its own mesh, swapped at runtime when the selected profile changes.
pub struct ControllerRenderer {
    pipeline: MeshPipeline,
    models: [Option<Mesh>; 2],
}

impl ControllerRenderer {
    pub fn new(device: &Device, queue: &Queue, output_format: TextureFormat) -> Self {
        Self {
            pipeline: MeshPipeline::new(device, queue, output_format),
            models: [None, None],
        }
    }

    /// Uploads a controller model for one hand, replacing the previous one.
    pub fn set_model(&mut self, device: &Device, queue: &Queue, hand: usize, scene: &Scene) {
        self.models[hand] = Some(Mesh::new(device, queue, &self.pipeline, scene));
    }

    /// Uploads the combined matrix (projection * view * model) for one hand and eye.
    pub fn set_view(&self, queue: &Queue, hand: usize, eye: Eye, matrix: Mat4) {
        if let Some(mesh) = &self.models[hand] {
            mesh.set_view(queue, eye, matrix);
        }
    }

    /// Records draw commands for one hand and eye into an existing render pass. Nothing is drawn
    /// until a model was uploaded.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, hand: usize, eye: Eye) {
        if let Some(mesh) = &self.models[hand] {
            pass.set_pipeline(&self.pipeline.pipeline);
            mesh.draw(pass, eye);
        }
    }
}

/// Byte offset of an eye's slot within the shared uniform buffer.
fn eye_offset(eye: Eye) -> u64 {
    match eye {
        Eye::Left => 0,
        Eye::Right => UNIFORM_STRIDE,
    }
}

fn create_placeholder_texture(device: &Device, queue: &Queue) -> TextureView {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("placeholder base colour"),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &[255, 255, 255, 255],
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    texture.create_view(&TextureViewDescriptor::default())
}

fn upload_texture(device: &Device, queue: &Queue, texture: &crate::scene::Texture) -> TextureView {
    let size = Extent3d {
        width: texture.width,
        height: texture.height,
        depth_or_array_layers: 1,
    };

    let gpu_texture = device.create_texture(&TextureDescriptor {
        label: Some("base colour"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        // glTF base colour textures are sRGB encoded.
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        TexelCopyTextureInfo {
            texture: &gpu_texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &texture.pixels,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(texture.width * 4),
            rows_per_image: Some(texture.height),
        },
        size,
    );

    gpu_texture.create_view(&TextureViewDescriptor::default())
}

/// An offscreen colour + depth target used by the capture endpoints.
pub struct CaptureTarget {
    pub width: u32,
    pub height: u32,
    color: Texture,
    color_view: TextureView,
    depth: Texture,
    depth_view: TextureView,
}

impl CaptureTarget {
    pub fn new(device: &Device, width: u32, height: u32, color_format: TextureFormat) -> Self {
        let color = device.create_texture(&TextureDescriptor {
            label: Some("capture colour"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: color_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let depth = device.create_texture(&TextureDescriptor {
            label: Some("capture depth"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        Self {
            width,
            height,
            color_view: color.create_view(&TextureViewDescriptor::default()),
            depth_view: depth.create_view(&TextureViewDescriptor::default()),
            color,
            depth,
        }
    }
}

/// What a capture should return.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    Color,
    Depth,
}

/// Renders both eyes side by side into an offscreen target and reads the result back.
///
/// Visible controller models are drawn into the capture as well, so the endpoints show the same
/// scene as the window. Each entry of the pose array is a controller's world-space model matrix.
///
/// Returns tightly packed pixels: RGBA8 for colour, or 8-bit greyscale for depth.
#[expect(clippy::too_many_arguments)]
pub fn capture_stereo(
    device: &Device,
    queue: &Queue,
    renderer: &SceneRenderer,
    controllers: Option<(&ControllerRenderer, [Option<Mat4>; 2])>,
    camera: &Camera,
    eye_width: u32,
    eye_height: u32,
    kind: CaptureKind,
) -> Vec<u8> {
    let target = CaptureTarget::new(device, eye_width * 2, eye_height, renderer.color_format);
    let aspect_ratio = eye_width as f32 / eye_height as f32;

    // Both eyes have their own uniform slot, so a single pass can draw them into two viewports.
    renderer.set_view(queue, camera, Eye::Left, aspect_ratio);
    renderer.set_view(queue, camera, Eye::Right, aspect_ratio);

    if let Some((controller_renderer, models)) = &controllers {
        for eye in [Eye::Left, Eye::Right] {
            let view_proj = Camera::projection_matrix(aspect_ratio) * camera.view_matrix(eye);

            for (hand, model) in models.iter().enumerate() {
                if let Some(model) = model {
                    controller_renderer.set_view(queue, hand, eye, view_proj * *model);
                }
            }
        }
    }

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("capture encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("capture pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &target.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.02,
                        g: 0.02,
                        b: 0.03,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &target.depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        for (index, eye) in [Eye::Left, Eye::Right].into_iter().enumerate() {
            pass.set_viewport(
                (index as u32 * eye_width) as f32,
                0.0,
                eye_width as f32,
                eye_height as f32,
                0.0,
                1.0,
            );

            renderer.draw(&mut pass, eye);

            if let Some((controller_renderer, models)) = &controllers {
                for (hand, model) in models.iter().enumerate() {
                    if model.is_some() {
                        controller_renderer.draw(&mut pass, hand, eye);
                    }
                }
            }
        }
    }

    queue.submit([encoder.finish()]);

    match kind {
        CaptureKind::Color => {
            let mut pixels =
                read_back(device, queue, &target.color, target.width, target.height, 4);

            // The pipeline renders in the surface format, which is typically BGRA on Windows, but
            // PNG needs RGBA. Swap the channels rather than maintaining a second pipeline.
            if matches!(
                renderer.color_format,
                TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
            ) {
                for pixel in pixels.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }
            }

            pixels
        }
        CaptureKind::Depth => {
            let raw = read_back(device, queue, &target.depth, target.width, target.height, 4);
            linearize_depth(&raw)
        }
    }
}

/// Copies a texture to a mapped buffer and returns tightly packed rows.
fn read_back(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
) -> Vec<u8> {
    let unpadded_row = width * bytes_per_pixel;
    // Buffer copy rows must be 256-byte aligned, so the readback buffer is padded and the padding
    // stripped after mapping.
    let padded_row = unpadded_row.div_ceil(COPY_ALIGNMENT) * COPY_ALIGNMENT;

    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("readback"),
        size: (padded_row * height) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("readback encoder"),
    });

    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: NonZeroU32::new(height).map(|value| value.get()),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| ());
    // Blocking is acceptable: captures are on-demand from the HTTP thread, not per frame.
    // Both fields None waits indefinitely for the most recent submission, which is what we want.
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .ok();

    let mapped = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((unpadded_row * height) as usize);
    for row in 0..height {
        let start = (row * padded_row) as usize;
        pixels.extend_from_slice(&mapped[start..start + unpadded_row as usize]);
    }

    drop(mapped);
    buffer.unmap();

    pixels
}

/// Converts a Depth32Float buffer into an 8-bit greyscale ramp.
///
/// Two transformations are needed to make this readable. First the perspective depth is inverted to
/// recover a view-space distance, because raw depth values cluster hard against 1.0. Then the result
/// is normalised across the range actually present in the image rather than across the clip range:
/// a room a few metres across occupies a tiny fraction of the 0.02..100 m frustum, so scaling by the
/// clip range collapses the whole scene into a handful of near-white values.
///
/// Near surfaces read bright and distant ones dark. Pixels where nothing was drawn are black.
fn linearize_depth(raw: &[u8]) -> Vec<u8> {
    let near = Camera::near_clip();
    let far = Camera::far_clip();

    // Recover view-space distances, tracking the range so the ramp can be fitted to it.
    let mut min_distance = f32::MAX;
    let mut max_distance = f32::MIN;

    let distances = raw
        .chunks_exact(4)
        .map(|bytes| {
            let depth = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

            if depth >= 1.0 {
                return None;
            }

            // Inverse of the wgpu 0..1 perspective depth mapping.
            let distance = (near * far) / (far - depth * (far - near));
            min_distance = min_distance.min(distance);
            max_distance = max_distance.max(distance);

            Some(distance)
        })
        .collect::<Vec<_>>();

    // A single-depth or empty image has no range to stretch across; avoid dividing by zero.
    let span = max_distance - min_distance;
    let scale = if span > f32::EPSILON { 1.0 / span } else { 0.0 };

    distances
        .into_iter()
        .map(|distance| match distance {
            Some(distance) => {
                let normalized = ((distance - min_distance) * scale).clamp(0.0, 1.0);
                // Reserve 0 for "nothing drawn", so geometry starts at 1.
                1 + (254.0 * (1.0 - normalized)) as u8
            }
            None => 0,
        })
        .collect()
}
