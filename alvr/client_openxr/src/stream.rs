use crate::{
    from_xr_pose,
    graphics::{self, CompositionLayerBuilder},
    interaction::{self, InteractionContext},
    to_xr_fov, to_xr_pose, XrContext,
};
use alvr_client_core::{ClientCoreContext, DecodedFrame, Platform};
use alvr_common::{
    anyhow::Result,
    error,
    glam::{UVec2, Vec2, Vec3},
    RelaxedAtomic, HAND_LEFT_ID, HAND_RIGHT_ID,
};
use alvr_graphics::{ClientStreamRenderer, GraphicsContext, VulkanBackend};
use alvr_packets::{FaceData, NegotiatedStreamingConfig, ViewParams};
use alvr_session::{
    BodyTrackingSourcesConfig, ClientsideFoveationConfig, ClientsideFoveationMode, EncoderConfig,
    FaceTrackingSourcesConfig, FoveatedEncodingConfig, Settings,
};
use openxr as xr;
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

// When the latency goes too high, if prediction offset is not capped tracking poll will fail.
const MAX_PREDICTION: Duration = Duration::from_millis(70);

#[derive(PartialEq)]
pub struct StreamConfig {
    pub view_resolution: UVec2,
    pub refresh_rate_hint: f32,
    pub foveated_encoding_config: Option<FoveatedEncodingConfig>,
    pub clientside_foveation_config: Option<ClientsideFoveationConfig>,
    pub encoder_config: EncoderConfig,
    pub face_sources_config: Option<FaceTrackingSourcesConfig>,
    pub body_sources_config: Option<BodyTrackingSourcesConfig>,
}

impl StreamConfig {
    pub fn new(settings: &Settings, negotiated_config: NegotiatedStreamingConfig) -> StreamConfig {
        StreamConfig {
            view_resolution: negotiated_config.view_resolution,
            refresh_rate_hint: negotiated_config.refresh_rate_hint,
            foveated_encoding_config: negotiated_config
                .enable_foveated_encoding
                .then(|| settings.video.foveated_encoding.as_option().cloned())
                .flatten(),
            clientside_foveation_config: settings.video.clientside_foveation.as_option().cloned(),
            encoder_config: settings.video.encoder_config.clone(),
            face_sources_config: settings
                .headset
                .face_tracking
                .as_option()
                .map(|c| c.sources.clone()),
            body_sources_config: settings
                .headset
                .body_tracking
                .as_option()
                .map(|c| c.sources.clone()),
        }
    }
}

pub struct StreamContext {
    core_context: Arc<ClientCoreContext>,
    xr_context: XrContext,
    interaction_context: Arc<InteractionContext>,
    renderer: ClientStreamRenderer<VulkanBackend>,
    reference_space: Arc<xr::Space>,
    swapchain: xr::Swapchain<xr::Vulkan>,
    view_resolution: UVec2,
    refresh_rate: f32,
    last_good_view_params: [ViewParams; 2],
    input_thread: Option<JoinHandle<()>>,
    input_thread_running: Arc<RelaxedAtomic>,
}

