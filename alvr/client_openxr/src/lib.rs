mod interaction;

use alvr_client_core::{opengl::RenderViewInput, ClientCoreEvent};
use alvr_common::{
    glam::{Quat, UVec2, Vec2, Vec3},
    prelude::*,
    settings_schema::Switch,
    DeviceMotion, Fov, Pose, RelaxedAtomic, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_packets::{FaceData, Tracking};
use alvr_session::ClientsideFoveationMode;
use interaction::{FaceInputContext, HandsInteractionContext};
use khronos_egl::{self as egl, EGL1_4};
use openxr as xr;
use std::{
    collections::VecDeque,
    path::Path,
    ptr,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

const IPD_CHANGE_EPS: f32 = 0.001;
const DECODER_MAX_TIMEOUT_MULTIPLIER: f32 = 0.8;

// Platform of the device. It is used to match the VR runtime and enable features conditionally.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Platform {
    Quest,
    Pico,
    Vive,
    Yvr,
    Other,
}

struct HistoryView {
    timestamp: Duration,
    views: Vec<xr::View>,
}

struct StreamingInputContext {
    platform: Platform,
    xr_instance: xr::Instance,
    xr_session: xr::Session<xr::AnyGraphics>,
    hands_context: Arc<HandsInteractionContext>,
    face_context: Option<FaceInputContext>,
    history_view_sender: mpsc::Sender<HistoryView>,
    reference_space: Arc<xr::Space>,
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

fn to_xr_time(timestamp: Duration) -> xr::Time {
    xr::Time::from_nanos(timestamp.as_nanos() as _)
}

#[allow(unused_variables)]
fn init_egl() -> EglContext {
    let instance = unsafe { egl::DynamicInstance::<EGL1_4>::load_required().unwrap() };

    let display = instance.get_display(egl::DEFAULT_DISPLAY).unwrap();

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

// This function is allowed to return errors. It can happen when the session is destroyed
// asynchronously
fn update_streaming_input(ctx: &mut StreamingInputContext) {
    // Streaming related inputs are updated here. Make sure every input poll is done in this
    // thread
    if let Err(e) = ctx
        .xr_session
        .sync_actions(&[(&ctx.hands_context.action_set).into()])
    {
        error!("{e}");
        return;
    }

    let Some(now) = xr_runtime_now(&ctx.xr_instance) else {
        error!("Cannot poll tracking: invalid time");
        return;
    };

    let target_timestamp = now + alvr_client_core::get_head_prediction_offset();

    let mut device_motions = Vec::with_capacity(3);

    'head_tracking: {
        let Ok((view_flags, views)) = ctx
            .xr_session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                to_xr_time(target_timestamp),
                &ctx.reference_space,
            )
        else {
            error!("Cannot locate views");
            break 'head_tracking;
        };

        if !view_flags.contains(xr::ViewStateFlags::POSITION_VALID)
            || !view_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
        {
            break 'head_tracking;
        }

        let ipd = (to_vec3(views[0].pose.position) - to_vec3(views[1].pose.position)).length();
        if f32::abs(ctx.last_ipd - ipd) > IPD_CHANGE_EPS {
            alvr_client_core::send_views_config([to_fov(views[0].fov), to_fov(views[1].fov)], ipd);

            ctx.last_ipd = ipd;
        }

        // Note: Here is assumed that views are on the same plane and orientation. The head position
        // is approximated as the center point between the eyes.
        let head_position =
            (to_vec3(views[0].pose.position) + to_vec3(views[1].pose.position)) / 2.0;
        let head_orientation = to_quat(views[0].pose.orientation);

        ctx.history_view_sender
            .send(HistoryView {
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

    let tracker_time = to_xr_time(now + alvr_client_core::get_tracker_prediction_offset());

    let (left_hand_motion, left_hand_skeleton) = interaction::get_hand_motion(
        &ctx.xr_session,
        &ctx.reference_space,
        tracker_time,
        &ctx.hands_context.hand_sources[0],
        &mut ctx.last_hand_positions[0],
    );
    let (right_hand_motion, right_hand_skeleton) = interaction::get_hand_motion(
        &ctx.xr_session,
        &ctx.reference_space,
        tracker_time,
        &ctx.hands_context.hand_sources[1],
        &mut ctx.last_hand_positions[1],
    );

    if let Some(motion) = left_hand_motion {
        device_motions.push((*LEFT_HAND_ID, motion));
    }
    if let Some(motion) = right_hand_motion {
        device_motions.push((*RIGHT_HAND_ID, motion));
    }

    let face_data = if let Some(context) = &ctx.face_context {
        FaceData {
            eye_gazes: interaction::get_eye_gazes(context, &ctx.reference_space, to_xr_time(now)),
            fb_face_expression: interaction::get_fb_face_expression(context, to_xr_time(now)),
            htc_eye_expression: interaction::get_htc_eye_expression(context),
            htc_lip_expression: interaction::get_htc_lip_expression(context),
        }
    } else {
        Default::default()
    };

    alvr_client_core::send_tracking(Tracking {
        target_timestamp,
        device_motions,
        hand_skeletons: [left_hand_skeleton, right_hand_skeleton],
        face_data,
    });

    let button_entries = interaction::update_buttons(
        ctx.platform,
        &ctx.xr_session,
        &ctx.hands_context.button_actions,
    );
    if !button_entries.is_empty() {
        alvr_client_core::send_buttons(button_entries);
    }
}

pub fn entry_point() {
    alvr_client_core::init_logging();

    let platform = match alvr_client_core::manufacturer_name().as_str() {
        "Oculus" => Platform::Quest,
        "Pico" => Platform::Pico,
        "HTC" => Platform::Vive,
        "YVR" => Platform::Yvr,
        _ => Platform::Other,
    };

    let xr_entry = match platform {
        Platform::Quest => unsafe {
            xr::Entry::load_from(Path::new("libopenxr_loader_quest.so")).unwrap()
        },
        Platform::Pico => unsafe {
            xr::Entry::load_from(Path::new("libopenxr_loader_pico.so")).unwrap()
        },
        Platform::Yvr => unsafe {
            xr::Entry::load_from(Path::new("libopenxr_loader_yvr.so")).unwrap()
        },
        _ => unsafe { xr::Entry::load().unwrap() },
    };

    #[cfg(target_os = "android")]
    xr_entry.initialize_android_loader().unwrap();

    let available_extensions = xr_entry.enumerate_extensions().unwrap();

    // todo: switch to vulkan
    assert!(available_extensions.khr_opengl_es_enable);

    let mut exts = xr::ExtensionSet::default();
    exts.ext_hand_tracking = available_extensions.ext_hand_tracking;
    exts.fb_color_space = available_extensions.fb_color_space;
    exts.fb_display_refresh_rate = available_extensions.fb_display_refresh_rate;
    exts.fb_eye_tracking_social = available_extensions.fb_eye_tracking_social;
    exts.fb_face_tracking = available_extensions.fb_face_tracking;
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

        let views_config = xr_instance
            .enumerate_view_configuration_views(
                xr_system,
                xr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .unwrap();
        assert_eq!(views_config.len(), 2);

        let recommended_view_resolution = UVec2::new(
            views_config[0].recommended_image_rect_width,
            views_config[0].recommended_image_rect_height,
        );

        let supported_refresh_rates = if exts.fb_display_refresh_rate {
            xr_session.enumerate_display_refresh_rates().unwrap()
        } else {
            vec![90.0]
        };

        alvr_client_core::initialize(recommended_view_resolution, supported_refresh_rates, false);
        alvr_client_core::opengl::initialize();

        let hands_context = Arc::new(interaction::initialize_hands_interaction(
            platform,
            &xr_instance,
            xr_system,
            &xr_session.clone().into_any_graphics(),
        ));

        let is_streaming = Arc::new(RelaxedAtomic::new(false));

        let mut reference_space = Arc::new(
            xr_session
                .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
                .unwrap(),
        );
        let mut lobby_swapchains = None;
        let mut stream_swapchains = None;
        let mut stream_view_resolution = UVec2::ZERO;
        let mut streaming_input_thread = None::<thread::JoinHandle<_>>;
        let mut views_history = VecDeque::new();

        let (history_view_sender, history_view_receiver) = mpsc::channel();
        let mut reference_space_sender = None::<mpsc::Sender<_>>;

        let default_view = xr::View {
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
                angle_left: -0.1,
                angle_right: 0.1,
                angle_up: 0.1,
                angle_down: -0.1,
            },
        };

        let mut last_good_views = vec![default_view, default_view];

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

                            let swapchains = lobby_swapchains.get_or_insert_with(|| {
                                [
                                    create_swapchain(
                                        &xr_session,
                                        recommended_view_resolution,
                                        None,
                                    ),
                                    create_swapchain(
                                        &xr_session,
                                        recommended_view_resolution,
                                        None,
                                    ),
                                ]
                            });

                            alvr_client_core::opengl::resume(
                                recommended_view_resolution,
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
                            );

                            alvr_client_core::resume();
                        }
                        xr::SessionState::STOPPING => {
                            // Make sure streaming resources are destroyed before pausing
                            {
                                stream_swapchains.take();

                                is_streaming.set(false);

                                if let Some(thread) = streaming_input_thread.take() {
                                    thread.join().unwrap();
                                }
                            }

                            alvr_client_core::pause();

                            alvr_client_core::opengl::pause();

                            lobby_swapchains.take();

                            xr_session.end().unwrap();
                        }
                        xr::SessionState::EXITING => {
                            break 'session_loop;
                        }
                        xr::SessionState::LOSS_PENDING => {
                            break 'render_loop;
                        }
                        _ => (),
                    },
                    xr::Event::ReferenceSpaceChangePending(event) => {
                        info!(
                            "ReferenceSpaceChangePending type: {:?}",
                            event.reference_space_type()
                        );

                        reference_space = Arc::new(
                            xr_session
                                .create_reference_space(
                                    xr::ReferenceSpaceType::STAGE,
                                    xr::Posef::IDENTITY,
                                )
                                .unwrap(),
                        );

                        if let Some(sender) = &reference_space_sender {
                            sender.send(Arc::clone(&reference_space)).ok();
                        }

                        alvr_client_core::send_playspace(
                            xr_session
                                .reference_space_bounds_rect(xr::ReferenceSpaceType::STAGE)
                                .unwrap()
                                .map(|a| Vec2::new(a.width, a.height)),
                        );
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
                    // not used:
                    // VisibilityMaskChangedKHR
                    // MainSessionVisibilityChangedEXTX
                    // DisplayRefreshRateChangedFB
                    // SpatialAnchorCreateCompleteFB
                    // SpaceSetStatusCompleteFB
                    // SpaceQueryResultsAvailableFB
                    // SpaceQueryCompleteFB
                    // SpaceSaveCompleteFB
                    // SpaceEraseCompleteFB
                    // ViveTrackerConnectedHTCX
                    // MarkerTrackingUpdateVARJO
                }
            }

            let lobby_swapchains = if let Some(swapchains) = &mut lobby_swapchains {
                swapchains
            } else {
                thread::sleep(Duration::from_millis(100));
                continue;
            };

            while let Some(event) = alvr_client_core::poll_event() {
                match event {
                    ClientCoreEvent::UpdateHudMessage(message) => {
                        alvr_client_core::opengl::update_hud_message(&message);
                    }
                    ClientCoreEvent::StreamingStarted {
                        view_resolution,
                        refresh_rate_hint,
                        settings,
                    } => {
                        stream_view_resolution = view_resolution;

                        if exts.fb_display_refresh_rate {
                            xr_session
                                .request_display_refresh_rate(refresh_rate_hint)
                                .unwrap();
                        }

                        is_streaming.set(true);

                        let face_context =
                            if let Switch::Enabled(config) = settings.headset.face_tracking {
                                // todo: check which permissions are needed for htc
                                #[cfg(target_os = "android")]
                                {
                                    if config.sources.eye_tracking_fb {
                                        alvr_client_core::try_get_permission(
                                            "com.oculus.permission.EYE_TRACKING",
                                        );
                                    }
                                    if config.sources.face_tracking_fb {
                                        alvr_client_core::try_get_permission(
                                            "com.oculus.permission.FACE_TRACKING",
                                        );
                                    }
                                }

                                Some(interaction::initialize_face_input(
                                    &xr_instance,
                                    xr_system,
                                    &xr_session,
                                    config.sources.eye_tracking_fb,
                                    config.sources.face_tracking_fb,
                                    config.sources.eye_expressions_htc,
                                    config.sources.lip_expressions_htc,
                                ))
                            } else {
                                None
                            };

                        let mut context = StreamingInputContext {
                            platform,
                            xr_instance: xr_instance.clone(),
                            xr_session: xr_session.clone().into_any_graphics(),
                            hands_context: Arc::clone(&hands_context),
                            face_context,
                            history_view_sender: history_view_sender.clone(),
                            reference_space: Arc::clone(&reference_space),
                            last_ipd: 0.0,
                            last_hand_positions: [Vec3::ZERO; 2],
                        };

                        let is_streaming = Arc::clone(&is_streaming);

                        let (sender, reference_space_receiver) = mpsc::channel();
                        reference_space_sender = Some(sender);

                        streaming_input_thread = Some(thread::spawn(move || {
                            let mut deadline = Instant::now();
                            let frame_interval = Duration::from_secs_f32(1.0 / refresh_rate_hint);

                            while is_streaming.value() {
                                update_streaming_input(&mut context);

                                if let Ok(reference_space) = reference_space_receiver.try_recv() {
                                    context.reference_space = reference_space;
                                }

                                deadline += frame_interval / 3;
                                thread::sleep(deadline.saturating_duration_since(Instant::now()));
                            }
                        }));

                        let foveation_profile = if let Some(config) =
                            settings.video.clientside_foveation.into_option()
                        {
                            if exts.fb_swapchain_update_state
                                && exts.fb_foveation
                                && exts.fb_foveation_configuration
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

                        let swapchains = stream_swapchains.get_or_insert_with(|| {
                            [
                                create_swapchain(
                                    &xr_session,
                                    stream_view_resolution,
                                    foveation_profile.as_ref(),
                                ),
                                create_swapchain(
                                    &xr_session,
                                    stream_view_resolution,
                                    foveation_profile.as_ref(),
                                ),
                            ]
                        });

                        alvr_client_core::opengl::start_stream(
                            view_resolution,
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
                            settings.video.foveated_rendering.into_option(),
                        );

                        alvr_client_core::send_playspace(
                            xr_session
                                .reference_space_bounds_rect(xr::ReferenceSpaceType::STAGE)
                                .unwrap()
                                .map(|a| Vec2::new(a.width, a.height)),
                        );
                    }
                    ClientCoreEvent::StreamingStopped => {
                        stream_swapchains.take();

                        is_streaming.set(false);

                        if let Some(thread) = streaming_input_thread.take() {
                            thread.join().unwrap();
                        }
                    }
                    ClientCoreEvent::Haptics {
                        device_id,
                        duration,
                        frequency,
                        amplitude,
                    } => {
                        let action = if device_id == *LEFT_HAND_ID {
                            &hands_context.hand_sources[0].vibration_action
                        } else {
                            &hands_context.hand_sources[1].vibration_action
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

            let swapchains = if let Some(swapchains) = &mut stream_swapchains {
                swapchains
            } else {
                lobby_swapchains
            };

            let left_swapchain_idx = swapchains[0].acquire_image().unwrap();
            let right_swapchain_idx = swapchains[1].acquire_image().unwrap();

            swapchains[0].wait_image(xr::Duration::INFINITE).unwrap();
            swapchains[1].wait_image(xr::Duration::INFINITE).unwrap();

            let mut views = last_good_views.clone();

            let display_time;
            let view_resolution;
            if is_streaming.value() {
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

                while let Ok(views) = history_view_receiver.try_recv() {
                    if views_history.len() > 360 {
                        views_history.pop_front();
                    }

                    views_history.push_back(views);
                }

                for history_frame in &views_history {
                    if history_frame.timestamp == timestamp {
                        views = history_frame.views.clone();
                    }
                }

                alvr_client_core::opengl::render_stream(
                    hardware_buffer,
                    [left_swapchain_idx, right_swapchain_idx],
                );

                if !hardware_buffer.is_null() {
                    if let Some(now) = xr_runtime_now(&xr_instance) {
                        alvr_client_core::report_submit(timestamp, vsync_time.saturating_sub(now));
                    }
                }

                display_time = timestamp;

                view_resolution = stream_view_resolution;
            } else {
                display_time = vsync_time;

                let (flags, maybe_views) = xr_session
                    .locate_views(
                        xr::ViewConfigurationType::PRIMARY_STEREO,
                        frame_state.predicted_display_time,
                        &reference_space,
                    )
                    .unwrap();

                if flags.contains(xr::ViewStateFlags::ORIENTATION_VALID) {
                    views = maybe_views;
                }

                view_resolution = recommended_view_resolution;

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
            }

            swapchains[0].release_image().unwrap();
            swapchains[1].release_image().unwrap();

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
                    .space(&reference_space)
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

            last_good_views = views.clone();
        }
    }

    alvr_client_core::opengl::destroy();

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
                app.input_events(|_| InputStatus::Unhandled);
            }
            _ => (),
        });
    }

    // Note: the quit event is sent from OpenXR too, this will return rather quicly.
    rendering_thread.join().unwrap();
}
