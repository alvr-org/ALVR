mod interaction;

use alvr_client_core::{opengl::RenderViewInput, ClientCapabilities, ClientCoreEvent, Platform};
use alvr_common::{
    error,
    glam::{Quat, UVec2, Vec2, Vec3},
    info,
    parking_lot::RwLock,
    warn, DeviceMotion, Fov, Pose, RelaxedAtomic, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID,
};
use alvr_packets::{FaceData, Tracking};
use alvr_session::{
    BodyTrackingSourcesConfig, ClientsideFoveationConfig, ClientsideFoveationMode,
    FaceTrackingSourcesConfig, FoveatedEncodingConfig,
};
use interaction::InteractionContext;
use khronos_egl::{self as egl, EGL1_4};
use openxr as xr;
use std::{
    collections::VecDeque,
    path::Path,
    ptr,
    sync::{mpsc, Arc, Once},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

// When the latency goes too high, if prediction offset is not capped tracking poll will fail.
const MAX_PREDICTION: Duration = Duration::from_millis(70);
const IPD_CHANGE_EPS: f32 = 0.001;
const DECODER_MAX_TIMEOUT_MULTIPLIER: f32 = 0.8;

#[derive(Clone)]
pub struct XrContext {
    instance: xr::Instance,
    system: xr::SystemId,
    session: xr::Session<xr::OpenGlEs>,
}

pub struct SessionRunningContext {
    lobby_swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
    reference_space: Arc<RwLock<xr::Space>>,
    views_history_sender: mpsc::Sender<ViewsHistorySample>,
    views_history_receiver: mpsc::Receiver<ViewsHistorySample>,
    stream_context: Option<StreamContext>,
}

#[derive(PartialEq)]
struct StreamConfig {
    view_resolution: UVec2,
    refresh_rate_hint: f32,
    foveated_encoding_config: Option<FoveatedEncodingConfig>,
    clientside_foveation_config: Option<ClientsideFoveationConfig>,
    face_sources_config: Option<FaceTrackingSourcesConfig>,
    body_sources_config: Option<BodyTrackingSourcesConfig>,
}

struct StreamContext {
    view_resolution: UVec2,
    swapchains: [xr::Swapchain<xr::OpenGlEs>; 2],
    views_history: VecDeque<ViewsHistorySample>,
    last_good_views: Vec<xr::View>,
    running: Arc<RelaxedAtomic>,
    input_thread: Option<JoinHandle<()>>,
}

impl Drop for StreamContext {
    fn drop(&mut self) {
        self.running.set(false);
        self.input_thread.take().unwrap().join().ok();
    }
}

struct ViewsHistorySample {
    timestamp: Duration,
    views: Vec<xr::View>,
}

struct StreamInputContext {
    views_history_sender: mpsc::Sender<ViewsHistorySample>,
    reference_space: Arc<RwLock<xr::Space>>,
    last_ipd: f32,
    last_hand_positions: [Vec3; 2],
}

#[allow(unused)]
struct EglContext {
    instance: egl::DynamicInstance<EGL1_4>,
    display: egl::Display,
    config: egl::Config,
    context: egl::Context,
    dummy_surface: egl::Surface,
}

fn to_vec3(v: xr::Vector3f) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn to_quat(q: xr::Quaternionf) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}

fn to_pose(p: xr::Posef) -> Pose {
    Pose {
        orientation: to_quat(p.orientation),
        position: to_vec3(p.position),
    }
}

fn to_fov(f: xr::Fovf) -> Fov {
    Fov {
        left: f.angle_left,
        right: f.angle_right,
        up: f.angle_up,
        down: f.angle_down,
    }
}

