mod c_api;
mod extra_extensions;
mod graphics;
mod interaction;
mod lobby;
mod passthrough;
mod stream;

use crate::stream::ParsedStreamConfig;
use alvr_client_core::{ClientCapabilities, ClientCoreContext, ClientCoreEvent};
use alvr_common::{
    Fov, HAND_LEFT_ID, Pose, debug, error,
    glam::{Quat, UVec2, Vec3},
    info,
    parking_lot::RwLock,
};
use alvr_graphics::GraphicsContext;
use alvr_session::{BodyTrackingBDConfig, BodyTrackingSourcesConfig};
use alvr_system_info::Platform;
use extra_extensions::{
    BD_BODY_TRACKING_EXTENSION_NAME, BD_MOTION_TRACKING_EXTENSION_NAME,
    META_BODY_TRACKING_FIDELITY_EXTENSION_NAME, META_BODY_TRACKING_FULL_BODY_EXTENSION_NAME,
    META_DETACHED_CONTROLLERS_EXTENSION_NAME,
    META_SIMULTANEOUS_HANDS_AND_CONTROLLERS_EXTENSION_NAME, PICO_CONFIGURATION_EXTENSION_NAME,
};
use interaction::{InteractionContext, InteractionSourcesConfig};
use lobby::Lobby;
use openxr as xr;
use passthrough::PassthroughLayer;
use std::{ffi::CStr, path::Path, rc::Rc, sync::Arc, thread, time::Duration};
use stream::StreamContext;

