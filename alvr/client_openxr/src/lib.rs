mod interaction;

use alvr_client_core::ClientCoreEvent;
use alvr_client_opengl::RenderViewInput;
use alvr_common::{
    glam::{Quat, UVec2, Vec3},
    parking_lot::Mutex,
    prelude::*,
    Fov, RelaxedAtomic, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_sockets::{DeviceMotion, Tracking};
use interaction::StreamingInteractionContext;
use khronos_egl::{self as egl, EGL1_4};
use openxr as xr;
use std::{
    collections::VecDeque,
    path::Path,
    ptr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

const IPD_CHANGE_EPS: f32 = 0.001;

struct StreamingInputContext {
    is_streaming: Arc<RelaxedAtomic>,
    frame_interval: Duration,
    xr_instance: xr::Instance,
    xr_session: xr::Session<xr::AnyGraphics>,
    interaction_context: Arc<StreamingInteractionContext>,
    reference_space: Arc<xr::Space>,
    views_history: Arc<Mutex<VecDeque<(Duration, Vec<xr::View>)>>>,
}

struct EglContext {
    instance: egl::DynamicInstance<EGL1_4>,
    context: egl::Context,
    dummy_surface: egl::Surface,
}

fn to_vec3(v: xr::Vector3f) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn to_quat(q: xr::Quaternionf) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}

fn to_fov(f: xr::Fovf) -> Fov {
    Fov {
        left: f.angle_left,
        right: f.angle_right,
        up: f.angle_up,
        down: f.angle_down,
    }
}

fn to_duration(time: xr::Time) -> Duration {
    Duration::from_nanos(time.as_nanos() as _)
}

fn to_xr_time(timestamp: Duration) -> xr::Time {
    xr::Time::from_nanos(timestamp.as_nanos() as _)
}

#[allow(unused_variables)]
fn init_xr_session_egl(
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
) -> (
    EglContext,
    xr::Session<xr::OpenGlEs>,
    xr::FrameWaiter,
    xr::FrameStream<xr::OpenGlEs>,
) {
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

    #[cfg(target_os = "android")]
    {
        let (xr_session, xr_frame_waiter, xr_frame_stream) = unsafe {
            xr_instance.create_session(
                xr_system,
                &xr::opengles::SessionCreateInfo::Android {
                    display: display.as_ptr(),
                    config: config.as_ptr(),
                    context: context.as_ptr(),
                },
            )
        }
        .unwrap();

        (
            EglContext {
                instance,
                context,
                dummy_surface,
            },
            xr_session,
            xr_frame_waiter,
            xr_frame_stream,
        )
    }
    #[cfg(not(target_os = "android"))]
    unimplemented!()
}

pub fn create_swapchain(
    session: &xr::Session<xr::OpenGlEs>,
    resolution: UVec2,
) -> xr::Swapchain<xr::OpenGlEs> {
    session
        .create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                | xr::SwapchainUsageFlags::SAMPLED,
            format: glow::SRGB8_ALPHA8,
            sample_count: 1,
            width: resolution.x,
            height: resolution.y,
            face_count: 1,
            array_size: 1,
            mip_count: 1,
        })
        .unwrap()
}

