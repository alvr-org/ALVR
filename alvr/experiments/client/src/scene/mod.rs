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
// use rend3::{
//     types::{Camera, CameraProjection, MipmapCount, MipmapSource, SampleCount, Texture},
//     util::output::OutputFrame,
//     ExtendedAdapterInfo, InstanceAdapterDevice, RenderGraph, RendererMode, Vendor,
// };
// use rend3_routine::{PbrRenderRoutine, RenderTextureOptions, SkyboxRoutine};
use std::sync::Arc;

const NEAR_PLANE_M: f32 = 0.01;

// Responsible for rendering the lobby room or HUD
pub struct Scene {
    graphics_context: Arc<GraphicsContext>,
    // renderer: Arc<rend3::Renderer>,
    // skybox_routine: SkyboxRoutine,
    // pbr_routine: PbrRenderRoutine,
    // graphics_context: Arc<GraphicsContext>,
    should_render_lobby: bool,
}

impl Scene {
    pub fn new(graphics_context: Arc<GraphicsContext>) -> StrResult<Self> {
        // log::error!("create scene");

        // // let iad = InstanceAdapterDevice {
        // //     instance: Arc::clone(&graphics_context.instance),
        // //     adapter: Arc::clone(&graphics_context.adapter),
        // //     device: Arc::clone(&graphics_context.device),
        // //     queue: Arc::clone(&graphics_context.queue),
        // //     mode: RendererMode::CPUPowered,
        // //     info: ExtendedAdapterInfo {
        // //         name: "".into(),
        // //         vendor: Vendor::Unknown(0),
        // //         device: 0,
        // //         device_type: DeviceType::Other,
        // //         backend: Backend::Vulkan,
        // //     },
        // // };

        // let iad = pollster::block_on(rend3::create_iad(
        //     None,
        //     None,
        //     Some(RendererMode::CPUPowered),
        //     None,
        // ))
        // .unwrap();

        // log::error!("create renderer");

        // let renderer = trace_err!(rend3::Renderer::new(iad, None))?;

        // log::error!("create pbr routine");

        // let pbr_routine = PbrRenderRoutine::new(
        //     &renderer,
        //     RenderTextureOptions {
        //         resolution: UVec2::new(100, 100),
        //         samples: SampleCount::One,
        //     },
        // );

        // log::error!("create skybox routine");

        // let mut skybox_routine = SkyboxRoutine::new(
        //     &renderer,
        //     RenderTextureOptions {
        //         resolution: UVec2::new(1, 1),
        //         samples: SampleCount::One,
        //     },
        // );
        // let skybox_handle = renderer.add_texture_cube(Texture {
        //     label: Some("skybox".into()),
        //     data: vec![
        //         255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        //         255, 0, 0, 255,
        //     ],
        //     format: TextureFormat::Rgba8UnormSrgb,
        //     size: UVec2::new(1, 1),
        //     mip_count: MipmapCount::ONE,
        //     mip_source: MipmapSource::Uploaded,
        // });
        // skybox_routine.set_background_texture(Some(skybox_handle));

        // Ok(Self {
        //     renderer,
        //     skybox_routine,
        //     // pbr_routine,
        // })

        Ok(Self {
            graphics_context,
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
                Color::TRANSPARENT // let the stream pass through
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

        // self.renderer
        //     .set_aspect_ratio(output_resolution.x as f32 / output_resolution.y as f32);

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
        // // self.renderer.set_camera_data(Camera {
        // //     projection: CameraProjection::Raw(projection),
        // //     view: Mat4::from_rotation_translation(
        // //         camera_view_config.orientation,
        // //         camera_view_config.position,
        // //     ),
        // // });

        // let (command_buffers, ready_data) = self.renderer.ready();
        // self.skybox_routine.ready(&self.renderer);

        // let mut graph = RenderGraph::new();

        // // self.pbr_routine.add_pre_cull_to_graph(&mut graph);
        // // self.pbr_routine
        // //     .add_shadow_culling_to_graph(&mut graph, &ready_data);
        // // self.pbr_routine.add_culling_to_graph(&mut graph);
        // // self.pbr_routine
        // //     .add_shadow_rendering_to_graph(&mut graph, &ready_data);
        // // self.pbr_routine.add_prepass_to_graph(&mut graph);
        // // self.skybox_routine.add_to_graph(&mut graph);
        // // self.pbr_routine.add_forward_to_graph(&mut graph);

        // // Note: consumes the graph, cannot keep between render() calls
        // graph.execute(
        //     &self.renderer,
        //     OutputFrame::View(output),
        //     command_buffers,
        //     &ready_data,
        // );
    }
}
