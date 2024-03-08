use crate::{
    graphics,
    interaction::{self, InteractionContext},
    XrContext,
};
use alvr_client_core::{ClientCoreContext, Platform};
use alvr_common::{
    error,
    glam::{UVec2, Vec2, Vec3},
    parking_lot::RwLock,
    DeviceMotion, Pose, RelaxedAtomic, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID,
};
use alvr_packets::{FaceData, NegotiatedStreamingConfig, Tracking};
use alvr_session::{
    BodyTrackingSourcesConfig, ClientsideFoveationConfig, ClientsideFoveationMode,
    FaceTrackingSourcesConfig, FoveatedEncodingConfig, Settings,
};
use openxr as xr;
use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

// When the latency goes too high, if prediction offset is not capped tracking poll will fail.
const MAX_PREDICTION: Duration = Duration::from_millis(70);
const IPD_CHANGE_EPS: f32 = 0.001;

#[derive(PartialEq)]
pub struct StreamConfig {
    pub view_resolution: UVec2,
    pub refresh_rate_hint: f32,
    pub foveated_encoding_config: Option<FoveatedEncodingConfig>,
    pub clientside_foveation_config: Option<ClientsideFoveationConfig>,
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

struct ViewsHistorySample {
    timestamp: Duration,
    views: Vec<xr::View>,
}

pub struct StreamContext {
    core_context: Arc<ClientCoreContext>,
    xr_instance: xr::Instance,
    rect: xr::Rect2Di,
    swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
    views_history: VecDeque<ViewsHistorySample>,
    last_good_views: Vec<xr::View>,
    running: Arc<RelaxedAtomic>,
    input_thread: Option<JoinHandle<()>>,
    views_history_receiver: mpsc::Receiver<ViewsHistorySample>,
}

impl StreamContext {
    pub fn new(
        core_ctx: Arc<ClientCoreContext>,
        xr_ctx: &XrContext,
        interaction_ctx: Arc<InteractionContext>,
        reference_space: Arc<RwLock<xr::Space>>,
        platform: Platform,
        config: &StreamConfig,
    ) -> StreamContext {
        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: config.view_resolution.x as _,
                height: config.view_resolution.y as _,
            },
        };

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

        let swapchains = [
            graphics::create_swapchain(
                &xr_ctx.session,
                config.view_resolution,
                foveation_profile.as_ref(),
            ),
            graphics::create_swapchain(
                &xr_ctx.session,
                config.view_resolution,
                foveation_profile.as_ref(),
            ),
        ];

        alvr_client_core::opengl::start_stream(
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
            config.foveated_encoding_config.clone(),
            platform != Platform::Lynx,
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

        let running = Arc::new(RelaxedAtomic::new(true));

        let (views_history_sender, views_history_receiver) = mpsc::channel();

        let mut input_context = StreamInputContext {
            views_history_sender,
            reference_space,
            last_ipd: 0.0,
            last_hand_positions: [Vec3::ZERO; 2],
        };
        let input_thread = thread::spawn({
            let core_ctx = Arc::clone(&core_ctx);
            let xr_ctx = xr_ctx.clone();
            let running = Arc::clone(&running);
            let interaction_ctx = Arc::clone(&interaction_ctx);
            let input_rate = config.refresh_rate_hint;
            move || {
                let mut deadline = Instant::now();
                let frame_interval = Duration::from_secs_f32(1.0 / input_rate);
                while running.value() {
                    stream_input_pipeline(&core_ctx, &xr_ctx, &interaction_ctx, &mut input_context);

                    deadline += frame_interval / 3;
                    thread::sleep(deadline.saturating_duration_since(Instant::now()));
                }
            }
        });

        StreamContext {
            core_context: core_ctx,
            xr_instance: xr_ctx.instance.clone(),
            rect,
            swapchains,
            views_history: VecDeque::new(),
            last_good_views: vec![crate::default_view(), crate::default_view()],
            running,
            input_thread: Some(input_thread),
            views_history_receiver,
        }
    }