fn streaming_input_loop(ctx: StreamingInputContext) {
    let mut deadline = Instant::now();

    let mut last_ipd = 0.0;

    while ctx.is_streaming.value() {
        // Streaming related inputs are updated here. Make sure every input poll is done in this
        // thread
        ctx.xr_session
            .sync_actions(&[(&ctx.interaction_context.action_set).into()])
            .unwrap();

        let now = to_duration(ctx.xr_instance.now().unwrap());

        let target_timestamp = now + alvr_client_core::get_head_prediction_offset();

        let (_, views) = ctx
            .xr_session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                to_xr_time(target_timestamp),
                &ctx.reference_space,
            )
            .unwrap();

        let ipd = (to_vec3(views[0].pose.position) - to_vec3(views[1].pose.position)).length();
        if f32::abs(last_ipd - ipd) > IPD_CHANGE_EPS {
            alvr_client_core::send_views_config([to_fov(views[0].fov), to_fov(views[1].fov)], ipd);

            last_ipd = ipd;
        }

        // Note: Here is assumed that views are on the same plane and orientation. The head position
        // is approximated as the center point between the eyes.
        let head_position =
            (to_vec3(views[0].pose.position) + to_vec3(views[1].pose.position)) / 2.0;
        let head_orientation = to_quat(views[0].pose.orientation);

        {
            let mut views_history_lock = ctx.views_history.lock();

            views_history_lock.push_back((target_timestamp, views));
            if views_history_lock.len() > 360 {
                views_history_lock.pop_front();
            }
        }

        let tracker_time = to_xr_time(now + alvr_client_core::get_tracker_prediction_offset());

        let mut device_motions = vec![(
            *HEAD_ID,
            DeviceMotion {
                orientation: head_orientation,
                position: head_position,
                linear_velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
            },
        )];

        let (left_hand_motion, left_hand_skeleton) = interaction::get_hand_motion(
            &ctx.xr_session,
            &ctx.reference_space,
            tracker_time,
            &ctx.interaction_context.left_hand_source,
        );
        let (right_hand_motion, right_hand_skeleton) = interaction::get_hand_motion(
            &ctx.xr_session,
            &ctx.reference_space,
            tracker_time,
            &ctx.interaction_context.right_hand_source,
        );

        if let Some(motion) = left_hand_motion {
            device_motions.push((*LEFT_HAND_ID, motion));
        }
        if let Some(motion) = right_hand_motion {
            device_motions.push((*RIGHT_HAND_ID, motion));
        }

        alvr_client_core::send_tracking(Tracking {
            target_timestamp,
            device_motions,
            left_hand_skeleton,
            right_hand_skeleton,
        });

        interaction::update_buttons(&ctx.xr_session, &ctx.interaction_context.button_actions);

        deadline += ctx.frame_interval / 3;
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
}

