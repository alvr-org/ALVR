use crate::{
    graphics::{self, CompositionLayerBuilder},
    interaction::{self, InteractionContext},
    XrContext,
};
use alvr_client_core::{
    decoder::{self, DecoderConfig, DecoderSource},
    graphics::{GraphicsContext, StreamRenderer},
    ClientCoreContext, Platform,
};
use alvr_common::{
    anyhow::Result,
    error,
    glam::{UVec2, Vec2},
    Pose, RelaxedAtomic, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID,
};
use alvr_packets::{FaceData, StreamConfig, ViewParams};
use alvr_session::{
    BodyTrackingSourcesConfig, ClientsideFoveationConfig, ClientsideFoveationMode, CodecType,
    FaceTrackingSourcesConfig, FoveatedEncodingConfig, MediacodecDataType,
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

#[derive(PartialEq, Clone)]
pub struct ParsedStreamConfig {
    pub view_resolution: UVec2,
    pub refresh_rate_hint: f32,
    pub encoding_gamma: f32,
    pub enable_hdr: bool,
    pub foveated_encoding_config: Option<FoveatedEncodingConfig>,
    pub clientside_foveation_config: Option<ClientsideFoveationConfig>,
    pub face_sources_config: Option<FaceTrackingSourcesConfig>,
    pub body_sources_config: Option<BodyTrackingSourcesConfig>,
    pub prefers_multimodal_input: bool,
    pub force_software_decoder: bool,
    pub max_buffering_frames: f32,
    pub buffering_history_weight: f32,
    pub decoder_options: Vec<(String, MediacodecDataType)>,
}

impl ParsedStreamConfig {
    pub fn new(config: &StreamConfig) -> ParsedStreamConfig {
        ParsedStreamConfig {
            view_resolution: config.negotiated_config.view_resolution,
            refresh_rate_hint: config.negotiated_config.refresh_rate_hint,
            encoding_gamma: config.negotiated_config.encoding_gamma,
            enable_hdr: config.negotiated_config.enable_hdr,
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
            face_sources_config: config
                .settings
                .headset
                .face_tracking
                .as_option()
                .map(|c| c.sources.clone()),
            body_sources_config: config
                .settings
                .headset
                .body_tracking
                .as_option()
                .map(|c| c.sources.clone()),
            prefers_multimodal_input: config
                .settings
                .headset
                .controllers
                .as_option()
                .map(|c| c.multimodal_tracking)
                .unwrap_or(false),
            force_software_decoder: config.settings.video.force_software_decoder,
            max_buffering_frames: config.settings.video.max_buffering_frames,
            buffering_history_weight: config.settings.video.buffering_history_weight,
            decoder_options: config.settings.video.mediacodec_extra_options.clone(),
        }
    }
}

pub struct StreamContext {
    core_context: Arc<ClientCoreContext>,
    xr_context: XrContext,
    interaction_context: Arc<InteractionContext>,
    reference_space: Arc<xr::Space>,
    swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
    last_good_view_params: [ViewParams; 2],
    input_thread: Option<JoinHandle<()>>,
    input_thread_running: Arc<RelaxedAtomic>,
    config: ParsedStreamConfig,
    renderer: StreamRenderer,
    decoder: Option<(DecoderConfig, DecoderSource)>,
}

impl StreamContext {
    pub fn new(
        core_ctx: Arc<ClientCoreContext>,
        xr_ctx: XrContext,
        gfx_ctx: Rc<GraphicsContext>,
        interaction_ctx: Arc<InteractionContext>,
        platform: Platform,
        config: ParsedStreamConfig,
    ) -> StreamContext {
        if xr_ctx.instance.exts().fb_display_refresh_rate.is_some() {
            xr_ctx
                .session
                .request_display_refresh_rate(config.refresh_rate_hint)
                .unwrap();
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

        let format = graphics::swapchain_format(&gfx_ctx, &xr_ctx.session, config.enable_hdr);

        let swapchains = [
            graphics::create_swapchain(
                &xr_ctx.session,
                &gfx_ctx,
                config.view_resolution,
                format,
                foveation_profile.as_ref(),
            ),
            graphics::create_swapchain(
                &xr_ctx.session,
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
        );

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

        let reference_space = Arc::new(interaction::get_reference_space(
            &xr_ctx.session,
            xr::ReferenceSpaceType::STAGE,
        ));

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

        StreamContext {
            core_context: core_ctx,
            xr_context: xr_ctx,
            interaction_context: interaction_ctx,
            reference_space,
            swapchains,
            last_good_view_params: [ViewParams::default(); 2],
            input_thread: Some(input_thread),
            input_thread_running,
            config,
            renderer,
            decoder: None,
        }
    }

    pub fn update_reference_space(&mut self) {
        self.input_thread_running.set(false);

        self.reference_space = Arc::new(interaction::get_reference_space(
            &self.xr_context.session,
            xr::ReferenceSpaceType::STAGE,
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
            let refresh_rate = self.config.refresh_rate_hint;
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

    pub fn maybe_initialize_decoder(&mut self, codec: CodecType, config_nal: Vec<u8>) {
        let new_config = DecoderConfig {
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
            let (mut sink, source) = decoder::create_decoder(config.clone(), {
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
    ) -> (CompositionLayerBuilder, Duration) {
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
            self.renderer
                .render(buffer_ptr, [left_swapchain_idx, right_swapchain_idx])
        };

        self.swapchains[0].release_image().unwrap();
        self.swapchains[1].release_image().unwrap();

        if !buffer_ptr.is_null() {
            if let Some(xr_now) = crate::xr_runtime_now(&self.xr_context.instance) {
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

        let layer = CompositionLayerBuilder::new(
            &self.reference_space,
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
    xr_ctx: XrContext,
    interaction_ctx: &InteractionContext,
    reference_space: Arc<xr::Space>,
    refresh_rate: f32,
    running: Arc<RelaxedAtomic>,
) {
    let mut last_controller_poses = [Pose::default(); 2];
    let mut last_palm_poses = [Pose::default(); 2];
    let mut last_ipd = 0.0;

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

        let Some(xr_now) = crate::xr_runtime_now(&xr_ctx.instance) else {
            error!("Cannot poll tracking: invalid time");
            return;
        };

        let mut device_motions = Vec::with_capacity(3);

        let Some((head_motion, local_views)) =
            interaction::get_head_data(&xr_ctx.session, &reference_space, xr_now, &mut last_ipd)
        else {
            continue;
        };

        device_motions.push((*HEAD_ID, head_motion));

        if let Some(views) = local_views {
            core_ctx.send_view_params(views);
        }

        let (left_hand_motion, left_hand_skeleton) = crate::interaction::get_hand_data(
            &xr_ctx.session,
            &reference_space,
            xr_now,
            &interaction_ctx.hands_interaction[0],
            &mut last_controller_poses[0],
            &mut last_palm_poses[0],
        );
        let (right_hand_motion, right_hand_skeleton) = crate::interaction::get_hand_data(
            &xr_ctx.session,
            &reference_space,
            xr_now,
            &interaction_ctx.hands_interaction[1],
            &mut last_controller_poses[1],
            &mut last_palm_poses[1],
        );

        // Note: When multimodal input is enabled, we are sure that when free hands are used
        // (not holding controllers) the controller data is None.
        if interaction_ctx.uses_multimodal_hands || left_hand_skeleton.is_none() {
            if let Some(motion) = left_hand_motion {
                device_motions.push((*HAND_LEFT_ID, motion));
            }
        }
        if interaction_ctx.uses_multimodal_hands || right_hand_skeleton.is_none() {
            if let Some(motion) = right_hand_motion {
                device_motions.push((*HAND_RIGHT_ID, motion));
            }
        }

        let face_data = FaceData {
            eye_gazes: interaction::get_eye_gazes(
                &xr_ctx.session,
                &interaction_ctx.face_sources,
                &reference_space,
                xr_now,
            ),
            fb_face_expression: interaction::get_fb_face_expression(
                &interaction_ctx.face_sources,
                xr_now,
            ),
            htc_eye_expression: interaction::get_htc_eye_expression(&interaction_ctx.face_sources),
            htc_lip_expression: interaction::get_htc_lip_expression(&interaction_ctx.face_sources),
        };

        if let Some((tracker, joint_count)) = &interaction_ctx.body_sources.body_tracker_fb {
            device_motions.append(&mut interaction::get_fb_body_tracking_points(
                &reference_space,
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

        let button_entries =
            interaction::update_buttons(&xr_ctx.session, &interaction_ctx.button_actions);
        if !button_entries.is_empty() {
            core_ctx.send_buttons(button_entries);
        }

        deadline += frame_interval / 3;
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
}
