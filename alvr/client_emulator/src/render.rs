//! wgpu rendering of the scene, both to the on-screen view and to offscreen targets that back the
//! HTTP capture endpoints.
//!
//! The renderer borrows eframe's wgpu device rather than creating its own, so there is no
//! cross-device or cross-API texture sharing to get wrong.

use crate::{
    camera::{Camera, Eye},
    scene::Scene,
    skinned::{SkinnedModel, SkinnedVertex},
};
use alvr_common::{glam::Mat4, warn};
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU32;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    CompareFunction, DepthStencilState, Device, Extent3d, FilterMode, FragmentState, FrontFace,
    IndexFormat, LoadOp, MipmapFilterMode, MultisampleState, Operations, Origin3d,
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
    pub fn new(
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        output_format: TextureFormat,
    ) -> Self {
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

    /// Uploads a view-projection matrix built elsewhere, for drawing the scene from a pose other
    /// than the live camera's.
    pub fn set_view_projection(&self, queue: &Queue, eye: Eye, view_proj: Mat4) {
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

/// Most skin joints a hand model may have, matching the array in `hand.wgsl`.
const MAX_SKIN_JOINTS: usize = 64;

/// What a hand model is tinted with: one colour, or two when the model is a fingerless glove.
#[derive(Clone, Copy)]
pub struct HandTint {
    /// Bare skin, past the cuff.
    pub skin: [f32; 3],
    /// The glove itself. `None` tints the whole model as skin.
    pub glove: Option<[f32; 3]>,
}

impl HandTint {
    /// The colour at a vertex that is `bare` bare skin and the rest glove.
    fn at(&self, bare: f32) -> [f32; 3] {
        let Some(glove) = self.glove else {
            return self.skin;
        };

        // Smoothstepped across the blended ring of vertices at the cuff, so the seam follows the
        // modelled ridge instead of stepping between whole vertices.
        let bare = bare.clamp(0.0, 1.0);
        let alpha = bare * bare * (3.0 - 2.0 * bare);

        std::array::from_fn(|channel| {
            glove[channel] + (self.skin[channel] - glove[channel]) * alpha
        })
    }
}

/// The skinned-mesh pipeline and the resources every hand's bind group references.
///
/// Separate from [`MeshPipeline`] because the vertex format carries joint influences and the
/// fragment stage shades from the normal: a hand model is a bare mesh with no baked lighting, so
/// the scene's unlit path would draw it as a flat silhouette.
struct SkinPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    placeholder_view: TextureView,
}

impl SkinPipeline {
    fn new(device: &Device, queue: &Queue, output_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("hand.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("hand bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniforms>() as u64
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        // The pose is the same for both eyes, so unlike the view matrix this one
                        // is written once per frame.
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(skin_buffer_size()),
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
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("hand pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("hand pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<SkinnedVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 32,
                            shader_location: 3,
                        },
                        VertexAttribute {
                            format: VertexFormat::Uint16x4,
                            offset: 44,
                            shader_location: 4,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 52,
                            shader_location: 5,
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
                // Left off for the same reason as the scene: a model that turns out to be wound
                // the other way should look wrong-shaded, not full of holes.
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
            label: Some("hand base colour sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            placeholder_view: create_placeholder_texture(device, queue),
        }
    }
}

/// Size of the joint matrix uniform block, which the shader declares as a fixed-length array.
fn skin_buffer_size() -> u64 {
    (MAX_SKIN_JOINTS * 16 * std::mem::size_of::<f32>()) as u64
}

/// The joint matrix block as bytes, padded to the array the shader declares.
///
/// Written whole rather than only as far as the pose reaches, so the joints a model does not have
/// stay identities instead of holding a previous model's matrices. `glam`'s `Mat4` is not `Pod`
/// here — bytemuck support is behind a feature this workspace does not enable — so the columns are
/// laid out explicitly, which is the same order the GPU expects anyway.
fn packed_matrices(matrices: &[Mat4]) -> Vec<u8> {
    let mut floats = [0.0f32; MAX_SKIN_JOINTS * 16];

    for (slot, target) in floats.chunks_exact_mut(16).enumerate() {
        let matrix = matrices.get(slot).copied().unwrap_or(Mat4::IDENTITY);

        target.copy_from_slice(&matrix.to_cols_array());
    }

    bytemuck::cast_slice(&floats).to_vec()
}

/// GPU form of one hand model.
struct SkinnedMesh {
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    skin_buffer: Buffer,
    primitives: Vec<GpuPrimitive>,
    /// Joints the model actually has, so a pose is not uploaded past the end of the array.
    joint_count: usize,
}

impl SkinnedMesh {
    fn new(
        device: &Device,
        queue: &Queue,
        pipeline: &SkinPipeline,
        model: &SkinnedModel,
        tint: HandTint,
    ) -> Self {
        // The configured colours are folded into the vertices rather than carried as uniforms,
        // matching how the static loader bakes in the material factor. A hand model typically has
        // no material at all, which would otherwise draw it plain white — and the one this ships
        // against is a fingerless glove, which drawn in a single colour reads as an unexplained
        // ridge across every finger rather than as a cuff.
        let vertices = model
            .vertices
            .iter()
            .zip(&model.bare_skin)
            .map(|(vertex, bare)| {
                let base = tint.at(*bare);

                SkinnedVertex {
                    color: [
                        vertex.color[0] * base[0],
                        vertex.color[1] * base[1],
                        vertex.color[2] * base[2],
                    ],
                    ..*vertex
                }
            })
            .collect::<Vec<_>>();

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("hand vertices"),
            size: (vertices.len() * std::mem::size_of::<SkinnedVertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("hand uniforms"),
            size: UNIFORM_STRIDE * 2,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let skin_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("hand joint matrices"),
            size: skin_buffer_size(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Until the first pose arrives the joints are identities, which draws the bind pose rather
        // than collapsing every vertex onto the origin.
        queue.write_buffer(&skin_buffer, 0, &packed_matrices(&[]));

        let mut primitives = Vec::new();
        for primitive in &model.primitives {
            let index_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("hand indices"),
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
                label: Some("hand bind group"),
                layout: &pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &uniform_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(std::mem::size_of::<Uniforms>() as u64),
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: skin_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(
                            texture_view.as_ref().unwrap_or(&pipeline.placeholder_view),
                        ),
                    },
                    BindGroupEntry {
                        binding: 3,
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
            skin_buffer,
            primitives,
            joint_count: model.joints.len().min(MAX_SKIN_JOINTS),
        }
    }

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

/// Renders the emulated hands' skinned models into the scene view.
///
/// Separate from [`ControllerRenderer`] because the geometry is skinned, but used the same way:
/// one model per hand, replaced when the settings name a different file.
pub struct HandRenderer {
    pipeline: SkinPipeline,
    models: [Option<SkinnedMesh>; 2],
}

impl HandRenderer {
    pub fn new(device: &Device, queue: &Queue, output_format: TextureFormat) -> Self {
        Self {
            pipeline: SkinPipeline::new(device, queue, output_format),
            models: [None, None],
        }
    }

    /// Uploads a hand model for one hand, replacing the previous one.
    pub fn set_model(
        &mut self,
        device: &Device,
        queue: &Queue,
        hand: usize,
        model: &SkinnedModel,
        tint: HandTint,
    ) {
        if model.joints.len() > MAX_SKIN_JOINTS {
            warn!(
                "The hand model has {} joints; only the first {MAX_SKIN_JOINTS} will be posed",
                model.joints.len()
            );
        }

        self.models[hand] = Some(SkinnedMesh::new(device, queue, &self.pipeline, model, tint));
    }

    /// Uploads the pose of one hand: the skinning matrices, already in world space.
    pub fn set_pose(&self, queue: &Queue, hand: usize, matrices: &[Mat4]) {
        let Some(mesh) = &self.models[hand] else {
            return;
        };

        let count = matrices.len().min(mesh.joint_count);
        if count == 0 {
            return;
        }

        queue.write_buffer(&mesh.skin_buffer, 0, &packed_matrices(&matrices[..count]));
    }

    /// Uploads the view-projection matrix for one hand and eye. The model transform is already in
    /// the joint matrices, so this is the view and projection alone.
    pub fn set_view(&self, queue: &Queue, hand: usize, eye: Eye, view_proj: Mat4) {
        if let Some(mesh) = &self.models[hand] {
            queue.write_buffer(
                &mesh.uniform_buffer,
                eye_offset(eye),
                bytemuck::bytes_of(&Uniforms {
                    view_proj: view_proj.to_cols_array_2d(),
                }),
            );
        }
    }

    /// Records draw commands for one hand and eye into an existing render pass.
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
        format: TextureFormat::Rgba8Unorm,
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
        // glTF base colour textures are sRGB encoded, and are uploaded as plain Unorm on purpose:
        // the render target is not an sRGB one (egui draws in display values throughout), so
        // decoding them to linear on sampling, with nothing encoding the result back, rendered
        // every texture roughly as if gamma were applied twice, much too dark. Undecoded, a
        // texture comes out as it was authored, and in step with the material colours, which are
        // passed through as they are.
        format: TextureFormat::Rgba8Unorm,
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

/// Devices that can carry a debug gizmo at once: a pose per hand and per controller.
///
/// Slots rather than a list so the renderer's buffers are allocated once. Hands take 0 and 1,
/// controllers 2 and 3; [`GizmoRenderer::slot`] is the only place that mapping is written down.
pub const GIZMO_SLOTS: usize = 4;

/// What a gizmo is drawn for. Hands and controllers each choose their own pose, independently.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GizmoDevice {
    Hand,
    Controller,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GizmoVertex {
    position: [f32; 3],
    color: [f32; 3],
}

/// How long an axis is drawn, in metres. Sized against a hand: long enough to read the direction
/// off, short enough that three of them at a fingertip do not swamp the hand itself.
const AXIS_LENGTH: f32 = 0.05;
const SHAFT_RADIUS: f32 = 0.0022;
const HEAD_RADIUS: f32 = 0.006;
/// Fraction of the axis taken by the arrowhead.
const HEAD_FRACTION: f32 = 0.28;

/// The unit gizmo: three arrows along +X, +Y and +Z, coloured red, green and blue.
///
/// Arrowheads rather than plain bars because an axis has a sign, and a bar drawn from the origin
/// only tells you the sign if you can already see where the origin is — which, inside a fist, you
/// cannot.
fn axes_mesh() -> (Vec<GizmoVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (axis, color) in [
        (0, [1.0, 0.15, 0.15]),
        (1, [0.15, 0.9, 0.2]),
        (2, [0.25, 0.4, 1.0]),
    ] {
        // Local frame for this axis: the offset closure places a vertex `t` along it and `u`, `v`
        // across its square section. Which of the other two axes is which does not matter.
        let offset = |t: f32, u: f32, v: f32| {
            let mut position = [0.0f32; 3];
            position[axis] = t;
            position[(axis + 1) % 3] = u;
            position[(axis + 2) % 3] = v;

            GizmoVertex { position, color }
        };

        let shaft_end = AXIS_LENGTH * (1.0 - HEAD_FRACTION);
        let base = vertices.len() as u32;

        // Square shaft: the four corners at the origin, then the four at the head.
        for t in [0.0, shaft_end] {
            for (u, v) in [
                (-SHAFT_RADIUS, -SHAFT_RADIUS),
                (SHAFT_RADIUS, -SHAFT_RADIUS),
                (SHAFT_RADIUS, SHAFT_RADIUS),
                (-SHAFT_RADIUS, SHAFT_RADIUS),
            ] {
                vertices.push(offset(t, u, v));
            }
        }

        for corner in 0..4u32 {
            let next = (corner + 1) % 4;
            indices.extend([
                base + corner,
                base + next,
                base + 4 + next,
                base + corner,
                base + 4 + next,
                base + 4 + corner,
            ]);
        }

        // Pyramid head: a square base at the end of the shaft rising to a point on the axis.
        let head = vertices.len() as u32;
        for (u, v) in [
            (-HEAD_RADIUS, -HEAD_RADIUS),
            (HEAD_RADIUS, -HEAD_RADIUS),
            (HEAD_RADIUS, HEAD_RADIUS),
            (-HEAD_RADIUS, HEAD_RADIUS),
        ] {
            vertices.push(offset(shaft_end, u, v));
        }
        vertices.push(offset(AXIS_LENGTH, 0.0, 0.0));

        for corner in 0..4u32 {
            let next = (corner + 1) % 4;
            indices.extend([head + corner, head + next, head + 4]);

            // Base of the pyramid, so the head is solid seen from behind.
            if corner < 2 {
                indices.extend([head, head + corner + 1, head + corner + 2]);
            }
        }
    }

    (vertices, indices)
}

/// Draws coordinate axes at a pose, for reading off where an interaction pose actually is.
///
/// Deliberately drawn over everything, depth test and all. Grip sits *inside* the fist and poke on
/// the surface of a fingertip, so a depth-tested gizmo would be hidden exactly when it matters.
pub struct GizmoRenderer {
    pipeline: RenderPipeline,
    vertices: Buffer,
    indices: Buffer,
    index_count: u32,
    uniforms: Buffer,
    bind_group: BindGroup,
}

impl GizmoRenderer {
    pub fn new(device: &Device, queue: &Queue, output_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("gizmo.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("gizmo bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("gizmo pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("gizmo pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<GizmoVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
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
                // The arrows are generated without consistent winding and are solid, so culling
                // would drop half of each one.
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let (vertices, indices) = axes_mesh();

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gizmo vertices"),
            size: (vertices.len() * std::mem::size_of::<GizmoVertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gizmo indices"),
            size: (indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));

        // One slot per device per eye, so a stereo pass draws both without rewriting between them.
        let uniforms = device.create_buffer(&BufferDescriptor {
            label: Some("gizmo uniforms"),
            size: UNIFORM_STRIDE * (GIZMO_SLOTS * 2) as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("gizmo bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniforms,
                    offset: 0,
                    size: Some(
                        std::num::NonZeroU64::new(std::mem::size_of::<Uniforms>() as u64).unwrap(),
                    ),
                }),
            }],
        });

        Self {
            pipeline,
            vertices: vertex_buffer,
            indices: index_buffer,
            index_count: indices.len() as u32,
            uniforms,
            bind_group,
        }
    }

    /// Which slot a device's gizmo lives in. The one place the slot layout is defined.
    pub fn slot(device: GizmoDevice, hand: usize) -> usize {
        match device {
            GizmoDevice::Hand => hand,
            GizmoDevice::Controller => 2 + hand,
        }
    }

    fn offset(slot: usize, eye: Eye) -> u64 {
        UNIFORM_STRIDE * (slot * 2 + eye as usize) as u64
    }

    /// Uploads the combined matrix (projection * view * pose) for one slot and eye.
    pub fn set_view(&self, queue: &Queue, slot: usize, eye: Eye, matrix: Mat4) {
        queue.write_buffer(
            &self.uniforms,
            Self::offset(slot, eye),
            bytemuck::bytes_of(&Uniforms {
                view_proj: matrix.to_cols_array_2d(),
            }),
        );
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, slot: usize, eye: Eye) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[Self::offset(slot, eye) as u32]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

/// The emulated devices drawn on top of whatever is behind them.
///
/// Grouped rather than passed one by one because the same set goes to the window's paint callback
/// and to the offscreen captures, and the two must draw the same thing.
#[derive(Default)]
pub struct DeviceModels<'a> {
    /// Per hand, a controller's world-space model matrix when its model should be drawn.
    pub controllers: Option<(&'a ControllerRenderer, [Option<Mat4>; 2])>,
    /// Per hand, the world-space skinning matrices when a hand model should be drawn.
    pub hands: Option<(&'a HandRenderer, [Option<&'a [Mat4]>; 2])>,
    /// Per slot, the world-space transform of a debug pose gizmo; see [`GizmoRenderer::slot`].
    ///
    /// Independent of the model entries above: a gizmo is drawn for an enabled device whether or
    /// not its model is being displayed, since hiding the hand to see the pose underneath is
    /// exactly what the display toggle is for.
    pub gizmos: Option<(&'a GizmoRenderer, [Option<Mat4>; GIZMO_SLOTS])>,
}

impl DeviceModels<'_> {
    /// Uploads the per-eye matrices, and the hand poses, which do not depend on the eye.
    pub fn upload(&self, queue: &Queue, eye_views: [Mat4; 2], projection: Mat4) {
        if let Some((renderer, poses)) = &self.hands {
            for (hand, pose) in poses.iter().enumerate() {
                if let Some(pose) = pose {
                    renderer.set_pose(queue, hand, pose);
                }
            }
        }

        for (index, eye) in [Eye::Left, Eye::Right].into_iter().enumerate() {
            let view_proj = projection * eye_views[index];

            if let Some((renderer, models)) = &self.controllers {
                for (hand, model) in models.iter().enumerate() {
                    if let Some(model) = model {
                        renderer.set_view(queue, hand, eye, view_proj * *model);
                    }
                }
            }

            if let Some((renderer, poses)) = &self.hands {
                for (hand, pose) in poses.iter().enumerate() {
                    if pose.is_some() {
                        // The hand's own transform is already in its skinning matrices, so this is
                        // the view and projection alone.
                        renderer.set_view(queue, hand, eye, view_proj);
                    }
                }
            }

            if let Some((renderer, transforms)) = &self.gizmos {
                for (slot, transform) in transforms.iter().enumerate() {
                    if let Some(transform) = transform {
                        renderer.set_view(queue, slot, eye, view_proj * *transform);
                    }
                }
            }
        }
    }

    /// Records the draw commands for one eye. The caller has already drawn what is behind them.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, eye: Eye) {
        if let Some((renderer, models)) = &self.controllers {
            for (hand, model) in models.iter().enumerate() {
                if model.is_some() {
                    renderer.draw(pass, hand, eye);
                }
            }
        }

        if let Some((renderer, poses)) = &self.hands {
            for (hand, pose) in poses.iter().enumerate() {
                if pose.is_some() {
                    renderer.draw(pass, hand, eye);
                }
            }
        }

        // Last, and over the top of the models: a gizmo inside a fist has to stay readable.
        if let Some((renderer, transforms)) = &self.gizmos {
            for (slot, transform) in transforms.iter().enumerate() {
                if transform.is_some() {
                    renderer.draw(pass, slot, eye);
                }
            }
        }
    }
}

/// Renders both eyes side by side into an offscreen target and reads the result back.
///
/// Visible controller and hand models are drawn into the capture as well, so the endpoints show
/// the same scene as the window.
///
/// Returns tightly packed pixels: RGBA8 for colour, or 8-bit greyscale for depth.
#[expect(clippy::too_many_arguments)]
pub fn capture_stereo(
    device: &Device,
    queue: &Queue,
    renderer: &SceneRenderer,
    devices: &DeviceModels<'_>,
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

    devices.upload(
        queue,
        [
            camera.view_matrix(Eye::Left),
            camera.view_matrix(Eye::Right),
        ],
        Camera::projection_matrix(aspect_ratio),
    );

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
            devices.draw(&mut pass, eye);
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