fn from_xr_vec3(v: xr::Vector3f) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn to_xr_vec3(v: Vec3) -> xr::Vector3f {
    xr::Vector3f {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

fn from_xr_quat(q: xr::Quaternionf) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}

fn to_xr_quat(q: Quat) -> xr::Quaternionf {
    xr::Quaternionf {
        x: q.x,
        y: q.y,
        z: q.z,
        w: q.w,
    }
}

fn from_xr_pose(p: xr::Posef) -> Pose {
    Pose {
        orientation: from_xr_quat(p.orientation),
        position: from_xr_vec3(p.position),
    }
}

fn to_xr_pose(p: Pose) -> xr::Posef {
    xr::Posef {
        orientation: to_xr_quat(p.orientation),
        position: to_xr_vec3(p.position),
    }
}

fn from_xr_fov(f: xr::Fovf) -> Fov {
    Fov {
        left: f.angle_left,
        right: f.angle_right,
        up: f.angle_up,
        down: f.angle_down,
    }
}

fn to_xr_fov(f: Fov) -> xr::Fovf {
    xr::Fovf {
        angle_left: f.left,
        angle_right: f.right,
        angle_up: f.up,
        angle_down: f.down,
    }
}

fn from_xr_time(timestamp: xr::Time) -> Duration {
    Duration::from_nanos(timestamp.as_nanos() as _)
}

fn to_xr_time(timestamp: Duration) -> xr::Time {
    xr::Time::from_nanos(timestamp.as_nanos() as _)
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

// This exists to circumvent dead-code analysis
fn create_session(
    xr_instance: &xr::Instance,
    xr_system: xr::SystemId,
    graphics_context: &GraphicsContext,
) -> (
    xr::Session<xr::OpenGlEs>,
    xr::FrameWaiter,
    xr::FrameStream<xr::OpenGlEs>,
) {
    unsafe {
        xr_instance
            .create_session(xr_system, &graphics::session_create_info(graphics_context))
            .unwrap()
    }
}

pub fn entry_point() {
    alvr_client_core::init_logging();

    const LEGACY_OPENXR_VERSION: xr::Version = xr::Version::new(1, 0, 34);
    const CURRENT_OPENXR_VERSION: xr::Version = xr::Version::new(1, 1, 36);

    // Using a provisional platform, before we can get the runtime info
    let (loader_suffix, openxr_version) = match alvr_system_info::platform(None, None) {
        Platform::Quest1 => ("_quest1", LEGACY_OPENXR_VERSION),
        Platform::PicoNeo3
        | Platform::PicoG3
        | Platform::Pico4
        | Platform::Pico4Pro
        | Platform::Pico4Enterprise => ("_pico_old", LEGACY_OPENXR_VERSION),
        p if p.is_vive() => ("", LEGACY_OPENXR_VERSION),
        p if p.is_yvr() => ("_yvr", LEGACY_OPENXR_VERSION),
        Platform::Lynx => ("_lynx", LEGACY_OPENXR_VERSION),
        _ => ("", CURRENT_OPENXR_VERSION),
    };
    let xr_entry = unsafe {
        xr::Entry::load_from(Path::new(&format!("libopenxr_loader{loader_suffix}.so"))).unwrap()
    };

    #[cfg(target_os = "android")]
    xr_entry.initialize_android_loader().unwrap();

    let available_extensions = xr_entry.enumerate_extensions().unwrap();
    info!("OpenXR available extensions: {available_extensions:#?}");
    info!(
        "Extra available extensions: {:#?}",
        available_extensions
            .other
            .iter()
            .map(|vec| CStr::from_bytes_with_nul(vec)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned())
            .collect::<Vec<_>>()
    );

    // todo: switch to vulkan
    assert!(available_extensions.khr_opengl_es_enable);

    let mut exts = xr::ExtensionSet::default();
    exts.bd_controller_interaction = available_extensions.bd_controller_interaction;
    exts.ext_eye_gaze_interaction = available_extensions.ext_eye_gaze_interaction;
    exts.ext_hand_tracking = available_extensions.ext_hand_tracking;
    exts.ext_local_floor = available_extensions.ext_local_floor;
    exts.ext_user_presence = available_extensions.ext_user_presence;
    exts.fb_body_tracking = available_extensions.fb_body_tracking;
    exts.fb_color_space = available_extensions.fb_color_space;
    exts.fb_composition_layer_settings = available_extensions.fb_composition_layer_settings;
    exts.fb_display_refresh_rate = available_extensions.fb_display_refresh_rate;
    exts.fb_eye_tracking_social = available_extensions.fb_eye_tracking_social;
    exts.fb_face_tracking2 = available_extensions.fb_face_tracking2;
    exts.fb_foveation = available_extensions.fb_foveation;
    exts.fb_foveation_configuration = available_extensions.fb_foveation_configuration;
    exts.fb_passthrough = available_extensions.fb_passthrough;
    exts.fb_swapchain_update_state = available_extensions.fb_swapchain_update_state;
    exts.htc_facial_tracking = available_extensions.htc_facial_tracking;
    exts.htc_passthrough = available_extensions.htc_passthrough;
    exts.htc_vive_focus3_controller_interaction =
        available_extensions.htc_vive_focus3_controller_interaction;
    #[cfg(target_os = "android")]
    {
        exts.khr_android_create_instance = true;
    }
    exts.khr_convert_timespec_time = true;
    exts.khr_opengl_es_enable = true;
    exts.other = available_extensions
        .other
        .into_iter()
        .filter(|ext| {
            [
                META_BODY_TRACKING_FULL_BODY_EXTENSION_NAME,
                META_BODY_TRACKING_FIDELITY_EXTENSION_NAME,
                META_SIMULTANEOUS_HANDS_AND_CONTROLLERS_EXTENSION_NAME,
                META_DETACHED_CONTROLLERS_EXTENSION_NAME,
                BD_BODY_TRACKING_EXTENSION_NAME,
                BD_MOTION_TRACKING_EXTENSION_NAME,
                PICO_CONFIGURATION_EXTENSION_NAME,
            ]
            .contains(&CStr::from_bytes_with_nul(ext).unwrap().to_str().unwrap())
        })
        .collect();

    let available_layers = xr_entry.enumerate_layers().unwrap();
    info!("OpenXR available layers: {available_layers:#?}");

    let other_exts = exts
        .other
        .iter()
        .map(|vec| {
            CStr::from_bytes_with_nul(vec)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();

    let xr_instance = xr_entry
        .create_instance(
            &xr::ApplicationInfo {
                application_name: "ALVR Client",
                application_version: 0,
                engine_name: "ALVR",
                engine_version: 0,
                api_version: openxr_version,
            },
            &exts,
            &[],
        )
        .unwrap();

    let platform = alvr_system_info::platform(
        xr_instance
            .properties()
            .ok()
            .map(|s| s.runtime_name.to_owned()),
        xr_instance
            .properties()
            .ok()
            .map(|s| s.runtime_version.into_raw()),
    );

    let graphics_context = Rc::new(GraphicsContext::new_gl());

    let mut last_lobby_message = String::new();

    'session_loop: loop {
        let xr_system = xr_instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .unwrap();

        // mandatory call
        let _ = xr_instance
            .graphics_requirements::<xr::OpenGlEs>(xr_system)
            .unwrap();

        let (xr_session, mut xr_frame_waiter, mut xr_frame_stream) =
            create_session(&xr_instance, xr_system, &graphics_context);

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

        let max_view_resolution = UVec2::new(
            views_config[0].max_image_rect_width,
            views_config[0].max_image_rect_height,
        );

        let refresh_rates = if exts.fb_display_refresh_rate {
            xr_session.enumerate_display_refresh_rates().unwrap()
        } else {
            vec![90.0]
        };

        if exts.fb_color_space {
            xr_session
                .set_color_space(xr::ColorSpaceFB::REC709)
                .unwrap();
        }

        let capabilities = ClientCapabilities {
            platform,
            default_view_resolution,
            max_view_resolution,
            refresh_rates,
            foveated_encoding: platform != Platform::Unknown,
            encoder_high_profile: platform != Platform::Unknown,
            encoder_10_bits: platform != Platform::Unknown,
            encoder_av1: matches!(
                platform,
                Platform::Quest3 | Platform::Quest3S | Platform::Pico4Ultra
            ),
            prefer_10bit: false,
            preferred_encoding_gamma: 1.0,
            prefer_hdr: false,
        };
        let core_context = Arc::new(ClientCoreContext::new(capabilities));

        let interaction_context = Arc::new(RwLock::new(InteractionContext::new(
            xr_session.clone(),
            other_exts.clone(),
            xr_system,
            platform,
        )));

        let mut lobby = Lobby::new(
            xr_session.clone(),
            Rc::clone(&graphics_context),
            Arc::clone(&interaction_context),
            platform,
            UVec2::min(default_view_resolution * 2, max_view_resolution),
            &last_lobby_message,
        );

        // For Meta/Quest enabling body tracking would disable multimodal input
        let lobby_body_tracking_config = if platform.is_pico() {
            Some(BodyTrackingSourcesConfig {
                bd: BodyTrackingBDConfig::BodyTracking {
                    high_accuracy: true,
                    prompt_calibration_on_start: false,
                },
                meta: Default::default(),
            })
        } else {
            None
        };
        let lobby_interaction_sources = InteractionSourcesConfig {
            face_tracking: None,
            body_tracking: lobby_body_tracking_config,
            prefers_multimodal_input: true,
        };
        interaction_context
            .write()
            .select_sources(&lobby_interaction_sources);

        let mut session_running = false;
        let mut stream_context = None::<StreamContext>;
        let mut passthrough_layer = None;

        let mut event_storage = xr::EventDataBuffer::new();
        let mut headset_is_worn = true;
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

                            core_context.resume();

                            passthrough_layer = PassthroughLayer::new(&xr_session, platform).ok();

                            session_running = true;
                        }
                        xr::SessionState::STOPPING => {
                            session_running = false;

                            passthrough_layer = None;

                            core_context.pause();

                            xr_session.end().unwrap();
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            break 'render_loop;
                        }
                        _ => (),
                    },
                    xr::Event::ReferenceSpaceChangePending(event) => {
                        info!(
                            "ReferenceSpaceChangePending type: {:?}",
                            event.reference_space_type()
                        );

                        lobby.update_reference_space();

                        if let Some(stream) = &mut stream_context {
                            stream.update_reference_space();
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
                    xr::Event::InteractionProfileChanged(_)
                    | xr::Event::PassthroughStateChangedFB(_) => {
                        // todo
                    }
                    xr::Event::UserPresenceChangedEXT(event) => {
                        debug!("user present: {:?}", event.is_user_present());
                        headset_is_worn = event.is_user_present();

                        core_context.send_proximity_state(event.is_user_present());
                    }
                    xr::Event::Unknown => {
                        // use event_storage.as_raw(), reinterpret as sys::BaseInStructure, get type
                        // and then reinterpret as the event struct
                    }
                    _ => (),
                }
            }

            if !session_running {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            while let Some(event) = core_context.poll_event() {
                match event {
                    ClientCoreEvent::UpdateHudMessage(message) => {
                        last_lobby_message.clone_from(&message);
                        lobby.update_hud_message(&message);
                    }
                    ClientCoreEvent::StreamingStarted(config) => {
                        let config = ParsedStreamConfig::new(&config);

                        let context = StreamContext::new(
                            Arc::clone(&core_context),
                            xr_session.clone(),
                            Rc::clone(&graphics_context),
                            Arc::clone(&interaction_context),
                            config,
                        );

                        if !context.uses_passthrough() {
                            passthrough_layer = None;
                        }

                        stream_context = Some(context);

                        core_context.send_proximity_state(headset_is_worn);
                    }
                    ClientCoreEvent::StreamingStopped => {
                        if passthrough_layer.is_none() {
                            passthrough_layer = PassthroughLayer::new(&xr_session, platform).ok();
                        }

                        interaction_context
                            .write()
                            .select_sources(&lobby_interaction_sources);

                        stream_context = None;
                    }
                    ClientCoreEvent::Haptics {
                        device_id,
                        duration,
                        frequency,
                        amplitude,
                    } => {
                        let idx = if device_id == *HAND_LEFT_ID { 0 } else { 1 };
                        let action =
                            &interaction_context.read().hands_interaction[idx].vibration_action;

                        action
                            .apply_feedback(
                                &xr_session,
                                xr::Path::NULL,
                                &xr::HapticVibration::new()
                                    .amplitude(amplitude.clamp(0.0, 1.0))
                                    .frequency(frequency.max(0.0))
                                    .duration(xr::Duration::from_nanos(duration.as_nanos() as _)),
                            )
                            .unwrap();
                    }
                    ClientCoreEvent::DecoderConfig { codec, config_nal } => {
                        if let Some(stream) = &mut stream_context {
                            stream.maybe_initialize_decoder(codec, config_nal);
                        }
                    }
                    ClientCoreEvent::RealTimeConfig(config) => {
                        if config.passthrough.is_some() && passthrough_layer.is_none() {
                            passthrough_layer = PassthroughLayer::new(&xr_session, platform).ok();
                        } else if config.passthrough.is_none() && passthrough_layer.is_some() {
                            passthrough_layer = None;
                        }

                        if let Some(stream) = &mut stream_context {
                            stream.update_real_time_config(&config);
                        }
                    }
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

            // todo: allow rendering lobby and stream layers at the same time and add cross fade
            let (layer, display_time) = if let Some(stream) = &mut stream_context {
                stream.render(frame_interval, vsync_time)
            } else {
                (lobby.render(vsync_time), vsync_time)
            };

            let layers: &[&xr::CompositionLayerBase<_>] =
                if let Some(passthrough_layer) = &passthrough_layer {
                    &[passthrough_layer, &layer.build()]
                } else {
                    &[&layer.build()]
                };

            graphics_context.make_current();
            let res = xr_frame_stream.end(
                to_xr_time(display_time),
                xr::EnvironmentBlendMode::OPAQUE,
                layers,
            );

            if let Err(e) = res {
                let time = to_xr_time(display_time);
                error!("End frame failed! {e}, timestamp: {display_time:?}, time: {time:?}");

                if !platform.is_vive() {
                    xr_frame_stream
                        .end(
                            frame_state.predicted_display_time,
                            xr::EnvironmentBlendMode::OPAQUE,
                            &[],
                        )
                        .unwrap();
                }
            }
        }
    }

    // grapics_context is dropped here
}

#[allow(unused)]
fn xr_runtime_now(xr_instance: &xr::Instance) -> Option<xr::Time> {
    xr_instance
        .now()
        .ok()
        .filter(|&time_nanos| time_nanos.as_nanos() > 0)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
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