fn default_view() -> xr::View {
    xr::View {
        pose: xr::Posef {
            orientation: xr::Quaternionf {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            position: xr::Vector3f::default(),
        },
        fov: xr::Fovf {
            angle_left: -1.0,
            angle_right: 1.0,
            angle_up: 1.0,
            angle_down: -1.0,
        },
    }
}

fn to_xr_time(timestamp: Duration) -> xr::Time {
    xr::Time::from_nanos(timestamp.as_nanos() as _)
}

#[allow(unused_variables)]
fn init_egl() -> EglContext {
    let instance = unsafe { egl::DynamicInstance::<EGL1_4>::load_required().unwrap() };

    let display = unsafe { instance.get_display(egl::DEFAULT_DISPLAY).unwrap() };

    let version = instance.initialize(display).unwrap();

    let mut configs = Vec::with_capacity(instance.get_config_count(display).unwrap());
    instance.get_configs(display, &mut configs).unwrap();

    const CONFIG_ATTRIBS: [i32; 19] = [
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::ALPHA_SIZE,
        8,
        egl::DEPTH_SIZE,
        0,
        egl::STENCIL_SIZE,
        0,
        egl::SAMPLES,
        0,
        egl::SURFACE_TYPE,
        egl::PBUFFER_BIT,
        egl::RENDERABLE_TYPE,
        egl::OPENGL_ES3_BIT,
        egl::NONE,
    ];
    let config = instance
        .choose_first_config(display, &CONFIG_ATTRIBS)
        .unwrap()
        .unwrap();

    instance.bind_api(egl::OPENGL_ES_API).unwrap();

    const CONTEXT_ATTRIBS: [i32; 3] = [egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE];
    let context = instance
        .create_context(display, config, None, &CONTEXT_ATTRIBS)
        .unwrap();

    const PBUFFER_ATTRIBS: [i32; 5] = [egl::WIDTH, 16, egl::HEIGHT, 16, egl::NONE];
    let dummy_surface = instance
        .create_pbuffer_surface(display, config, &PBUFFER_ATTRIBS)
        .unwrap();

    instance
        .make_current(
            display,
            Some(dummy_surface),
            Some(dummy_surface),
            Some(context),
        )
        .unwrap();

    EglContext {
        instance,
        display,
        config,
        context,
        dummy_surface,
    }
}

#[allow(unused)]
fn create_xr_session(
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
    egl_context: &EglContext,
) -> (
    xr::Session<xr::OpenGlEs>,
    xr::FrameWaiter,
    xr::FrameStream<xr::OpenGlEs>,
) {
    #[cfg(target_os = "android")]
    unsafe {
        xr_instance
            .create_session(
                xr_system,
                &xr::opengles::SessionCreateInfo::Android {
                    display: egl_context.display.as_ptr(),
                    config: egl_context.config.as_ptr(),
                    context: egl_context.context.as_ptr(),
                },
            )
            .unwrap()
    }
    #[cfg(not(target_os = "android"))]
    unimplemented!()
}

pub fn create_swapchain(
    session: &xr::Session<xr::OpenGlEs>,
    resolution: UVec2,
    foveation: Option<&xr::FoveationProfileFB>,
) -> xr::Swapchain<xr::OpenGlEs> {
    let swapchain_info = xr::SwapchainCreateInfo {
        create_flags: xr::SwapchainCreateFlags::EMPTY,
        usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT | xr::SwapchainUsageFlags::SAMPLED,
        format: glow::SRGB8_ALPHA8,
        sample_count: 1,
        width: resolution.x,
        height: resolution.y,
        face_count: 1,
        array_size: 1,
        mip_count: 1,
    };

    if let Some(foveation) = foveation {
        let swapchain = session
            .create_swapchain_with_foveation(
                &swapchain_info,
                xr::SwapchainCreateFoveationFlagsFB::SCALED_BIN,
            )
            .unwrap();

        swapchain.update_foveation(foveation).unwrap();

        swapchain
    } else {
        session.create_swapchain(&swapchain_info).unwrap()
    }
}

fn stream_input_pipeline(
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

    let Some(now) = xr_runtime_now(&xr_ctx.instance) else {
        error!("Cannot poll tracking: invalid time");
        return;
    };

    let target_timestamp = now
        + Duration::min(
            alvr_client_core::get_head_prediction_offset(),
            MAX_PREDICTION,
        );

    let mut device_motions = Vec::with_capacity(3);

    'head_tracking: {
        let Ok((view_flags, views)) = xr_ctx.session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            to_xr_time(target_timestamp),
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

        let ipd = (to_vec3(views[0].pose.position) - to_vec3(views[1].pose.position)).length();
        if f32::abs(stream_ctx.last_ipd - ipd) > IPD_CHANGE_EPS {
            alvr_client_core::send_views_config([to_fov(views[0].fov), to_fov(views[1].fov)], ipd);

            stream_ctx.last_ipd = ipd;
        }

        // Note: Here is assumed that views are on the same plane and orientation. The head position
        // is approximated as the center point between the eyes.
        let head_position =
            (to_vec3(views[0].pose.position) + to_vec3(views[1].pose.position)) / 2.0;
        let head_orientation = to_quat(views[0].pose.orientation);

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

    let tracker_time = to_xr_time(
        now + Duration::min(
            alvr_client_core::get_tracker_prediction_offset(),
            MAX_PREDICTION,
        ),
    );

    let (left_hand_motion, left_hand_skeleton) = interaction::get_hand_motion(
        &xr_ctx.session,
        &stream_ctx.reference_space.read(),
        tracker_time,
        &interaction_ctx.hands_interaction[0],
        &mut stream_ctx.last_hand_positions[0],
    );
    let (right_hand_motion, right_hand_skeleton) = interaction::get_hand_motion(
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
            to_xr_time(now),
        ),
        fb_face_expression: interaction::get_fb_face_expression(
            &interaction_ctx.face_sources,
            to_xr_time(now),
        ),
        htc_eye_expression: interaction::get_htc_eye_expression(&interaction_ctx.face_sources),
        htc_lip_expression: interaction::get_htc_lip_expression(&interaction_ctx.face_sources),
    };

    if let Some(body_tracker_full_body_meta) =
        &interaction_ctx.body_sources.body_tracker_full_body_meta
    {
        device_motions.append(&mut interaction::get_meta_body_tracking_full_body_points(
            &stream_ctx.reference_space.read(),
            to_xr_time(now),
            body_tracker_full_body_meta,
            interaction_ctx.body_sources.enable_full_body,
        ));
    }

    alvr_client_core::send_tracking(Tracking {
        target_timestamp,
        device_motions,
        hand_skeletons: [left_hand_skeleton, right_hand_skeleton],
        face_data,
    });

    let button_entries =
        interaction::update_buttons(&xr_ctx.session, &interaction_ctx.button_actions);
    if !button_entries.is_empty() {
        alvr_client_core::send_buttons(button_entries);
    }
}