impl StreamContext {
    pub fn new(
        core_ctx: Arc<ClientCoreContext>,
        graphics_ctx: GraphicsContext<VulkanBackend>,
        xr_ctx: XrContext,
        interaction_ctx: Arc<InteractionContext>,
        platform: Platform,
        config: &StreamConfig,
    ) -> Result<Self> {
        if xr_ctx.instance.exts().fb_display_refresh_rate.is_some() {
            xr_ctx
                .session
                .request_display_refresh_rate(config.refresh_rate_hint)
                .unwrap();
        }
        // todo: check which permissions are needed for htc
        #[cfg(target_os = "android")]
        if let Some(config) = &config.face_sources_config {
            if (config.combined_eye_gaze || config.eye_tracking_fb)
                && matches!(platform, Platform::Quest3 | Platform::QuestPro)
            {
                alvr_client_core::try_get_permission("com.oculus.permission.EYE_TRACKING")
            }
            if config.combined_eye_gaze && matches!(platform, Platform::Pico4 | Platform::PicoNeo3)
            {
                alvr_client_core::try_get_permission("com.picovr.permission.EYE_TRACKING")
            }
            if config.face_tracking_fb && matches!(platform, Platform::Quest3 | Platform::QuestPro)
            {
                alvr_client_core::try_get_permission("android.permission.RECORD_AUDIO");
                alvr_client_core::try_get_permission("com.oculus.permission.FACE_TRACKING")
            }
        }

        #[cfg(target_os = "android")]
        if let Some(config) = &config.body_sources_config {
            if (config.body_tracking_full_body_meta.enabled())
                && matches!(platform, Platform::Quest3 | Platform::QuestPro)
            {
                alvr_client_core::try_get_permission("com.oculus.permission.BODY_TRACKING")
            }
        }

        let foveation_profile = if let Some(config) = &config.clientside_foveation_config {
            if xr_ctx.instance.exts().fb_swapchain_update_state.is_some()
                && xr_ctx.instance.exts().fb_foveation.is_some()
                && xr_ctx.instance.exts().fb_foveation_configuration.is_some()
            {
                let level;
                let dynamic;
                match config.mode {
                    ClientsideFoveationMode::Static { level: lvl } => {
                        level = lvl;
                        dynamic = false;
                    }
                    ClientsideFoveationMode::Dynamic { max_level } => {
                        level = max_level;
                        dynamic = true;
                    }
                };

                xr_ctx
                    .session
                    .create_foveation_profile(Some(xr::FoveationLevelProfile {
                        level: xr::FoveationLevelFB::from_raw(level as i32),
                        vertical_offset: config.vertical_offset_deg,
                        dynamic: xr::FoveationDynamicFB::from_raw(dynamic as i32),
                    }))
                    .ok()
            } else {
                None
            }
        } else {
            None
        };

        let swapchain = graphics::create_swapchain(&xr_ctx.session, config.view_resolution, 2);

        let wgpu_swapchain = graphics_ctx.create_vulkan_swapchain_external(
            &swapchain.enumerate_images().unwrap(),
            config.view_resolution,
            2,
        );
        let renderer = ClientStreamRenderer::new(
            graphics_ctx,
            3,
            wgpu_swapchain,
            config.view_resolution,
            platform == Platform::Lynx,
        )?;

        core_ctx.send_playspace(
            xr_ctx
                .session
                .reference_space_bounds_rect(xr::ReferenceSpaceType::STAGE)
                .unwrap()
                .map(|a| Vec2::new(a.width, a.height)),
        );

        core_ctx.send_active_interaction_profile(
            *HAND_LEFT_ID,
            interaction_ctx.hands_interaction[0].controllers_profile_id,
        );
        core_ctx.send_active_interaction_profile(
            *HAND_RIGHT_ID,
            interaction_ctx.hands_interaction[1].controllers_profile_id,
        );

        let input_thread_running = Arc::new(RelaxedAtomic::new(true));

        let reference_space = Arc::new(interaction::get_stage_reference_space(&xr_ctx.session));

        let input_thread = thread::spawn({
            let core_ctx = Arc::clone(&core_ctx);
            let xr_ctx = xr_ctx.clone();
            let interaction_ctx = Arc::clone(&interaction_ctx);
            let reference_space = Arc::clone(&reference_space);
            let refresh_rate = config.refresh_rate_hint;
            let running = Arc::clone(&input_thread_running);
            move || {
                stream_input_loop(
                    &core_ctx,
                    xr_ctx,
                    &interaction_ctx,
                    Arc::clone(&reference_space),
                    refresh_rate,
                    running,
                )
            }
        });

        Ok(StreamContext {
            core_context: core_ctx,
            xr_context: xr_ctx,
            interaction_context: interaction_ctx,
            renderer,
            reference_space,
            swapchain,
            view_resolution: config.view_resolution,
            refresh_rate: config.refresh_rate_hint,
            last_good_view_params: [ViewParams::default(); 2],
            input_thread: Some(input_thread),
            input_thread_running,
        })
    }

    pub fn update_reference_space(&mut self) {
        self.input_thread_running.set(false);

        self.reference_space = Arc::new(interaction::get_stage_reference_space(
            &self.xr_context.session,
        ));

        self.core_context.send_playspace(
            self.xr_context
                .session
                .reference_space_bounds_rect(xr::ReferenceSpaceType::STAGE)
                .unwrap()
                .map(|a| Vec2::new(a.width, a.height)),
        );

        if let Some(running) = self.input_thread.take() {
            running.join().ok();
        }

        self.input_thread_running.set(true);

        self.input_thread = Some(thread::spawn({
            let core_ctx = Arc::clone(&self.core_context);
            let xr_ctx = self.xr_context.clone();
            let interaction_ctx = Arc::clone(&self.interaction_context);
            let reference_space = Arc::clone(&self.reference_space);
            let refresh_rate = self.refresh_rate;
            let running = Arc::clone(&self.input_thread_running);
            move || {
                stream_input_loop(
                    &core_ctx,
                    xr_ctx,
                    &interaction_ctx,
                    Arc::clone(&reference_space),
                    refresh_rate,
                    running,
                )
            }
        }));
    }

    pub fn render(
        &mut self,
        decoded_frame: Option<DecodedFrame>,
        vsync_time: Duration,
    ) -> CompositionLayerBuilder {
        let timestamp;
        let view_params;
        let buffer_ptr;
        if let Some(frame) = decoded_frame {
            timestamp = frame.timestamp;
            view_params = frame.view_params;
            buffer_ptr = frame.buffer_ptr;

            self.last_good_view_params = frame.view_params;
        } else {
            timestamp = vsync_time;
            view_params = self.last_good_view_params;
            buffer_ptr = std::ptr::null_mut();
        }

        let swapchain_idx = self.swapchain.acquire_image().unwrap() as usize;

        self.swapchain.wait_image(xr::Duration::INFINITE).unwrap();

        unsafe {
            self.renderer
                .render_from_android_buffer(buffer_ptr, 0, swapchain_idx)
        };

        self.swapchain.release_image().unwrap();

        if !buffer_ptr.is_null() {
            if let Some(now) = crate::xr_runtime_now(&self.xr_context.instance) {
                self.core_context
                    .report_submit(timestamp, vsync_time.saturating_sub(now));
            }
        }

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.view_resolution.x as _,
                height: self.view_resolution.y as _,
            },
        };

        CompositionLayerBuilder::new(
            &self.reference_space,
            [
                xr::CompositionLayerProjectionView::new()
                    .pose(to_xr_pose(view_params[0].pose))
                    .fov(to_xr_fov(view_params[0].fov))
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchain)
                            .image_array_index(0)
                            .image_rect(rect),
                    ),
                xr::CompositionLayerProjectionView::new()
                    .pose(to_xr_pose(view_params[1].pose))
                    .fov(to_xr_fov(view_params[1].fov))
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchain)
                            .image_array_index(1)
                            .image_rect(rect),
                    ),
            ],
        )
    }
}

