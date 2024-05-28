use crate::GraphicsContext;
use alvr_common::{
    anyhow::Result,
    glam::{Mat4, UVec2, Vec3, Vec4},
    Fov, Pose,
};
use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId, GlyphPositioner, HorizontalAlign, Layout, SectionGeometry, SectionText, VerticalAlign,
};
use rend3::{
    graph::{RenderGraph, ViewportRect},
    types::{
        Camera, CameraProjection, Handedness, MeshBuilder, MipmapCount, MipmapSource, Object,
        ObjectMeshKind, ResourceHandle, SampleCount, Texture,
    },
    ExtendedAdapterInfo, InstanceAdapterDevice, Renderer, RendererProfile, ShaderPreProcessor,
    Vendor,
};
use rend3_routine::{
    base::{
        BaseRenderGraph, BaseRenderGraphInputs, BaseRenderGraphRoutines, BaseRenderGraphSettings,
        OutputRenderTarget,
    },
    pbr::{AlbedoComponent, PbrMaterial, PbrRoutine},
    skybox::SkyboxRoutine,
    tonemapping::TonemappingRoutine,
};
use std::sync::Arc;
use wgpu::TextureFormat;

const HUD_TEXTURE_SIDE: usize = 2048;
const HUD_FONT_SIZE: f32 = 40.0;

// Left handed matrix, with clip space [0; 1], far plane to infinity
fn projection_from_fov(fov: Fov) -> Mat4 {
    const NEAR: f32 = 0.1;

    let tanl = fov.left.tan();
    let tanr = fov.right.tan();
    let tanu = fov.up.tan();
    let tand = fov.down.tan();

    let x = 2.0 / (tanr - tanl);
    let y = 2.0 / (tanu - tand);
    let cx = (tanr + tanl) / (tanr - tanl);
    let cy = (tanu + tand) / (tanu - tand);
    Mat4::from_cols(
        Vec4::new(x, 0.0, 0.0, 0.0),
        Vec4::new(0.0, y, 0.0, 0.0),
        Vec4::new(cx, cy, 0.0, -1.0),
        Vec4::new(0.0, 0.0, NEAR, 0.0),
    )
}

pub struct LobbyRenderer {
    renderer: Arc<Renderer>,
    base_rendergraph: BaseRenderGraph,
    pbr_routine: PbrRoutine,
    skybox_routine: SkyboxRoutine,
    tonemapping_routine: TonemappingRoutine,
    swapchains: [Vec<wgpu::Texture>; 2],
    swapchain_resolution: UVec2,
    _floor_handle: ResourceHandle<Object>,
}

impl LobbyRenderer {
    pub fn new<T: Clone>(
        graphics_context: &GraphicsContext<T>,
        swapchains: [Vec<wgpu::Texture>; 2],
        swapchain_resolution: UVec2,
    ) -> Result<Self> {
        let iad = InstanceAdapterDevice {
            instance: Arc::clone(&graphics_context.instance),
            adapter: Arc::clone(&graphics_context.adapter),
            device: Arc::clone(&graphics_context.device),
            queue: Arc::clone(&graphics_context.queue),
            profile: RendererProfile::CpuDriven,
            info: ExtendedAdapterInfo {
                name: graphics_context.adapter.get_info().name,
                vendor: match graphics_context.adapter.get_info().vendor {
                    0x1002 => Vendor::Amd,
                    0x10DE => Vendor::Nv,
                    0x13B5 => Vendor::Arm,
                    0x1414 => Vendor::Microsoft,
                    0x14E4 => Vendor::Broadcom,
                    0x5143 => Vendor::Qualcomm,
                    0x8086 => Vendor::Intel,
                    v => Vendor::Unknown(v as usize),
                },
                device: graphics_context.adapter.get_info().device as usize,
                device_type: graphics_context.adapter.get_info().device_type,
                backend: graphics_context.adapter.get_info().backend,
            },
        };

        let renderer = Renderer::new(iad, Handedness::Left, None)?;

        let mut shader_preprocessor = ShaderPreProcessor::new();

        rend3_routine::builtin_shaders(&mut shader_preprocessor);

        let base_rendergraph = BaseRenderGraph::new(&renderer, &shader_preprocessor);

        let mut data_core = renderer.data_core.lock();
        let pbr_routine = PbrRoutine::new(
            &renderer,
            &mut data_core,
            &shader_preprocessor,
            &base_rendergraph.interfaces,
            &base_rendergraph.gpu_culler.culling_buffer_map_handle,
        );
        drop(data_core);

        let skybox_routine = SkyboxRoutine::new(
            &renderer,
            &shader_preprocessor,
            &base_rendergraph.interfaces,
        );

        let tonemapping_routine = TonemappingRoutine::new(
            &renderer,
            &shader_preprocessor,
            &base_rendergraph.interfaces,
            TextureFormat::Rgba8UnormSrgb,
        );

        let floor_vertices = [
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
        ];
        let floor_indices = [0, 1, 2, 2, 3, 0];
        let floor_plane = MeshBuilder::new(floor_vertices.to_vec(), Handedness::Left)
            .with_indices(floor_indices.to_vec())
            .build()?;
        let floor_mesh = renderer.add_mesh(floor_plane).unwrap();
        let floor_material = renderer.add_material(PbrMaterial {
            albedo: AlbedoComponent::Value(Vec4::new(0.0, 1.0, 0.0, 1.0)),
            unlit: true,
            ..PbrMaterial::default()
        });
        let floor_object = Object {
            mesh_kind: ObjectMeshKind::Static(floor_mesh),
            material: floor_material,
            transform: Mat4::IDENTITY,
        };
        let _floor_handle = renderer.add_object(floor_object);

        Ok(Self {
            renderer,
            base_rendergraph,
            pbr_routine,
            skybox_routine,
            tonemapping_routine,
            swapchains,
            swapchain_resolution,
            _floor_handle,
        })
    }