fn initialize_stream(
    xr_ctx: &XrContext,
    interaction_ctx: Arc<InteractionContext>,
    session_ctx: &SessionRunningContext,
    platform: Platform,
    config: &StreamConfig,
) -> StreamContext {
    let stream_view_resolution = config.view_resolution;

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
        if config.combined_eye_gaze && matches!(platform, Platform::Pico4 | Platform::PicoNeo3) {
            alvr_client_core::try_get_permission("com.picovr.permission.EYE_TRACKING")
        }
        if config.face_tracking_fb && matches!(platform, Platform::Quest3 | Platform::QuestPro) {
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
        create_swapchain(
            &xr_ctx.session,
            stream_view_resolution,
            foveation_profile.as_ref(),
        ),
        create_swapchain(
            &xr_ctx.session,
            stream_view_resolution,
            foveation_profile.as_ref(),
        ),
    ];

    alvr_client_core::opengl::start_stream(
        stream_view_resolution,
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

    alvr_client_core::send_playspace(
        xr_ctx
            .session
            .reference_space_bounds_rect(xr::ReferenceSpaceType::STAGE)
            .unwrap()
            .map(|a| Vec2::new(a.width, a.height)),
    );

    alvr_client_core::send_active_interaction_profile(
        *HAND_LEFT_ID,
        interaction_ctx.hands_interaction[0].controllers_profile_id,
    );
    alvr_client_core::send_active_interaction_profile(
        *HAND_RIGHT_ID,
        interaction_ctx.hands_interaction[1].controllers_profile_id,
    );

    let running = Arc::new(RelaxedAtomic::new(true));

    let mut input_context = StreamInputContext {
        views_history_sender: session_ctx.views_history_sender.clone(),
        reference_space: Arc::clone(&session_ctx.reference_space),
        last_ipd: 0.0,
        last_hand_positions: [Vec3::ZERO; 2],
    };
    let input_thread = thread::spawn({
        let xr_ctx = xr_ctx.clone();
        let running = Arc::clone(&running);
        let interaction_ctx = Arc::clone(&interaction_ctx);
        let input_rate = config.refresh_rate_hint;
        move || {
            let mut deadline = Instant::now();
            let frame_interval = Duration::from_secs_f32(1.0 / input_rate);
            while running.value() {
                stream_input_pipeline(&xr_ctx, &interaction_ctx, &mut input_context);

                deadline += frame_interval / 3;
                thread::sleep(deadline.saturating_duration_since(Instant::now()));
            }
        }
    });

    StreamContext {
        view_resolution: stream_view_resolution,
        swapchains,
        views_history: VecDeque::new(),
        last_good_views: vec![default_view(), default_view()],
        running,
        input_thread: Some(input_thread),
    }
}

pub fn entry_point() {
    alvr_client_core::init_logging();

    let platform = alvr_client_core::platform();

    let loader_suffix = match platform {
        Platform::Quest1
        | Platform::Quest2
        | Platform::Quest3
        | Platform::QuestPro
        | Platform::QuestUnknown => "quest",
        Platform::PicoNeo3 | Platform::Pico4 => "pico",
        Platform::Yvr => "yvr",
        Platform::Lynx => "lynx",
        _ => "generic",
    };
    let xr_entry = unsafe {
        xr::Entry::load_from(Path::new(&format!("libopenxr_loader_{loader_suffix}.so"))).unwrap()
    };

    #[cfg(target_os = "android")]
    xr_entry.initialize_android_loader().unwrap();

    let available_extensions = xr_entry.enumerate_extensions().unwrap();

    // todo: switch to vulkan
    assert!(available_extensions.khr_opengl_es_enable);

    let mut exts = xr::ExtensionSet::default();
    exts.bd_controller_interaction = available_extensions.bd_controller_interaction;
    exts.ext_eye_gaze_interaction = available_extensions.ext_eye_gaze_interaction;
    exts.ext_hand_tracking = available_extensions.ext_hand_tracking;
    exts.fb_color_space = available_extensions.fb_color_space;
    exts.fb_display_refresh_rate = available_extensions.fb_display_refresh_rate;
    exts.fb_eye_tracking_social = available_extensions.fb_eye_tracking_social;
    exts.fb_face_tracking2 = available_extensions.fb_face_tracking2;
    exts.fb_body_tracking = available_extensions.fb_body_tracking;
    exts.meta_body_tracking_full_body = available_extensions.meta_body_tracking_full_body;
    exts.fb_foveation = available_extensions.fb_foveation;
    exts.fb_foveation_configuration = available_extensions.fb_foveation_configuration;
    exts.fb_swapchain_update_state = available_extensions.fb_swapchain_update_state;
    exts.htc_facial_tracking = available_extensions.htc_facial_tracking;
    exts.htc_vive_focus3_controller_interaction =
        available_extensions.htc_vive_focus3_controller_interaction;
    #[cfg(target_os = "android")]
    {
        exts.khr_android_create_instance = true;
    }
    exts.khr_convert_timespec_time = true;
    exts.khr_opengl_es_enable = true;

    let xr_instance = xr_entry
        .create_instance(
            &xr::ApplicationInfo {
                application_name: "ALVR Client",
                application_version: 0,
                engine_name: "ALVR",
                engine_version: 0,
            },
            &exts,
            &[],
        )
        .unwrap();

    let egl_context = init_egl();

    let mut last_lobby_message = String::new();
    let mut stream_config = None::<StreamConfig>;

    'session_loop: loop {
        let xr_system = xr_instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .unwrap();

        // mandatory call
        let _ = xr_instance
            .graphics_requirements::<xr::OpenGlEs>(xr_system)
            .unwrap();

        let (xr_session, mut xr_frame_waiter, mut xr_frame_stream) =
            create_xr_session(&xr_instance, xr_system, &egl_context);

        let xr_ctx = XrContext {
            instance: xr_instance.clone(),
            system: xr_system,
            session: xr_session.clone(),
        };

        let views_config = xr_instance
            .enumerate_view_configuration_views(
                xr_system,
                xr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .unwrap();
        assert_eq!(views_config.len(), 2);

        let default_view_resolution = UVec2::new(
            views_config[0].recommended_image_rect_width,
            views_config[0].recommended_image_rect_height,
        );

        let refresh_rates = if exts.fb_display_refresh_rate {
            xr_session.enumerate_display_refresh_rates().unwrap()
        } else {
            vec![90.0]
        };

        // Todo: refactor the logic to call this before the session creation
        static INIT_ONCE: Once = Once::new();
        INIT_ONCE.call_once(|| {
            alvr_client_core::initialize(ClientCapabilities {
                default_view_resolution,
                external_decoder: false,
                refresh_rates,
                foveated_encoding: platform != Platform::Unknown,
                encoder_high_profile: platform != Platform::Unknown,
                encoder_10_bits: platform != Platform::Unknown,
                encoder_av1: platform == Platform::Quest3,
            });
        });

        alvr_client_core::opengl::initialize();
        alvr_client_core::opengl::update_hud_message(&last_lobby_message);

        let interaction_context = Arc::new(interaction::initialize_interaction(
            &xr_ctx,
            platform,
            stream_config
                .as_ref()
                .and_then(|c| c.face_sources_config.clone()),
            stream_config
                .as_ref()
                .and_then(|c| c.body_sources_config.clone()),
        ));

        let mut session_running_context = None;

        let mut event_storage = xr::EventDataBuffer::new();
        'render_loop: loop {
            while let Some(event) = xr_instance.poll_event(&mut event_storage).unwrap() {
                match event {
                    xr::Event::EventsLost(event) => {
                        error!("OpenXR: lost {} events!", event.lost_event_count());
                    }
                    xr::Event::InstanceLossPending(_) => break 'session_loop,
                    xr::Event::SessionStateChanged(event) => match event.state() {
                        xr::SessionState::READY => {
                            xr_session
                                .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                                .unwrap();

                            let lobby_swapchains = [
                                create_swapchain(&xr_session, default_view_resolution, None),
                                create_swapchain(&xr_session, default_view_resolution, None),
                            ];

                            alvr_client_core::opengl::resume(
                                default_view_resolution,
                                [
                                    lobby_swapchains[0]
                                        .enumerate_images()
                                        .unwrap()
                                        .iter()
                                        .map(|i| *i as _)
                                        .collect(),
                                    lobby_swapchains[1]
                                        .enumerate_images()
                                        .unwrap()
                                        .iter()
                                        .map(|i| *i as _)
                                        .collect(),
                                ],
                            );

                            alvr_client_core::resume();

                            let (views_history_sender, views_history_receiver) = mpsc::channel();

                            let reference_space = Arc::new(RwLock::new(
                                xr_session
                                    .create_reference_space(
                                        xr::ReferenceSpaceType::STAGE,
                                        xr::Posef::IDENTITY,
                                    )
                                    .unwrap(),
                            ));

                            session_running_context = Some(SessionRunningContext {
                                lobby_swapchains,
                                reference_space,
                                views_history_sender,
                                views_history_receiver,
                                stream_context: None,
                            });
                        }
                        xr::SessionState::STOPPING => {
                            alvr_client_core::pause();
                            alvr_client_core::opengl::pause();

                            // Delete all resources and stop thread
                            session_running_context = None;

                            xr_session.end().unwrap();
                        }
                        xr::SessionState::EXITING => break 'render_loop,
                        xr::SessionState::LOSS_PENDING => break 'render_loop,
                        _ => (),
                    },
                    xr::Event::ReferenceSpaceChangePending(event) => {
                        info!(
                            "ReferenceSpaceChangePending type: {:?}",
                            event.reference_space_type()
                        );

                        if let Some(ctx) = &session_running_context {
                            *ctx.reference_space.write() = xr_session
                                .create_reference_space(
                                    xr::ReferenceSpaceType::STAGE,
                                    xr::Posef::IDENTITY,
                                )
                                .unwrap();

                            alvr_client_core::send_playspace(
                                xr_session
                                    .reference_space_bounds_rect(xr::ReferenceSpaceType::STAGE)
                                    .unwrap()
                                    .map(|a| Vec2::new(a.width, a.height)),
                            );
                        }
                    }
                    xr::Event::PerfSettingsEXT(event) => {
                        info!(
                            "Perf: from {:?} to {:?}, domain: {:?}/{:?}",
                            event.from_level(),
                            event.to_level(),
                            event.domain(),
                            event.sub_domain(),
                        );
                    }
                    xr::Event::InteractionProfileChanged(_) => {
                        // todo
                    }
                    xr::Event::PassthroughStateChangedFB(_) => {
                        // todo
                    }
                    _ => (),
                }
            }

            let session_context = if let Some(ctx) = &mut session_running_context {
                ctx
            } else {
                thread::sleep(Duration::from_millis(100));
                continue;
            };

            while let Some(event) = alvr_client_core::poll_event() {
                match event {
                    ClientCoreEvent::UpdateHudMessage(message) => {
                        last_lobby_message = message.clone();
                        alvr_client_core::opengl::update_hud_message(&message);
                    }
                    ClientCoreEvent::StreamingStarted {
                        settings,
                        negotiated_config,
                    } => {
                        let new_config = StreamConfig {
                            view_resolution: negotiated_config.view_resolution,
                            refresh_rate_hint: negotiated_config.refresh_rate_hint,
                            foveated_encoding_config: negotiated_config
                                .enable_foveated_encoding
                                .then(|| settings.video.foveated_encoding.as_option().cloned())
                                .flatten(),
                            clientside_foveation_config: settings
                                .video
                                .clientside_foveation
                                .as_option()
                                .cloned(),
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
                        };

                        // combined_eye_gaze is a setting that needs to be enabled at session
                        // creation. Since HTC headsets don't support session reinitialization, skip
                        // all elements that need it, that is face and eye tracking.
                        if stream_config.as_ref() != Some(&new_config)
                            && !matches!(
                                platform,
                                Platform::Focus3 | Platform::XRElite | Platform::ViveUnknown
                            )
                        {
                            stream_config = Some(new_config);

                            xr_session.request_exit().ok();
                            continue;
                        }

                        session_context.stream_context = Some(initialize_stream(
                            &xr_ctx,
                            Arc::clone(&interaction_context),
                            session_context,
                            platform,
                            &new_config,
                        ));

                        stream_config = Some(new_config);
                    }
                    ClientCoreEvent::StreamingStopped => {
                        session_context.stream_context = None;
                    }
                    ClientCoreEvent::Haptics {
                        device_id,
                        duration,
                        frequency,
                        amplitude,
                    } => {
                        let action = if device_id == *HAND_LEFT_ID {
                            &interaction_context.hands_interaction[0].vibration_action
                        } else {
                            &interaction_context.hands_interaction[1].vibration_action
                        };

                        action
                            .apply_feedback(
                                &xr_session,
                                xr::Path::NULL,
                                &xr::HapticVibration::new()
                                    .amplitude(amplitude)
                                    .frequency(frequency)
                                    .duration(xr::Duration::from_nanos(duration.as_nanos() as _)),
                            )
                            .unwrap();
                    }
                    _ => panic!(),
                }
            }

            let frame_state = match xr_frame_waiter.wait() {
                Ok(state) => state,
                Err(e) => {
                    error!("{e}");
                    panic!();
                }
            };
            let frame_interval =
                Duration::from_nanos(frame_state.predicted_display_period.as_nanos() as _);
            let vsync_time =
                Duration::from_nanos(frame_state.predicted_display_time.as_nanos() as _);

            xr_frame_stream.begin().unwrap();

            if !frame_state.should_render {
                xr_frame_stream
                    .end(
                        frame_state.predicted_display_time,
                        xr::EnvironmentBlendMode::OPAQUE,
                        &[],
                    )
                    .unwrap();

                continue;
            }

            let mut views;
            let display_time;
            let view_resolution;
            let swapchains;
            if let Some(context) = &mut session_context.stream_context {
                let frame_poll_deadline = Instant::now()
                    + Duration::from_secs_f32(
                        frame_interval.as_secs_f32() * DECODER_MAX_TIMEOUT_MULTIPLIER,
                    );
                let mut frame_result = None;
                while frame_result.is_none() && Instant::now() < frame_poll_deadline {
                    frame_result = alvr_client_core::get_frame();
                    thread::yield_now();
                }

                let (timestamp, hardware_buffer) = if let Some(pair) = frame_result {
                    pair
                } else {
                    warn!("Timed out when waiting for frame!");
                    (vsync_time, ptr::null_mut())
                };

                while let Ok(views) = session_context.views_history_receiver.try_recv() {
                    if context.views_history.len() > 360 {
                        context.views_history.pop_front();
                    }

                    context.views_history.push_back(views);
                }

                views = context.last_good_views.clone();

                for history_frame in &context.views_history {
                    if history_frame.timestamp == timestamp {
                        views = history_frame.views.clone();
                    }
                }

                let left_swapchain_idx = context.swapchains[0].acquire_image().unwrap();
                let right_swapchain_idx = context.swapchains[1].acquire_image().unwrap();

                context.swapchains[0]
                    .wait_image(xr::Duration::INFINITE)
                    .unwrap();
                context.swapchains[1]
                    .wait_image(xr::Duration::INFINITE)
                    .unwrap();

                alvr_client_core::opengl::render_stream(
                    hardware_buffer,
                    [left_swapchain_idx, right_swapchain_idx],
                );

                context.swapchains[0].release_image().unwrap();
                context.swapchains[1].release_image().unwrap();

                if !hardware_buffer.is_null() {
                    if let Some(now) = xr_runtime_now(&xr_instance) {
                        alvr_client_core::report_submit(timestamp, vsync_time.saturating_sub(now));
                    }
                }

                display_time = timestamp;
                view_resolution = context.view_resolution;
                swapchains = &context.swapchains;
            } else {
                let (flags, maybe_views) = xr_session
                    .locate_views(
                        xr::ViewConfigurationType::PRIMARY_STEREO,
                        frame_state.predicted_display_time,
                        &session_context.reference_space.read(),
                    )
                    .unwrap();

                views = if flags.contains(xr::ViewStateFlags::ORIENTATION_VALID) {
                    maybe_views
                } else {
                    vec![default_view(), default_view()]
                };

                let left_swapchain_idx =
                    session_context.lobby_swapchains[0].acquire_image().unwrap();
                let right_swapchain_idx =
                    session_context.lobby_swapchains[1].acquire_image().unwrap();

                session_context.lobby_swapchains[0]
                    .wait_image(xr::Duration::INFINITE)
                    .unwrap();
                session_context.lobby_swapchains[1]
                    .wait_image(xr::Duration::INFINITE)
                    .unwrap();

                alvr_client_core::opengl::render_lobby([
                    RenderViewInput {
                        pose: to_pose(views[0].pose),
                        fov: to_fov(views[0].fov),
                        swapchain_index: left_swapchain_idx,
                    },
                    RenderViewInput {
                        pose: to_pose(views[1].pose),
                        fov: to_fov(views[1].fov),
                        swapchain_index: right_swapchain_idx,
                    },
                ]);

                session_context.lobby_swapchains[0].release_image().unwrap();
                session_context.lobby_swapchains[1].release_image().unwrap();

                display_time = vsync_time;
                view_resolution = default_view_resolution;
                swapchains = &session_context.lobby_swapchains;
            }

            let rect = xr::Rect2Di {
                offset: xr::Offset2Di { x: 0, y: 0 },
                extent: xr::Extent2Di {
                    width: view_resolution.x as _,
                    height: view_resolution.y as _,
                },
            };

            let res = xr_frame_stream.end(
                to_xr_time(display_time),
                xr::EnvironmentBlendMode::OPAQUE,
                &[&xr::CompositionLayerProjection::new()
                    .space(&session_context.reference_space.read())
                    .views(&[
                        xr::CompositionLayerProjectionView::new()
                            .pose(views[0].pose)
                            .fov(views[0].fov)
                            .sub_image(
                                xr::SwapchainSubImage::new()
                                    .swapchain(&swapchains[0])
                                    .image_array_index(0)
                                    .image_rect(rect),
                            ),
                        xr::CompositionLayerProjectionView::new()
                            .pose(views[1].pose)
                            .fov(views[1].fov)
                            .sub_image(
                                xr::SwapchainSubImage::new()
                                    .swapchain(&swapchains[1])
                                    .image_array_index(0)
                                    .image_rect(rect),
                            ),
                    ])],
            );

            if let Err(e) = res {
                let time = to_xr_time(display_time);
                error!("End frame failed! {e}, timestamp: {display_time:?}, time: {time:?}");

                xr_frame_stream
                    .end(
                        frame_state.predicted_display_time,
                        xr::EnvironmentBlendMode::OPAQUE,
                        &[],
                    )
                    .unwrap();
            }

            if let Some(context) = &mut session_context.stream_context {
                context.last_good_views = views.clone();
            }
        }

        alvr_client_core::opengl::destroy();
    }

    alvr_client_core::destroy();
}

#[allow(unused)]
fn xr_runtime_now(xr_instance: &xr::Instance) -> Option<Duration> {
    let time_nanos = xr_instance.now().ok()?.as_nanos();

    (time_nanos > 0).then(|| Duration::from_nanos(time_nanos as _))
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    use android_activity::{InputStatus, MainEvent, PollEvent};

    let rendering_thread = thread::spawn(|| {
        // workaround for the Pico runtime
        let context = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) }.unwrap();
        let _env = vm.attach_current_thread().unwrap();

        entry_point();
    });

    let mut should_quit = false;
    while !should_quit {
        app.poll_events(Some(Duration::from_millis(100)), |event| match event {
            PollEvent::Main(MainEvent::Destroy) => {
                should_quit = true;
            }
            PollEvent::Main(MainEvent::InputAvailable) => {
                if let Ok(mut iter) = app.input_events_iter() {
                    while iter.next(|_| InputStatus::Unhandled) {}
                }
            }
            _ => (),
        });
    }

    // Note: the quit event is sent from OpenXR too, this will return rather quicly.
    rendering_thread.join().unwrap();
}