pub fn entry_point() {
    alvr_client_core::init_logging();

    let device_name = alvr_client_core::get_device_name();

    let xr_entry = if device_name == "Quest" {
        unsafe { xr::Entry::load_from(Path::new("libopenxr_loader_quest.so")).unwrap() }
    } else if device_name.contains("Pico") || device_name == "A8150" {
        unsafe { xr::Entry::load_from(Path::new("libopenxr_loader_pico.so")).unwrap() }
    } else {
        unsafe { xr::Entry::load().unwrap() }
    };

    #[cfg(target_os = "android")]
    xr_entry.initialize_android_loader().unwrap();

    let available_extensions = xr_entry.enumerate_extensions().unwrap();

    // todo: switch to vulkan
    assert!(available_extensions.khr_opengl_es_enable);

    let mut enabled_extensions = xr::ExtensionSet::default();
    enabled_extensions.khr_opengl_es_enable = true;
    enabled_extensions.khr_convert_timespec_time = true;
    enabled_extensions.ext_hand_tracking = available_extensions.ext_hand_tracking;
    enabled_extensions.fb_display_refresh_rate = available_extensions.fb_display_refresh_rate;
    #[cfg(target_os = "android")]
    {
        enabled_extensions.khr_android_create_instance = true;
    }

    let xr_instance = xr_entry
        .create_instance(
            &xr::ApplicationInfo {
                application_name: "ALVR Client",
                application_version: 0,
                engine_name: "ALVR",
                engine_version: 0,
            },
            &enabled_extensions,
            &[],
        )
        .unwrap();

    let xr_system = xr_instance
        .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
        .unwrap();

    // mandatory call
    let _ = xr_instance
        .graphics_requirements::<xr::OpenGlEs>(xr_system)
        .unwrap();

    let (_egl_context, xr_session, mut xr_frame_waiter, mut xr_frame_stream) =
        init_xr_session_egl(&xr_instance, xr_system);

    let views = xr_instance
        .enumerate_view_configuration_views(xr_system, xr::ViewConfigurationType::PRIMARY_STEREO)
        .unwrap();
    assert_eq!(views.len(), 2);

    let recommended_resolution = UVec2::new(
        views[0].recommended_image_rect_width,
        views[0].recommended_image_rect_height,
    );

    alvr_client_core::initialize(recommended_resolution, vec![72.0, 90.0], false);
    alvr_client_opengl::initialize();

    let streaming_interaction_context = Arc::new(interaction::initialize_streaming_interaction(
        &xr_instance,
        xr_system,
        &xr_session.clone().into_any_graphics(),
    ));

    let reference_space = Arc::new(
        xr_session
            .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
            .unwrap(),
    );

    let is_streaming = Arc::new(RelaxedAtomic::new(false));

    let mut lobby_swapchains = None;
    let mut stream_swapchains = None;
    let mut stream_view_resolution = UVec2::ZERO;
    let mut streaming_input_thread = None;
    let views_history = Arc::new(Mutex::new(VecDeque::new()));

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
    'main_loop: loop {
        while let Some(event) = xr_instance.poll_event(&mut event_storage).unwrap() {
            match event {
                xr::Event::EventsLost(e) => {
                    error!("OpenXR: lost {} events!", e.lost_event_count());
                }
                xr::Event::InstanceLossPending(_) => break 'main_loop,
                xr::Event::SessionStateChanged(e) => match e.state() {
                    xr::SessionState::READY => {
                        xr_session
                            .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                            .unwrap();

                        let swapchains = lobby_swapchains.get_or_insert_with(|| {
                            [
                                create_swapchain(&xr_session, recommended_resolution),
                                create_swapchain(&xr_session, recommended_resolution),
                            ]
                        });

                        alvr_client_opengl::resume(
                            recommended_resolution,
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
                        alvr_client_core::pause();

                        alvr_client_opengl::pause();

                        lobby_swapchains.take();

                        xr_session.end().unwrap();
                    }
                    xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                        break 'main_loop;
                    }
                    _ => (),
                },
                xr::Event::ReferenceSpaceChangePending(_) => {
                    // e.
                }
                xr::Event::PerfSettingsEXT(e) => {
                    info!(
                        "Perf: from {:?} to {:?}, domain: {:?}/{:?}",
                        e.from_level(),
                        e.to_level(),
                        e.domain(),
                        e.sub_domain(),
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
                    alvr_client_opengl::update_hud_message(&message);
                }
                ClientCoreEvent::StreamingStarted {
                    view_resolution,
                    fps,
                    foveated_rendering,
                    oculus_foveation_level,
                    dynamic_oculus_foveation,
                    extra_latency,
                } => {
                    xr_session.request_display_refresh_rate(fps).unwrap();

                    stream_view_resolution = view_resolution;

                    is_streaming.set(true);

                    let context = StreamingInputContext {
                        is_streaming: Arc::clone(&is_streaming),
                        frame_interval: Duration::from_secs_f32(1.0 / fps),
                        xr_instance: xr_instance.clone(),
                        xr_session: xr_session.clone().into_any_graphics(),
                        interaction_context: Arc::clone(&streaming_interaction_context),
                        reference_space: Arc::clone(&reference_space),
                        views_history: Arc::clone(&views_history),
                    };

                    streaming_input_thread = Some(thread::spawn(|| {
                        streaming_input_loop(context);
                    }));

                    let swapchains = stream_swapchains.get_or_insert_with(|| {
                        [
                            create_swapchain(&xr_session, stream_view_resolution),
                            create_swapchain(&xr_session, stream_view_resolution),
                        ]
                    });

                    alvr_client_opengl::start_stream(
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
                        foveated_rendering,
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
                        &streaming_interaction_context
                            .left_hand_source
                            .vibration_action
                    } else {
                        &streaming_interaction_context
                            .right_hand_source
                            .vibration_action
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

        let display_time;
        let views;
        let view_resolution;
        let swapchains;
        if is_streaming.value() {
            let (timestamp, hardware_buffer) = if let Some(pair) = alvr_client_core::get_frame() {
                pair
            } else {
                (
                    to_duration(frame_state.predicted_display_time),
                    ptr::null_mut(),
                )
            };

            display_time = timestamp;

            {
                let mut history_views = None;

                for (history_timestamp, views) in &*views_history.lock() {
                    if *history_timestamp == timestamp {
                        history_views = Some(views.clone());
                    }
                }

                views = if let Some(views) = history_views {
                    last_good_views = views.clone();
                    views
                } else {
                    last_good_views.clone()
                };
            }

            view_resolution = stream_view_resolution;

            swapchains = stream_swapchains.as_mut().unwrap();

            let left_view_idx = swapchains[0].acquire_image().unwrap();
            let right_view_idx = swapchains[1].acquire_image().unwrap();

            swapchains[0].wait_image(xr::Duration::INFINITE).unwrap();
            swapchains[1].wait_image(xr::Duration::INFINITE).unwrap();

            alvr_client_opengl::render_stream(hardware_buffer, [left_view_idx, right_view_idx]);

            swapchains[0].release_image().unwrap();
            swapchains[1].release_image().unwrap();

            let vsync_queue = Duration::from_nanos(
                (frame_state.predicted_display_time - xr_instance.now().unwrap()).as_nanos() as _,
            );
            alvr_client_core::report_submit(timestamp, vsync_queue);
        } else {
            display_time = to_duration(frame_state.predicted_display_time);

            views = xr_session
                .locate_views(
                    xr::ViewConfigurationType::PRIMARY_STEREO,
                    frame_state.predicted_display_time,
                    &reference_space,
                )
                .unwrap()
                .1;

            view_resolution = recommended_resolution;

            swapchains = lobby_swapchains;

            let left_view_idx = swapchains[0].acquire_image().unwrap();
            let right_view_idx = swapchains[1].acquire_image().unwrap();

            swapchains[0].wait_image(xr::Duration::INFINITE).unwrap();
            swapchains[1].wait_image(xr::Duration::INFINITE).unwrap();

            alvr_client_opengl::render_lobby([
                RenderViewInput {
                    position: to_vec3(views[0].pose.position),
                    orientation: to_quat(views[0].pose.orientation),
                    fov: to_fov(views[0].fov),
                    swapchain_index: left_view_idx,
                },
                RenderViewInput {
                    position: to_vec3(views[1].pose.position),
                    orientation: to_quat(views[1].pose.orientation),
                    fov: to_fov(views[1].fov),
                    swapchain_index: right_view_idx,
                },
            ]);

            swapchains[0].release_image().unwrap();
            swapchains[1].release_image().unwrap();
        }

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: view_resolution.x as _,
                height: view_resolution.y as _,
            },
        };
        xr_frame_stream
            .end(
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
            )
            .unwrap();
    }

    alvr_client_opengl::destroy();

    alvr_client_core::destroy();

    drop((
        xr_session,
        xr_frame_waiter,
        xr_frame_stream,
        lobby_swapchains,
    ));
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    let rendering_thread = thread::spawn(|| {
        // workaround for the Pico runtime
        let context = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) }.unwrap();
        let _env = vm.attach_current_thread().unwrap();

        entry_point();
    });

    let mut should_quit = false;
    while !should_quit {
        app.poll_events(Some(Duration::from_millis(100)), |event| {
            if matches!(
                event,
                android_activity::PollEvent::Main(android_activity::MainEvent::Destroy)
            ) {
                should_quit = true;
            }
        });
    }

    // Note: the quit event is sent from OpenXR too, this will return rather quicly.
    rendering_thread.join().unwrap();
}
