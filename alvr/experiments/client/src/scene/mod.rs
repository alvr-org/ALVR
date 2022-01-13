use crate::{
    xr::{SceneButtons, XrHandPoseInput},
    ViewConfig,
};
use alvr_common::{
    glam::{Mat4, UVec2, Vec4},
    log,
    prelude::*,
};
use alvr_graphics::{
    wgpu::{
        Backend, Color, CommandEncoderDescriptor, DeviceType, LoadOp, Operations,
        RenderPassColorAttachment, RenderPassDescriptor, TextureFormat, TextureView,
    },
    GraphicsContext,
};
use rend3::{
    types::{Camera, CameraProjection, MipmapCount, MipmapSource, SampleCount, Texture},
    util::output::OutputFrame,
    ExtendedAdapterInfo, InstanceAdapterDevice, RenderGraph, RendererMode, Vendor,
};
use rend3_routine::{PbrRenderRoutine, RenderTextureOptions, SkyboxRoutine, TonemappingRoutine};
use std::sync::Arc;

const NEAR_PLANE_M: f32 = 0.01;

// Responsible for rendering the lobby room or HUD
pub struct Scene {
    graphics_context: Arc<GraphicsContext>,
    renderer: Arc<rend3::Renderer>,
    pbr_routine: PbrRenderRoutine,
    skybox_routine: SkyboxRoutine,
    tonemapping_routine: TonemappingRoutine,
    should_render_lobby: bool,
}

impl Scene {
    pub fn new(graphics_context: Arc<GraphicsContext>) -> StrResult<Self> {
        log::error!("create scene");

        let iad = InstanceAdapterDevice {
            instance: Arc::clone(&graphics_context.instance),
            adapter: Arc::clone(&graphics_context.adapter),
            device: Arc::clone(&graphics_context.device),
            queue: Arc::clone(&graphics_context.queue),
            mode: RendererMode::CPUPowered,
            info: ExtendedAdapterInfo {
                name: "".into(),
                vendor: Vendor::Unknown(0),
                device: 0,
                device_type: DeviceType::Other,
                backend: Backend::Vulkan,
            },
        };

        // let iad = pollster::block_on(rend3::create_iad(
        //     None,
        //     None,
        //     Some(RendererMode::CPUPowered),
        //     None,
        // ))
        // .unwrap();

        let renderer = trace_err!(rend3::Renderer::new(iad, None))?;

        let pbr_routine = PbrRenderRoutine::new(
            &renderer,
            RenderTextureOptions {
                resolution: UVec2::new(1, 1),
                samples: SampleCount::One,
            },
        );

        let mut skybox_routine = SkyboxRoutine::new(
            &renderer,
            RenderTextureOptions {
                resolution: UVec2::new(1, 1),
                samples: SampleCount::One,
            },
        );
        let skybox_handle = renderer.add_texture_cube(Texture {
            label: Some("skybox".into()),
            data: vec![
                255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
                255, 0, 0, 255,
            ],
            format: TextureFormat::Rgba8UnormSrgb,
            size: UVec2::new(1, 1),
            mip_count: MipmapCount::ONE,
            mip_source: MipmapSource::Uploaded,
        });
        skybox_routine.set_background_texture(Some(skybox_handle));

        let tonemapping_routine =
            TonemappingRoutine::new(&renderer, UVec2::new(1, 1), TextureFormat::Rgba8UnormSrgb);

        Ok(Self {
            graphics_context,
            renderer,
            skybox_routine,
            pbr_routine,
            tonemapping_routine,
            should_render_lobby: false,
        })
    }

    pub fn update(
        &mut self,
        left_pose_input: XrHandPoseInput,
        right_pose_input: XrHandPoseInput,
        buttons: SceneButtons,
        stream_updated: bool,
        is_focused: bool,
    ) {
        self.should_render_lobby = !stream_updated;
    }

    pub fn render(
        &mut self,
        camera_view_config: &ViewConfig,
        output: Arc<TextureView>,
        output_resolution: UVec2,
    ) {
        let mut encoder = self
            .graphics_context
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let clear_color = if self.should_render_lobby {
                Color::RED
            } else {
                Color::GREEN // let the stream pass through
            };

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[RenderPassColorAttachment {
                    view: &output,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color),
                        store: true,
                    },
                }],
                ..Default::default()
            });
        }

        self.graphics_context.queue.submit(Some(encoder.finish()));

        // // Update view size
        // self.renderer
        //     .set_aspect_ratio(output_resolution.x as f32 / output_resolution.y as f32);
        // self.pbr_routine.resize(
        //     &self.renderer,
        //     RenderTextureOptions {
        //         resolution: output_resolution,
        //         samples: SampleCount::One,
        //     },
        // );
        // self.tonemapping_routine.resize(output_resolution);

        // // Update camera
        // let l = camera_view_config.fov.left.tan();
        // let r = camera_view_config.fov.right.tan();
        // let t = camera_view_config.fov.top.tan();
        // let b = camera_view_config.fov.bottom.tan();
        // let projection = Mat4::from_cols(
        //     // NB: the matrix here is defined as column major, it appears transposed but it's not
        //     Vec4::new(2_f32 / (r - l), 0_f32, 0_f32, 0_f32),
        //     Vec4::new(0_f32, 0_f32 / (t - b), 0_f32, 0_f32),
        //     Vec4::new((r + l) / (r - l), (t + b) / (t - b), -1_f32, -1_f32),
        //     Vec4::new(0_f32, 0_f32, -2_f32 * NEAR_PLANE_M, 0_f32),
        // );
        // self.renderer.set_camera_data(Camera {
        //     projection: CameraProjection::Raw(projection),
        //     view: Mat4::from_rotation_translation(
        //         camera_view_config.orientation,
        //         camera_view_config.position,
        //     ),
        // });

        // // Render
        // let (command_buffers, ready_data) = self.renderer.ready();
        // let mut graph = RenderGraph::new();
        // rend3_routine::add_default_rendergraph(
        //     &mut graph,
        //     &ready_data,
        //     &self.pbr_routine,
        //     Some(&self.skybox_routine),
        //     &self.tonemapping_routine,
        //     rend3::types::SampleCount::One,
        // );
        // graph.execute(
        //     &self.renderer,
        //     OutputFrame::View(output),
        //     command_buffers,
        //     &ready_data,
        // );
    }
}