impl Drop for StreamContext {
    fn drop(&mut self) {
        self.input_thread_running.set(false);
        self.input_thread.take().unwrap().join().ok();
    }
}

fn stream_input_loop(
    core_ctx: &ClientCoreContext,
    xr_ctx: XrContext,
    interaction_ctx: &InteractionContext,
    reference_space: Arc<xr::Space>,
    refresh_rate: f32,
    running: Arc<RelaxedAtomic>,
) {
    let mut last_hand_positions = [Vec3::ZERO; 2];

    let mut deadline = Instant::now();
    let frame_interval = Duration::from_secs_f32(1.0 / refresh_rate);
    while running.value() {
        // Streaming related inputs are updated here. Make sure every input poll is done in this
        // thread
        if let Err(e) = xr_ctx
            .session
            .sync_actions(&[(&interaction_ctx.action_set).into()])
        {
            error!("{e}");
            return;
        }

        let Some(now) = crate::xr_runtime_now(&xr_ctx.instance) else {
            error!("Cannot poll tracking: invalid time");
            return;
        };

        let target_timestamp =
            now + Duration::min(core_ctx.get_head_prediction_offset(), MAX_PREDICTION);

        let Ok((view_flags, views)) = xr_ctx.session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            crate::to_xr_time(target_timestamp),
            &reference_space,
        ) else {
            error!("Cannot locate views");
            continue;
        };

        if !view_flags.contains(xr::ViewStateFlags::POSITION_VALID)
            || !view_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
        {
            continue;
        }

        let view_params = [
            ViewParams {
                pose: from_xr_pose(views[0].pose),
                fov: crate::from_xr_fov(views[0].fov),
            },
            ViewParams {
                pose: from_xr_pose(views[1].pose),
                fov: crate::from_xr_fov(views[1].fov),
            },
        ];

        let mut device_motions = Vec::with_capacity(3);

        let tracker_time = crate::to_xr_time(
            now + Duration::min(core_ctx.get_tracker_prediction_offset(), MAX_PREDICTION),
        );

        let (left_hand_motion, left_hand_skeleton) = crate::interaction::get_hand_motion(
            &xr_ctx.session,
            &reference_space,
            tracker_time,
            &interaction_ctx.hands_interaction[0],
            &mut last_hand_positions[0],
        );
        let (right_hand_motion, right_hand_skeleton) = crate::interaction::get_hand_motion(
            &xr_ctx.session,
            &reference_space,
            tracker_time,
            &interaction_ctx.hands_interaction[1],
            &mut last_hand_positions[1],
        );

        if let Some(motion) = left_hand_motion {
            device_motions.push((*HAND_LEFT_ID, motion));
        }
        if let Some(motion) = right_hand_motion {
            device_motions.push((*HAND_RIGHT_ID, motion));
        }

        let face_data = FaceData {
            eye_gazes: interaction::get_eye_gazes(
                &xr_ctx.session,
                &interaction_ctx.face_sources,
                &reference_space,
                crate::to_xr_time(now),
            ),
            fb_face_expression: interaction::get_fb_face_expression(
                &interaction_ctx.face_sources,
                crate::to_xr_time(now),
            ),
            htc_eye_expression: interaction::get_htc_eye_expression(&interaction_ctx.face_sources),
            htc_lip_expression: interaction::get_htc_lip_expression(&interaction_ctx.face_sources),
        };

        if let Some(body_tracker_full_body_meta) =
            &interaction_ctx.body_sources.body_tracker_full_body_meta
        {
            device_motions.append(&mut interaction::get_meta_body_tracking_full_body_points(
                &reference_space,
                crate::to_xr_time(now),
                body_tracker_full_body_meta,
                interaction_ctx.body_sources.enable_full_body,
            ));
        }

        core_ctx.send_tracking(
            target_timestamp,
            view_params,
            device_motions,
            [left_hand_skeleton, right_hand_skeleton],
            face_data,
        );

        let button_entries =
            interaction::update_buttons(&xr_ctx.session, &interaction_ctx.button_actions);
        if !button_entries.is_empty() {
            core_ctx.send_buttons(button_entries);
        }

        deadline += frame_interval / 3;
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
}
