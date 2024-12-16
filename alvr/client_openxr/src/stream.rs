use crate::{
    graphics::{self, ProjectionLayerAlphaConfig, ProjectionLayerBuilder},
    interaction::{self, InteractionContext, InteractionSourcesConfig},
};
use alvr_client_core::{
    graphics::{GraphicsContext, StreamRenderer, StreamViewParams},
    video_decoder::{self, VideoDecoderConfig, VideoDecoderSource},
    ClientCoreContext, Platform,
};
use alvr_common::{
    anyhow::Result,
    error,
    glam::{Quat, UVec2, Vec2},
    parking_lot::RwLock,
    Pose, RelaxedAtomic, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID,
};
use alvr_packets::{FaceData, StreamConfig, ViewParams};
use alvr_session::{
    ClientsideFoveationConfig, ClientsideFoveationMode, CodecType, FoveatedEncodingConfig,
    MediacodecProperty, PassthroughMode,
};
use openxr as xr;
use std::{
    ptr,
    rc::Rc,
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const DECODER_MAX_TIMEOUT_MULTIPLIER: f32 = 0.8;

pub struct ParsedStreamConfig {
    pub view_resolution: UVec2,
    pub refresh_rate_hint: f32,
    pub encoding_gamma: f32,
    pub enable_hdr: bool,
    pub passthrough: Option<PassthroughMode>,
    pub foveated_encoding_config: Option<FoveatedEncodingConfig>,
    pub clientside_foveation_config: Option<ClientsideFoveationConfig>,
    pub force_software_decoder: bool,
    pub max_buffering_frames: f32,
    pub buffering_history_weight: f32,
    pub decoder_options: Vec<(String, MediacodecProperty)>,
    pub interaction_sources: InteractionSourcesConfig,
}

impl ParsedStreamConfig {
    pub fn new(config: &StreamConfig) -> Self {
        Self {
            view_resolution: config.negotiated_config.view_resolution,
            refresh_rate_hint: config.negotiated_config.refresh_rate_hint,
            encoding_gamma: config.negotiated_config.encoding_gamma,
            enable_hdr: config.negotiated_config.enable_hdr,
            passthrough: config.settings.video.passthrough.as_option().cloned(),
            foveated_encoding_config: config
                .negotiated_config
                .enable_foveated_encoding
                .then(|| config.settings.video.foveated_encoding.as_option().cloned())
                .flatten(),
            clientside_foveation_config: config
                .settings
                .video
                .clientside_foveation
                .as_option()
                .cloned(),
            force_software_decoder: config.settings.video.force_software_decoder,
            max_buffering_frames: config.settings.video.max_buffering_frames,
            buffering_history_weight: config.settings.video.buffering_history_weight,
            decoder_options: config.settings.video.mediacodec_extra_options.clone(),
            interaction_sources: InteractionSourcesConfig::new(config),
        }
    }
}

pub struct StreamContext {
    core_context: Arc<ClientCoreContext>,
    xr_session: xr::Session<xr::OpenGlEs>,
    interaction_context: Arc<RwLock<InteractionContext>>,
    stage_reference_space: Arc<xr::Space>,
    view_reference_space: Arc<xr::Space>,
    swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
    last_good_view_params: [ViewParams; 2],
    input_thread: Option<JoinHandle<()>>,
    input_thread_running: Arc<RelaxedAtomic>,
    config: ParsedStreamConfig,
    renderer: StreamRenderer,
    decoder: Option<(VideoDecoderConfig, VideoDecoderSource)>,
}

impl StreamContext {
    pub fn new(
        core_ctx: Arc<ClientCoreContext>,
        xr_session: xr::Session<xr::OpenGlEs>,
        gfx_ctx: Rc<GraphicsContext>,
        interaction_ctx: Arc<RwLock<InteractionContext>>,
        platform: Platform,
        config: ParsedStreamConfig,
    ) -> StreamContext {
        interaction_ctx
            .write()
            .select_sources(&config.interaction_sources);

        let xr_exts = xr_session.instance().exts();

        if xr_exts.fb_display_refresh_rate.is_some() {
            xr_session
                .request_display_refresh_rate(config.refresh_rate_hint)
                .unwrap();
        }

        let foveation_profile = if let Some(config) = &config.clientside_foveation_config {
            if xr_exts.fb_swapchain_update_state.is_some()
                && xr_exts.fb_foveation.is_some()
                && xr_exts.fb_foveation_configuration.is_some()
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

                xr_session
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

        let format = graphics::swapchain_format(&gfx_ctx, &xr_session, config.enable_hdr);

        let swapchains = [
            graphics::create_swapchain(
                &xr_session,
                &gfx_ctx,
                config.view_resolution,
                format,
                foveation_profile.as_ref(),
            ),
            graphics::create_swapchain(
                &xr_session,
                &gfx_ctx,
                config.view_resolution,
                format,
                foveation_profile.as_ref(),
            ),
        ];

        let renderer = StreamRenderer::new(
            gfx_ctx,
            config.view_resolution,
            [
                swapchains[0]
                    .enumerate_images()
                    .unwrap()
                    .iter()
                    .map(|i| *i as _)
                    .collect(),
                swapchains[1]
                    .enumerate_images()
                    .unwrap()
                    .iter()
                    .map(|i| *i as _)
                    .collect(),
            ],
            format,
            config.foveated_encoding_config.clone(),
            platform != Platform::Lynx && !((platform.is_pico()) && config.enable_hdr),
            !config.enable_hdr,
            config.encoding_gamma,
            config.passthrough.clone(),
        );

        core_ctx.send_active_interaction_profile(
            *HAND_LEFT_ID,
            interaction_ctx.read().hands_interaction[0].controllers_profile_id,
        );
        core_ctx.send_active_interaction_profile(
            *HAND_RIGHT_ID,
            interaction_ctx.read().hands_interaction[1].controllers_profile_id,
        );

        let input_thread_running = Arc::new(RelaxedAtomic::new(false));

        let stage_reference_space = Arc::new(interaction::get_reference_space(
            &xr_session,
            xr::ReferenceSpaceType::STAGE,
        ));
        let view_reference_space = Arc::new(interaction::get_reference_space(
            &xr_session,
            xr::ReferenceSpaceType::VIEW,
        ));

        let mut this = StreamContext {
            core_context: core_ctx,
            xr_session,
            interaction_context: interaction_ctx,
            stage_reference_space,
            view_reference_space,
            swapchains,
            last_good_view_params: [ViewParams::default(); 2],
            input_thread: None,
            input_thread_running,
            config,
            renderer,
            decoder: None,
        };

        this.update_reference_space();

        this
    }

    pub fn uses_passthrough(&self) -> bool {
        self.config.passthrough.is_some()
    }

    pub fn update_reference_space(&mut self) {
        self.input_thread_running.set(false);

        self.stage_reference_space = Arc::new(interaction::get_reference_space(
            &self.xr_session,
            xr::ReferenceSpaceType::STAGE,
        ));
        self.view_reference_space = Arc::new(interaction::get_reference_space(
            &self.xr_session,
            xr::ReferenceSpaceType::VIEW,
        ));

        self.core_context.send_playspace(
            self.xr_session
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
            let xr_session = self.xr_session.clone();
            let interaction_ctx = Arc::clone(&self.interaction_context);
            let stage_reference_space = Arc::clone(&self.stage_reference_space);
            let view_reference_space = Arc::clone(&self.view_reference_space);
            let refresh_rate = self.config.refresh_rate_hint;
            let running = Arc::clone(&self.input_thread_running);
            move || {
                stream_input_loop(
                    &core_ctx,
                    xr_session,
                    &interaction_ctx,
                    &stage_reference_space,
                    &view_reference_space,
                    refresh_rate,
                    running,
                )
            }
        }));
    }

    pub fn maybe_initialize_decoder(&mut self, codec: CodecType, config_nal: Vec<u8>) {
        let new_config = VideoDecoderConfig {
            codec,
            force_software_decoder: self.config.force_software_decoder,
            max_buffering_frames: self.config.max_buffering_frames,
            buffering_history_weight: self.config.buffering_history_weight,
            options: self.config.decoder_options.clone(),
            config_buffer: config_nal,
        };

        let maybe_config = if let Some((config, _)) = &self.decoder {
            (new_config != *config).then_some(new_config)
        } else {
            Some(new_config)
        };

        if let Some(config) = maybe_config {
            let (mut sink, source) = video_decoder::create_decoder(config.clone(), {
                let ctx = Arc::clone(&self.core_context);
                move |maybe_timestamp: Result<Duration>| match maybe_timestamp {
                    Ok(timestamp) => ctx.report_frame_decoded(timestamp),
                    Err(e) => ctx.report_fatal_decoder_error(&e.to_string()),
                }
            });
            self.decoder = Some((config, source));

            self.core_context.set_decoder_input_callback(Box::new(
                move |timestamp, buffer| -> bool { sink.push_nal(timestamp, buffer) },
            ));
        }
    }

    pub fn render(
        &mut self,
        frame_interval: Duration,
        vsync_time: Duration,
    ) -> (ProjectionLayerBuilder, Duration) {
        let frame_poll_deadline = Instant::now()
            + Duration::from_secs_f32(
                frame_interval.as_secs_f32() * DECODER_MAX_TIMEOUT_MULTIPLIER,
            );
        let mut frame_result = None;
        if let Some((_, source)) = &mut self.decoder {
            while frame_result.is_none() && Instant::now() < frame_poll_deadline {
                frame_result = source.get_frame();
                thread::sleep(Duration::from_micros(500));
            }
        }

        let (timestamp, view_params, buffer_ptr) =
            if let Some((timestamp, buffer_ptr)) = frame_result {
                let view_params = self.core_context.report_compositor_start(timestamp);

                // Avoid passing invalid timestamp to runtime
                let timestamp =
                    Duration::max(timestamp, vsync_time.saturating_sub(Duration::from_secs(1)));

                self.last_good_view_params = view_params;

                (timestamp, view_params, buffer_ptr)
            } else {
                (vsync_time, self.last_good_view_params, ptr::null_mut())
            };

        let left_swapchain_idx = self.swapchains[0].acquire_image().unwrap();
        let right_swapchain_idx = self.swapchains[1].acquire_image().unwrap();

        self.swapchains[0]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();
        self.swapchains[1]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();

        unsafe {
            self.renderer.render(
                buffer_ptr,
                [
                    StreamViewParams {
                        swapchain_index: left_swapchain_idx,
                        reprojection_rotation: Quat::IDENTITY,
                        fov: view_params[0].fov,
                    },
                    StreamViewParams {
                        swapchain_index: right_swapchain_idx,
                        reprojection_rotation: Quat::IDENTITY,
                        fov: view_params[1].fov,
                    },
                ],
            )
        };

        self.swapchains[0].release_image().unwrap();
        self.swapchains[1].release_image().unwrap();

        if !buffer_ptr.is_null() {
            if let Some(xr_now) = crate::xr_runtime_now(self.xr_session.instance()) {
                self.core_context.report_submit(
                    timestamp,
                    vsync_time.saturating_sub(Duration::from_nanos(xr_now.as_nanos() as u64)),
                );
            }
        }

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.config.view_resolution.x as _,
                height: self.config.view_resolution.y as _,
            },
        };

        let layer = ProjectionLayerBuilder::new(
            &self.stage_reference_space,
            [
                xr::CompositionLayerProjectionView::new()
                    .pose(crate::to_xr_pose(view_params[0].pose))
                    .fov(crate::to_xr_fov(view_params[0].fov))
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchains[0])
                            .image_array_index(0)
                            .image_rect(rect),
                    ),
                xr::CompositionLayerProjectionView::new()
                    .pose(crate::to_xr_pose(view_params[1].pose))
                    .fov(crate::to_xr_fov(view_params[1].fov))
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchains[1])
                            .image_array_index(0)
                            .image_rect(rect),
                    ),
            ],
            self.config
                .passthrough
                .clone()
                .map(|mode| ProjectionLayerAlphaConfig {
                    premultiplied: !matches!(mode, PassthroughMode::Blend { .. }),
                }),
        );

        (layer, timestamp)
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
    xr_session: xr::Session<xr::OpenGlEs>,
    interaction_ctx: &RwLock<InteractionContext>,
    stage_reference_space: &xr::Space,
    view_reference_space: &xr::Space,
    refresh_rate: f32,
    running: Arc<RelaxedAtomic>,
) {
    let platform = alvr_system_info::platform();

    let mut last_controller_poses = [Pose::default(); 2];
    let mut last_palm_poses = [Pose::default(); 2];
    let mut last_view_params = [ViewParams::default(); 2];

    let mut deadline = Instant::now();
    let frame_interval = Duration::from_secs_f32(1.0 / refresh_rate);
    while running.value() {
        let int_ctx = &*interaction_ctx.read();
        // Streaming related inputs are updated here. Make sure every input poll is done in this
        // thread
        if let Err(e) = xr_session.sync_actions(&[(&int_ctx.action_set).into()]) {
            error!("{e}");
            return;
        }

        let Some(xr_now) = crate::xr_runtime_now(xr_session.instance()) else {
            error!("Cannot poll tracking: invalid time");
            return;
        };

        let Some((head_motion, local_views)) = interaction::get_head_data(
            &xr_session,
            platform,
            stage_reference_space,
            view_reference_space,
            xr_now,
            &last_view_params,
        ) else {
            continue;
        };

        if let Some(views) = local_views {
            core_ctx.send_view_params(views);
            last_view_params = views;
        }

        let mut device_motions = Vec::with_capacity(3);

        device_motions.push((*HEAD_ID, head_motion));

        let (left_hand_motion, left_hand_skeleton) = crate::interaction::get_hand_data(
            &xr_session,
            stage_reference_space,
            xr_now,
            &int_ctx.hands_interaction[0],
            &mut last_controller_poses[0],
            &mut last_palm_poses[0],
        );
        let (right_hand_motion, right_hand_skeleton) = crate::interaction::get_hand_data(
            &xr_session,
            stage_reference_space,
            xr_now,
            &int_ctx.hands_interaction[1],
            &mut last_controller_poses[1],
            &mut last_palm_poses[1],
        );

        // Note: When multimodal input is enabled, we are sure that when free hands are used
        // (not holding controllers) the controller data is None.
        if int_ctx.multimodal_hands_enabled || left_hand_skeleton.is_none() {
            if let Some(motion) = left_hand_motion {
                device_motions.push((*HAND_LEFT_ID, motion));
            }
        }
        if int_ctx.multimodal_hands_enabled || right_hand_skeleton.is_none() {
            if let Some(motion) = right_hand_motion {
                device_motions.push((*HAND_RIGHT_ID, motion));
            }
        }

        let face_data = FaceData {
            eye_gazes: interaction::get_eye_gazes(
                &xr_session,
                &int_ctx.face_sources,
                stage_reference_space,
                xr_now,
            ),
            fb_face_expression: interaction::get_fb_face_expression(&int_ctx.face_sources, xr_now),
            htc_eye_expression: interaction::get_htc_eye_expression(&int_ctx.face_sources),
            htc_lip_expression: interaction::get_htc_lip_expression(&int_ctx.face_sources),
        };

        if let Some((tracker, joint_count)) = &int_ctx.body_sources.body_tracker_fb {
            device_motions.append(&mut interaction::get_fb_body_tracking_points(
                stage_reference_space,
                xr_now,
                tracker,
                *joint_count,
            ));
        }

        core_ctx.send_tracking(
            Duration::from_nanos(xr_now.as_nanos() as u64),
            device_motions,
            [left_hand_skeleton, right_hand_skeleton],
            face_data,
        );

        let button_entries = interaction::update_buttons(&xr_session, &int_ctx.button_actions);
        if !button_entries.is_empty() {
            core_ctx.send_buttons(button_entries);
        }

        deadline += frame_interval / 3;
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
}
