use alvr_common::{prelude::*, Fov};
use alvr_graphics::GraphicsContext;
use glam::{Mat4, Quat, UVec2, Vec3, Vec4};
use rend3::{
    types::{Camera, CameraProjection},
    util::output::OutputFrame,
    ExtendedAdapterInfo, InstanceAdapterDevice, RenderGraph, RendererMode, Vendor,
};
use rend3_routine::{PbrRenderRoutine, RenderTextureOptions, SampleCount};
use std::sync::Arc;
use wgpu::{Backend, DeviceType, TextureView};

const NEAR_PLANE_M: f32 = 0.01;

// Responsible for rendering the lobby room or HUD
pub struct SceneRenderer {
    renderer: Arc<rend3::Renderer>,
    pbr_routine: PbrRenderRoutine,
}

impl SceneRenderer {
    pub fn new(graphics_context: &GraphicsContext) -> StrResult<Self> {
        let iad = InstanceAdapterDevice {
            instance: Arc::clone(&graphics_context.instance),
            adapter: Arc::clone(&graphics_context.adapter),
            device: Arc::clone(&graphics_context.device),
            queue: Arc::clone(&graphics_context.queue),
            mode: RendererMode::GPUPowered,
            info: ExtendedAdapterInfo {
                name: "".into(),
                vendor: Vendor::Unknown(0),
                device: 0,
                device_type: DeviceType::Other,
                backend: Backend::Vulkan,
            },
        };

        let renderer = trace_err!(rend3::Renderer::new(iad, None))?;
        let pbr_routine = PbrRenderRoutine::new(
            &renderer,
            RenderTextureOptions {
                resolution: UVec2::new(1, 1),
                samples: SampleCount::One,
            },
        );

        Ok(Self {
            renderer,
            pbr_routine,
        })
    }

    pub fn render(
        &mut self,
        camera_rotation: Quat,
        camera_translation: Vec3,
        fov: Fov,
        output: Arc<TextureView>,
        output_resolution: UVec2,
    ) {
        self.renderer
            .set_aspect_ratio(output_resolution.x as f32 / output_resolution.y as f32);
        self.pbr_routine.resize(
            &self.renderer,
            RenderTextureOptions {
                resolution: output_resolution,
                samples: SampleCount::One,
            },
        );

        let l = fov.left.tan();
        let r = fov.right.tan();
        let t = fov.top.tan();
        let b = fov.bottom.tan();
        let projection = Mat4::from_cols(
            // NB: the matrix here is defined as column major, it appears transposed but it's not
            Vec4::new(2_f32 / (r - l), 0_f32, 0_f32, 0_f32),
            Vec4::new(0_f32, 0_f32 / (t - b), 0_f32, 0_f32),
            Vec4::new((r + l) / (r - l), (t + b) / (t - b), -1_f32, -1_f32),
            Vec4::new(0_f32, 0_f32, -2_f32 * NEAR_PLANE_M, 0_f32),
        );
        self.renderer.set_camera_data(Camera {
            projection: CameraProjection::Raw(projection),
            view: Mat4::from_rotation_translation(camera_rotation, camera_translation),
        });

        let (command_buffers, ready_data) = self.renderer.ready();

        let mut graph = RenderGraph::new();

        self.pbr_routine.add_pre_cull_to_graph(&mut graph);
        self.pbr_routine
            .add_shadow_culling_to_graph(&mut graph, &ready_data);
        self.pbr_routine.add_culling_to_graph(&mut graph);
        self.pbr_routine
            .add_shadow_rendering_to_graph(&mut graph, &ready_data);
        self.pbr_routine.add_prepass_to_graph(&mut graph);
        self.pbr_routine.add_forward_to_graph(&mut graph);

        // Note: consumes the graph, cannot keep between render() calls
        graph.execute(
            &self.renderer,
            OutputFrame::View(output),
            command_buffers,
            &ready_data,
        );
    }
}