    pub fn update_hud_message(&mut self, message: &str) -> Result<()> {
        let ubuntu_font =
            FontRef::try_from_slice(include_bytes!("../resources/Ubuntu-Medium.ttf")).unwrap();

        let section_glyphs = Layout::default()
            .h_align(HorizontalAlign::Center)
            .v_align(VerticalAlign::Center)
            .calculate_glyphs(
                &[&ubuntu_font],
                &SectionGeometry {
                    screen_position: (
                        HUD_TEXTURE_SIDE as f32 / 2_f32,
                        HUD_TEXTURE_SIDE as f32 / 2_f32,
                    ),
                    ..Default::default()
                },
                &[SectionText {
                    text: message,
                    scale: HUD_FONT_SIZE.into(),
                    font_id: FontId(0),
                }],
            );

        let scaled_font = ubuntu_font.as_scaled(HUD_FONT_SIZE);

        let mut texture_side_buffer = vec![0; HUD_TEXTURE_SIDE * HUD_TEXTURE_SIDE * 4];
        for section_glyph in section_glyphs {
            if let Some(outlined) = scaled_font.outline_glyph(section_glyph.glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|x, y, value| {
                    let x = x as usize + bounds.min.x as usize;
                    let y = y as usize + bounds.min.y as usize;
                    let value = (value * 255.0) as u8;
                    // Note: x axis is flipped to workaround rend3 bug
                    let coord_idx = (y * HUD_TEXTURE_SIDE + HUD_TEXTURE_SIDE - x - 1) * 4;
                    texture_side_buffer[coord_idx] = value;
                    texture_side_buffer[coord_idx + 1] = value;
                    texture_side_buffer[coord_idx + 2] = value;
                    texture_side_buffer[coord_idx + 3] = 255;
                });
            }
        }

        let mut cube_texture_buffer = vec![];
        cube_texture_buffer.append(&mut texture_side_buffer.clone());
        cube_texture_buffer.append(&mut texture_side_buffer.clone());
        cube_texture_buffer.append(&mut vec![0; HUD_TEXTURE_SIDE * HUD_TEXTURE_SIDE * 4]);
        cube_texture_buffer.append(&mut vec![0; HUD_TEXTURE_SIDE * HUD_TEXTURE_SIDE * 4]);
        cube_texture_buffer.append(&mut texture_side_buffer.clone());
        cube_texture_buffer.append(&mut texture_side_buffer);

        let skybox_texture = self.renderer.add_texture_cube(Texture {
            format: TextureFormat::Rgba8Unorm,
            size: UVec2::new(HUD_TEXTURE_SIDE as u32, HUD_TEXTURE_SIDE as u32),
            data: cube_texture_buffer,
            label: Some("skybox".into()),
            mip_count: MipmapCount::ONE,
            mip_source: MipmapSource::Uploaded,
        })?;
        self.skybox_routine
            .set_background_texture(Some(skybox_texture));

        Ok(())
    }

    fn render_eye(&mut self, pose: Pose, fov: Fov, swapchain_index: usize, view_index: usize) {
        self.renderer.set_camera_data(Camera {
            projection: CameraProjection::Raw(projection_from_fov(fov)),
            view: Mat4::from_rotation_translation(pose.orientation, pose.position).inverse(),
        });

        // Apply scene changes
        self.renderer.swap_instruction_buffers();
        let mut eval_output = self.renderer.evaluate_instructions();

        self.skybox_routine.evaluate(&self.renderer);

        // Build render graph
        let mut graph = RenderGraph::new();
        let frame_handle = graph.add_imported_render_target(
            &self.swapchains[view_index][swapchain_index],
            0..1,
            0..1,
            ViewportRect::from_size(self.swapchain_resolution),
        );

        self.base_rendergraph.add_to_graph(
            &mut graph,
            BaseRenderGraphInputs {
                eval_output: &eval_output,
                routines: BaseRenderGraphRoutines {
                    pbr: &self.pbr_routine,
                    skybox: Some(&self.skybox_routine),
                    tonemapping: &self.tonemapping_routine,
                },
                target: OutputRenderTarget {
                    handle: frame_handle,
                    resolution: self.swapchain_resolution,
                    samples: SampleCount::One,
                },
            },
            BaseRenderGraphSettings {
                ambient_color: Vec4::ZERO,
                clear_color: Vec4::new(1.0, 0.0, 0.0, 1.0),
            },
        );

        graph.execute(&self.renderer, &mut eval_output);
    }

    pub fn render(&mut self, poses: [Pose; 2], fov: [Fov; 2], swapchain_indexes: [usize; 2]) {
        self.render_eye(poses[0], fov[0], swapchain_indexes[0], 0);
        self.render_eye(poses[1], fov[1], swapchain_indexes[1], 1);
    }
}
