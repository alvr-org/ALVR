use alvr_client_core::ClientCoreEvent;
use alvr_client_opengl::RenderViewInput;
use alvr_common::{
    glam::{Quat, UVec2, Vec3},
    prelude::*,
    Fov, LEFT_CONTROLLER_HAPTIC_ID, LEFT_HAND_ID, LEFT_HAND_PATH,
};
use khronos_egl::{self as egl, EGL1_4};
use openxr as xr;
use std::{ptr, thread, time::Duration};

enum ButtonAction {
    Binary(xr::Action<bool>),
    Scalar(xr::Action<f32>),
}

struct EglContext {
    entry: egl::DynamicInstance<EGL1_4>,
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

fn init_xr_session_egl(
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
) -> (
    EglContext,
    xr::Session<xr::OpenGL>,
    xr::FrameWaiter,
    xr::FrameStream<xr::OpenGL>,
) {
    let entry = unsafe { egl::DynamicInstance::<EGL1_4>::load_required().unwrap() };

    let display = entry.get_display(egl::DEFAULT_DISPLAY).unwrap();

    let version = entry.initialize(display).unwrap();
    dbg!(version);

    let mut configs = Vec::with_capacity(entry.get_config_count(display).unwrap());
    entry.get_configs(display, &mut configs).unwrap();

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
    let config = entry
        .choose_first_config(display, &CONFIG_ATTRIBS)
        .unwrap()
        .unwrap();

    entry.bind_api(egl::OPENGL_ES_API).unwrap();

    const CONTEXT_ATTRIBS: [i32; 3] = [egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE];
    let context = entry
        .create_context(display, config, None, &CONTEXT_ATTRIBS)
        .unwrap();

    const PBUFFER_ATTRIBS: [i32; 5] = [egl::WIDTH, 16, egl::HEIGHT, 16, egl::NONE];
    let dummy_surface = entry
        .create_pbuffer_surface(display, config, &PBUFFER_ATTRIBS)
        .unwrap();

    entry
        .make_current(
            display,
            Some(dummy_surface),
            Some(dummy_surface),
            Some(context),
        )
        .unwrap();

    #[cfg(target_os = "android")]
    let (xr_session, xr_frame_waiter, xr_frame_stream) = unsafe {
        xr_instance.create_session(
            xr_system,
            &xr::opengl::SessionCreateInfo::Android {
                display: display.as_ptr(),
                config: config.as_ptr(),
                context: context.as_ptr(),
            },
        )
    }
    .unwrap();
    // invalid initialization just to avoid lints
    #[cfg(not(target_os = "android"))]
    let (xr_session, xr_frame_waiter, xr_frame_stream) = unsafe {
        xr_instance.create_session(
            xr_system,
            &xr::opengl::SessionCreateInfo::Xlib {
                x_display: ptr::null_mut(),
                visualid: 0,
                glx_fb_config: ptr::null_mut(),
                glx_drawable: 0,
                glx_context: ptr::null_mut(),
            },
        )
    }
    .unwrap();

    (
        EglContext {
            entry,
            context,
            dummy_surface,
        },
        xr_session,
        xr_frame_waiter,
        xr_frame_stream,
    )
}

pub fn create_swapchain(
    session: &xr::Session<xr::OpenGL>,
    resolution: UVec2,
) -> xr::Swapchain<xr::OpenGL> {
    session
        .create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                | xr::SwapchainUsageFlags::SAMPLED,
            format: glow::RGBA8,
            sample_count: 1,
            width: resolution.x,
            height: resolution.y,
            face_count: 1,
            array_size: 1,
            mip_count: 1,
        })
        .unwrap()
}