    pub fn render(
        &mut self,
        timestamp: Duration,
        hardware_buffer: *mut c_void,
        vsync_time: Duration,
    ) -> [xr::CompositionLayerProjectionView<xr::OpenGlEs>; 2] {
        while let Ok(views) = self.views_history_receiver.try_recv() {
            if self.views_history.len() > 360 {
                self.views_history.pop_front();
            }

            self.views_history.push_back(views);
        }

        let mut views = self.last_good_views.clone();

        for history_frame in &self.views_history {
            if history_frame.timestamp == timestamp {
                views = history_frame.views.clone();
            }
        }
        self.last_good_views = views.clone();

        let left_swapchain_idx = self.swapchains[0].acquire_image().unwrap();
        let right_swapchain_idx = self.swapchains[1].acquire_image().unwrap();

        self.swapchains[0]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();
        self.swapchains[1]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();

        alvr_client_core::opengl::render_stream(
            hardware_buffer,
            [left_swapchain_idx, right_swapchain_idx],
        );

        self.swapchains[0].release_image().unwrap();
        self.swapchains[1].release_image().unwrap();

        if !hardware_buffer.is_null() {
            if let Some(now) = crate::xr_runtime_now(&self.xr_instance) {
                self.core_context
                    .report_submit(timestamp, vsync_time.saturating_sub(now));
            }
        }

        [
            xr::CompositionLayerProjectionView::new()
                .pose(views[0].pose)
                .fov(views[0].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchains[0])
                        .image_array_index(0)
                        .image_rect(self.rect),
                ),
            xr::CompositionLayerProjectionView::new()
                .pose(views[1].pose)
                .fov(views[1].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchains[1])
                        .image_array_index(0)
                        .image_rect(self.rect),
                ),
        ]
    }
}

impl Drop for StreamContext {
    fn drop(&mut self) {
        self.running.set(false);
        self.input_thread.take().unwrap().join().ok();
    }
}

struct StreamInputContext {
    views_history_sender: mpsc::Sender<ViewsHistorySample>,
    reference_space: Arc<RwLock<xr::Space>>,
    last_ipd: f32,
    last_hand_positions: [Vec3; 2],
}

fn stream_input_pipeline(
    core_ctx: &ClientCoreContext,
    xr_ctx: &XrContext,
    interaction_ctx: &InteractionContext,
    stream_ctx: &mut StreamInputContext,
) {
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

    let mut device_motions = Vec::with_capacity(3);

    'head_tracking: {
        let Ok((view_flags, views)) = xr_ctx.session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            crate::to_xr_time(target_timestamp),
            &stream_ctx.reference_space.read(),
        ) else {
            error!("Cannot locate views");
            break 'head_tracking;
        };

        if !view_flags.contains(xr::ViewStateFlags::POSITION_VALID)
            || !view_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
        {
            break 'head_tracking;
        }

        let ipd = (crate::to_vec3(views[0].pose.position) - crate::to_vec3(views[1].pose.position))
            .length();
        if f32::abs(stream_ctx.last_ipd - ipd) > IPD_CHANGE_EPS {
            core_ctx.send_views_config(
                [crate::to_fov(views[0].fov), crate::to_fov(views[1].fov)],
                ipd,
            );

            stream_ctx.last_ipd = ipd;
        }

        // Note: Here is assumed that views are on the same plane and orientation. The head position
        // is approximated as the center point between the eyes.
        let head_position =
            (crate::to_vec3(views[0].pose.position) + crate::to_vec3(views[1].pose.position)) / 2.0;
        let head_orientation = crate::to_quat(views[0].pose.orientation);

        stream_ctx
            .views_history_sender
            .send(ViewsHistorySample {
                timestamp: target_timestamp,
                views,
            })
            .ok();

        device_motions.push((
            *HEAD_ID,
            DeviceMotion {
                pose: Pose {
                    orientation: head_orientation,
                    position: head_position,
                },
                linear_velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
            },
        ));
    }

    let tracker_time = crate::to_xr_time(
        now + Duration::min(core_ctx.get_tracker_prediction_offset(), MAX_PREDICTION),
    );

    let (left_hand_motion, left_hand_skeleton) = crate::interaction::get_hand_motion(
        &xr_ctx.session,
        &stream_ctx.reference_space.read(),
        tracker_time,
        &interaction_ctx.hands_interaction[0],
        &mut stream_ctx.last_hand_positions[0],
    );
    let (right_hand_motion, right_hand_skeleton) = crate::interaction::get_hand_motion(
        &xr_ctx.session,
        &stream_ctx.reference_space.read(),
        tracker_time,
        &interaction_ctx.hands_interaction[1],
        &mut stream_ctx.last_hand_positions[1],
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
            &stream_ctx.reference_space.read(),
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
            &stream_ctx.reference_space.read(),
            crate::to_xr_time(now),
            body_tracker_full_body_meta,
            interaction_ctx.body_sources.enable_full_body,
        ));
    }

    core_ctx.send_tracking(Tracking {
        target_timestamp,
        device_motions,
        hand_skeletons: [left_hand_skeleton, right_hand_skeleton],
        face_data,
    });

    let button_entries =
        interaction::update_buttons(&xr_ctx.session, &interaction_ctx.button_actions);
    if !button_entries.is_empty() {
        core_ctx.send_buttons(button_entries);
    }
}