pub fn events_thread() {}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn entry_point() {
    alvr_client_core::init_logging();

    let xr_entry = unsafe { xr::Entry::load().unwrap() };

    #[cfg(target_os = "android")]
    xr_entry.initialize_android_loader().unwrap();

    let available_extensions = xr_entry.enumerate_extensions().unwrap();

    // todo: switch to vulkan
    assert!(available_extensions.khr_opengl_es_enable);

    let mut enabled_extensions = xr::ExtensionSet::default();
    enabled_extensions.khr_opengl_es_enable = true;
    enabled_extensions.fb_display_refresh_rate = true;
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
        .graphics_requirements::<xr::OpenGL>(xr_system)
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
    #[cfg(target_os = "android")]
    alvr_client_opengl::initialize();

    let action_set = xr_instance
        .create_action_set("alvr_input", "ALVR input", 0)
        .unwrap();

    // let pose_actions = [
    //     (*LEFT_HAND_ID, LEFT_HAND_PATH, )
    // ]

    // xr_instance.suggest_interaction_profile_bindings(
    //     xr_instance
    //         .string_to_path("/interaction_profiles/khr/simple_controller")
    //         .unwrap(),
    //     &[],
    // );

    // xr_session.attach_action_sets(&[&action_set]).unwrap();

    let reference_space = xr_session
        .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
        .unwrap();

    let mut lobby_swapchains = None;
    // let mut stream_swapchain = None;

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

                        let textures: [Vec<i32>; 2] = [
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
                        ];

                        #[cfg(target_os = "android")]
                        alvr_client_opengl::resume(recommended_resolution, textures);

                        alvr_client_core::resume();
                    }
                    xr::SessionState::STOPPING => {
                        alvr_client_core::pause();

                        #[cfg(target_os = "android")]
                        alvr_client_opengl::pause();

                        lobby_swapchains.take();

                        xr_session.end().unwrap();
                    }
                    xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                        break 'main_loop;
                    }
                    _ => (),
                },
                xr::Event::ReferenceSpaceChangePending(e) => {
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
                    // todo
                }
                ClientCoreEvent::StreamingStopped => {
                    // todo
                }
                ClientCoreEvent::Haptics {
                    device_id,
                    duration,
                    frequency,
                    amplitude,
                } => {
                    // todo
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

        let left_view_idx = lobby_swapchains[0].acquire_image().unwrap();
        let right_view_idx = lobby_swapchains[1].acquire_image().unwrap();

        lobby_swapchains[0]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();
        lobby_swapchains[1]
            .wait_image(xr::Duration::INFINITE)
            .unwrap();

        let (_, views) = xr_session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                frame_state.predicted_display_time,
                &reference_space,
            )
            .unwrap();

        // let head_position =
        //     (to_vec3(views[0].pose.position) + to_vec3(views[0].pose.position)) / 2.0;

        let view_inputs = [
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
        ];

        #[cfg(target_os = "android")]
        alvr_client_opengl::render_lobby(view_inputs);

        lobby_swapchains[0].release_image().unwrap();
        lobby_swapchains[1].release_image().unwrap();

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: recommended_resolution.x as _,
                height: recommended_resolution.y as _,
            },
        };
        xr_frame_stream
            .end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[&xr::CompositionLayerProjection::new()
                    .space(&reference_space)
                    .views(&[
                        xr::CompositionLayerProjectionView::new()
                            .pose(views[0].pose)
                            .fov(views[0].fov)
                            .sub_image(
                                xr::SwapchainSubImage::new()
                                    .swapchain(&lobby_swapchains[0])
                                    .image_array_index(0)
                                    .image_rect(rect),
                            ),
                        xr::CompositionLayerProjectionView::new()
                            .pose(views[1].pose)
                            .fov(views[1].fov)
                            .sub_image(
                                xr::SwapchainSubImage::new()
                                    .swapchain(&lobby_swapchains[1])
                                    .image_array_index(0)
                                    .image_rect(rect),
                            ),
                    ])],
            )
            .unwrap();
    }

    #[cfg(target_os = "android")]
    alvr_client_opengl::destroy();

    alvr_client_core::destroy();

    drop((
        xr_session,
        xr_frame_waiter,
        xr_frame_stream,
        lobby_swapchains,
    ));
}
