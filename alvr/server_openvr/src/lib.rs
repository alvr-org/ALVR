mod graphics;
mod props;
mod tracking;

#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    non_snake_case,
    clippy::unseparated_literal_suffix
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

use alvr_common::{
    BUTTON_INFO, HAND_LEFT_ID, HAND_RIGHT_ID, HAND_TRACKER_LEFT_ID, HAND_TRACKER_RIGHT_ID, HEAD_ID,
    Pose, ViewParams, error,
    parking_lot::{Mutex, RwLock},
    settings_schema::Switch,
    warn,
};
use alvr_filesystem as afs;
use alvr_packets::{ButtonValue, Haptics};
use alvr_server_core::{
    HandType, ServerCoreContext, ServerCoreEvent, ServerNegotiatedStreamingConfig,
};
use alvr_session::{
    BodyTrackingSinkConfig, CodecType, ControllersConfig, ControllersEmulationMode,
};
use std::{
    collections::VecDeque,
    ffi::{CString, OsStr, c_char, c_void},
    ptr,
    sync::{Once, mpsc},
    thread,
    time::{Duration, Instant},
};

static SERVER_CORE_CONTEXT: RwLock<Option<ServerCoreContext>> = RwLock::new(None);
static LOCAL_VIEW_PARAMS: RwLock<[ViewParams; 2]> = RwLock::new([ViewParams::DUMMY; 2]);
static HEAD_POSE_QUEUE: Mutex<VecDeque<(Duration, Pose)>> = Mutex::new(VecDeque::new());

fn make_settings(negotiated: Option<&ServerNegotiatedStreamingConfig>) -> Settings {
    let settings = alvr_server_core::settings();
    let video = &settings.video;
    let nvenc = &video.encoder_config.nvenc;
    let amf = &video.encoder_config.amf;
    let hdr = &video.encoder_config.hdr;

    let mut capture_frame_dir = [0i8; 1024];
    let cstr = CString::new(settings.extra.capture.capture_frame_dir.as_str()).unwrap_or_default();
    let bytes = cstr.as_bytes_with_nul();
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr().cast(),
            capture_frame_dir.as_mut_ptr(),
            bytes.len().min(1024),
        )
    };

    let (controllers_enabled, controller_is_tracker, use_separate_hand_trackers) =
        if let Switch::Enabled(config) = &settings.headset.controllers {
            (
                true,
                matches!(config.emulation_mode, ControllersEmulationMode::ViveTracker),
                config
                    .hand_skeleton
                    .as_option()
                    .is_some_and(|c| c.steamvr_input_2_0),
            )
        } else {
            (false, false, false)
        };

    let body_tracking_vive_enabled =
        if let Switch::Enabled(config) = &settings.headset.body_tracking {
            matches!(config.sink, BodyTrackingSinkConfig::FakeViveTracker)
        } else if let Switch::Enabled(config) = &settings.headset.multimodal_tracking {
            config.detached_controllers_steamvr_sink
        } else {
            false
        };

    let body_tracking_has_legs = settings
        .headset
        .body_tracking
        .as_option()
        .map(|c| c.sources.meta.prefer_full_body)
        .unwrap_or(false);

    let (
        fov_center_size_x,
        fov_center_size_y,
        fov_center_shift_x,
        fov_center_shift_y,
        fov_edge_ratio_x,
        fov_edge_ratio_y,
    ) = if let Switch::Enabled(config) = &video.foveated_encoding {
        (
            config.center_size_x,
            config.center_size_y,
            config.center_shift_x,
            config.center_shift_y,
            config.edge_ratio_x,
            config.edge_ratio_y,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    };

    let (enable_color_correction, brightness, contrast, saturation, gamma, sharpening) =
        if let Switch::Enabled(config) = &video.color_correction {
            (
                true,
                config.brightness,
                config.contrast,
                config.saturation,
                config.gamma,
                config.sharpening,
            )
        } else {
            (false, 0.0, 0.0, 0.0, 0.0, 0.0)
        };

    // Resolution and refresh rate come from negotiation; fall back to persisted steamvr_hmd_init_config.
    let (render_width, render_height, target_width, target_height, refresh_rate) =
        if let Some(n) = negotiated {
            (
                n.transcoding_view_resolution.x * 2,
                n.transcoding_view_resolution.y,
                n.emulated_headset_view_resolution.x * 2,
                n.emulated_headset_view_resolution.y,
                n.refresh_rate as i32,
            )
        } else {
            let openvr = alvr_server_core::steamvr_hmd_init_config();
            (
                openvr.eye_resolution_width * 2,
                openvr.eye_resolution_height,
                openvr.target_eye_resolution_width * 2,
                openvr.target_eye_resolution_height,
                openvr.refresh_rate as i32,
            )
        };

    // Encoder params determined by negotiation; default to zero before any client connects.
    let (
        enable_foveated_encoding,
        codec,
        h264_profile,
        use_10bit_encoder,
        encoding_gamma,
        enable_hdr,
    ) = if let Some(n) = negotiated {
        (
            n.enable_foveated_encoding,
            n.codec as i32,
            n.h264_profile as i32,
            n.use_10bit_encoder,
            n.encoding_gamma as f64,
            n.enable_hdr,
        )
    } else {
        (false, 0, 0, false, 0.0, false)
    };

    Settings {
        m_refreshRate: refresh_rate,
        m_renderWidth: render_width,
        m_renderHeight: render_height,
        m_recommendedTargetWidth: target_width as i32,
        m_recommendedTargetHeight: target_height as i32,
        m_nAdapterIndex: video.adapter_index as i32,
        m_captureFrameDir: capture_frame_dir,
        m_enableFoveatedEncoding: enable_foveated_encoding,
        m_foveationCenterSizeX: fov_center_size_x,
        m_foveationCenterSizeY: fov_center_size_y,
        m_foveationCenterShiftX: fov_center_shift_x,
        m_foveationCenterShiftY: fov_center_shift_y,
        m_foveationEdgeRatioX: fov_edge_ratio_x,
        m_foveationEdgeRatioY: fov_edge_ratio_y,
        m_enableColorCorrection: enable_color_correction,
        m_brightness: brightness,
        m_contrast: contrast,
        m_saturation: saturation,
        m_gamma: gamma,
        m_sharpening: sharpening,
        m_codec: codec,
        m_h264Profile: h264_profile,
        m_use10bitEncoder: use_10bit_encoder,
        m_encodingGamma: encoding_gamma,
        m_enableHdr: enable_hdr,
        m_forceHdrSrgbCorrection: hdr.force_hdr_srgb_correction,
        m_clampHdrExtendedRange: hdr.clamp_hdr_extended_range,
        m_enableAmfPreAnalysis: amf.enable_pre_analysis,
        m_enableVbaq: video.encoder_config.enable_vbaq,
        m_enableAmfHmqb: amf.enable_hmqb,
        m_useAmfPreproc: amf.use_preproc,
        m_amfPreProcSigma: amf.preproc_sigma,
        m_amfPreProcTor: amf.preproc_tor,
        m_encoderQualityPreset: video.encoder_config.quality_preset as u32,
        m_amdBitrateCorruptionFix: video.bitrate.image_corruption_fix,
        m_nvencQualityPreset: nvenc.quality_preset as u32,
        m_rateControlMode: video.encoder_config.rate_control_mode as u32,
        m_fillerData: video.encoder_config.filler_data,
        m_entropyCoding: video.encoder_config.entropy_coding as u32,
        m_forceSwEncoding: video.encoder_config.software.force_software_encoding,
        m_swThreadCount: video.encoder_config.software.thread_count,
        m_nvencTuningPreset: nvenc.tuning_preset as u32,
        m_nvencMultiPass: nvenc.multi_pass as u32,
        m_nvencAdaptiveQuantizationMode: nvenc.adaptive_quantization_mode as u32,
        m_nvencLowDelayKeyFrameScale: nvenc.low_delay_key_frame_scale,
        m_nvencRefreshRate: nvenc.refresh_rate,
        m_nvencEnableIntraRefresh: nvenc.enable_intra_refresh,
        m_nvencIntraRefreshPeriod: nvenc.intra_refresh_period,
        m_nvencIntraRefreshCount: nvenc.intra_refresh_count,
        m_nvencMaxNumRefFrames: nvenc.max_num_ref_frames,
        m_nvencGopLength: nvenc.gop_length,
        m_nvencPFrameStrategy: nvenc.p_frame_strategy,
        m_nvencRateControlMode: nvenc.rate_control_mode,
        m_nvencRcBufferSize: nvenc.rc_buffer_size,
        m_nvencRcInitialDelay: nvenc.rc_initial_delay,
        m_nvencRcMaxBitrate: nvenc.rc_max_bitrate,
        m_nvencRcAverageBitrate: nvenc.rc_average_bitrate,
        m_nvencEnableWeightedPrediction: nvenc.enable_weighted_prediction,
        m_minimumIdrIntervalMs: settings.connection.minimum_idr_interval_ms,
        m_enableViveTrackerProxy: settings.headset.enable_vive_tracker_proxy,
        m_trackingRefOnly: settings.headset.tracking_ref_only,
        m_enableLinuxVulkanAsyncCompute: settings.extra.patches.linux_async_compute,
        m_enableLinuxAsyncReprojection: settings.extra.patches.linux_async_reprojection,
        m_enableControllers: controllers_enabled,
        m_controllerIsTracker: controller_is_tracker,
        m_enableBodyTrackingFakeVive: body_tracking_vive_enabled,
        m_bodyTrackingHasLegs: body_tracking_has_legs,
        m_useSeparateHandTrackers: use_separate_hand_trackers,
    }
}

fn spawn_event_loop(events_receiver: mpsc::Receiver<ServerCoreEvent>) {
    thread::spawn(move || {
        if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
            context.start_connection();
        }

        let mut last_resync = Instant::now();
        loop {
            let event = match events_receiver.recv_timeout(Duration::from_millis(5)) {
                Ok(event) => event,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };

            match event {
                ServerCoreEvent::SetOpenvrProperty { device_id, prop } => {
                    props::set_openvr_prop(None, device_id, prop)
                }
                ServerCoreEvent::ClientConnected(config) => unsafe {
                    if InitializeStreaming(make_settings(Some(&config))) {
                        RequestDriverResync();
                    } else {
                        SERVER_CORE_CONTEXT.write().take();

                        ShutdownSteamvr();
                    }
                },

                ServerCoreEvent::ClientDisconnected => unsafe { DeinitializeStreaming() },
                ServerCoreEvent::Battery(info) => unsafe {
                    SetBattery(info.device_id, info.gauge_value, info.is_plugged)
                },
                ServerCoreEvent::PlayspaceSync(bounds) => unsafe {
                    SetChaperoneArea(bounds.x, bounds.y)
                },
                ServerCoreEvent::LocalViewParams(params) => unsafe {
                    *LOCAL_VIEW_PARAMS.write() = params;

                    let ffi_params = [
                        tracking::to_ffi_view_params(params[0]),
                        tracking::to_ffi_view_params(params[1]),
                    ];
                    SetLocalViewParams(ffi_params.as_ptr());
                },
                ServerCoreEvent::Tracking { poll_timestamp } => {
                    let headset_config = &alvr_server_core::settings().headset;

                    let controllers_config = headset_config.controllers.clone().into_option();
                    let track_body = headset_config.body_tracking.enabled();

                    let tracked = controllers_config.as_ref().is_some_and(|c| c.tracked);
                    let detached_controllers = headset_config
                        .multimodal_tracking
                        .as_option()
                        .is_some_and(|c| c.detached_controllers_steamvr_sink);

                    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
                        let target_timestamp =
                            poll_timestamp + context.get_motion_to_photon_latency();
                        let controllers_pose_time_offset = context.get_tracker_pose_time_offset();
                        // We need to remove the additional offset that SteamVR adds
                        let target_controller_timestamp =
                            target_timestamp.saturating_sub(controllers_pose_time_offset);

                        let ffi_head_motion = if let Some(motion) =
                            context.get_device_motion(*HEAD_ID, poll_timestamp)
                        {
                            let motion = motion.predict(poll_timestamp, target_timestamp);

                            let mut head_pose_queue_lock = HEAD_POSE_QUEUE.lock();
                            head_pose_queue_lock.push_back((poll_timestamp, motion.pose));
                            while head_pose_queue_lock.len() > 360 {
                                head_pose_queue_lock.pop_front();
                            }

                            tracking::to_ffi_motion(*HEAD_ID, motion)
                        } else {
                            FfiDeviceMotion::default()
                        };

                        let (ffi_left_controller_motion, ffi_right_controller_motion) =
                            if tracked && let Some(config) = &controllers_config {
                                let ffi_left_controller_motion = context
                                    .get_device_motion(*HAND_LEFT_ID, poll_timestamp)
                                    .map(|motion| {
                                        let motion = motion
                                            .predict(poll_timestamp, target_controller_timestamp);
                                        let motion = tracking::offset_controller_motion(
                                            config,
                                            *HAND_LEFT_ID,
                                            motion,
                                        );
                                        tracking::to_ffi_motion(*HAND_LEFT_ID, motion)
                                    });
                                let ffi_right_controller_motion = context
                                    .get_device_motion(*HAND_RIGHT_ID, poll_timestamp)
                                    .map(|motion| {
                                        let motion = motion
                                            .predict(poll_timestamp, target_controller_timestamp);
                                        let motion = tracking::offset_controller_motion(
                                            config,
                                            *HAND_RIGHT_ID,
                                            motion,
                                        );
                                        tracking::to_ffi_motion(*HAND_RIGHT_ID, motion)
                                    });

                                (ffi_left_controller_motion, ffi_right_controller_motion)
                            } else {
                                (None, None)
                            };

                        let (
                            ffi_left_hand_skeleton,
                            ffi_right_hand_skeleton,
                            use_separate_hand_trackers,
                            predict_hand_skeleton,
                        ) = if let Some(ControllersConfig {
                            hand_skeleton: Switch::Enabled(hand_skeleton_config),
                            ..
                        }) = controllers_config
                        {
                            let left_hand_skeleton = context
                                .get_hand_skeleton(HandType::Left, poll_timestamp)
                                .map(|s| {
                                    tracking::to_openvr_ffi_hand_skeleton(
                                        headset_config,
                                        *HAND_LEFT_ID,
                                        &s,
                                    )
                                });
                            let right_hand_skeleton = context
                                .get_hand_skeleton(HandType::Right, poll_timestamp)
                                .map(|s| {
                                    tracking::to_openvr_ffi_hand_skeleton(
                                        headset_config,
                                        *HAND_RIGHT_ID,
                                        &s,
                                    )
                                });

                            (
                                tracked.then_some(left_hand_skeleton).flatten(),
                                tracked.then_some(right_hand_skeleton).flatten(),
                                hand_skeleton_config.steamvr_input_2_0,
                                hand_skeleton_config.predict,
                            )
                        } else {
                            (None, None, false, false)
                        };

                        let ffi_left_hand_data = FfiHandData {
                            controllerMotion: if let Some(motion) = &ffi_left_controller_motion {
                                motion
                            } else {
                                ptr::null()
                            },
                            handSkeleton: if let Some(skeleton) = &ffi_left_hand_skeleton {
                                skeleton
                            } else {
                                ptr::null()
                            },
                            isHandTracker: use_separate_hand_trackers
                                && ffi_left_controller_motion.is_none()
                                && ffi_left_hand_skeleton.is_some(),
                            predictHandSkeleton: predict_hand_skeleton,
                        };
                        let ffi_right_hand_data = FfiHandData {
                            controllerMotion: if let Some(motion) = &ffi_right_controller_motion {
                                motion
                            } else {
                                ptr::null()
                            },
                            handSkeleton: if let Some(skeleton) = &ffi_right_hand_skeleton {
                                skeleton
                            } else {
                                ptr::null()
                            },
                            isHandTracker: use_separate_hand_trackers
                                && ffi_right_controller_motion.is_none()
                                && ffi_right_hand_skeleton.is_some(),
                            predictHandSkeleton: predict_hand_skeleton,
                        };

                        let ffi_body_tracker_motions = if track_body || detached_controllers {
                            tracking::BODY_TRACKER_IDS
                                .iter()
                                .filter_map(|id| {
                                    Some(tracking::to_ffi_motion(
                                        *id,
                                        context.get_device_motion(*id, poll_timestamp)?,
                                    ))
                                })
                                .collect::<Vec<_>>()
                        } else {
                            vec![]
                        };

                        // There are two pairs of controllers/hand tracking devices registered in
                        // OpenVR, two lefts and two rights. If enabled with use_separate_hand_trackers,
                        // we select at runtime which device to use (selected for left and right hand
                        // independently. Selection is done by setting deviceIsConnected.
                        unsafe {
                            SetTracking(
                                poll_timestamp.as_nanos() as _,
                                controllers_pose_time_offset.as_secs_f32(),
                                ffi_head_motion,
                                ffi_left_hand_data,
                                ffi_right_hand_data,
                                ffi_body_tracker_motions.as_ptr(),
                                ffi_body_tracker_motions.len() as i32,
                            )
                        };
                    }
                }
                ServerCoreEvent::Buttons(entries) => {
                    for entry in entries {
                        let value = match entry.value {
                            ButtonValue::Binary(value) => FfiButtonValue {
                                type_: FfiButtonType_BUTTON_TYPE_BINARY,
                                __bindgen_anon_1: FfiButtonValue__bindgen_ty_1 {
                                    binary: value.into(),
                                },
                            },

                            ButtonValue::Scalar(value) => FfiButtonValue {
                                type_: FfiButtonType_BUTTON_TYPE_SCALAR,
                                __bindgen_anon_1: FfiButtonValue__bindgen_ty_1 { scalar: value },
                            },
                        };
                        unsafe { SetButton(entry.path_id, value) };
                    }
                }
                ServerCoreEvent::RequestIDR => unsafe { RequestIDR() },
                ServerCoreEvent::CaptureFrame => unsafe { CaptureFrame() },
                ServerCoreEvent::GameRenderLatencyFeedback(game_latency) => {
                    if cfg!(target_os = "linux") && game_latency.as_secs_f32() > 0.25 {
                        let now = Instant::now();
                        if now.saturating_duration_since(last_resync).as_secs_f32() > 0.1 {
                            last_resync = now;
                            warn!("Desync detected. Attempting recovery.");
                            unsafe {
                                RequestDriverResync();
                            }
                        }
                    }
                }
                ServerCoreEvent::ShutdownPending => {
                    SERVER_CORE_CONTEXT.write().take();

                    unsafe { ShutdownSteamvr() };
                }
                ServerCoreEvent::RestartPending => {
                    if let Some(context) = SERVER_CORE_CONTEXT.write().take() {
                        context.restart();
                    }

                    unsafe { ShutdownSteamvr() };
                }
                ServerCoreEvent::ProximityState(headset_is_worn) => unsafe {
                    SetProximityState(headset_is_worn)
                },
            }
        }

        unsafe { ShutdownOpenvrClient() };
    });
}

extern "C" fn driver_ready_idle(set_default_chap: bool) {
    thread::spawn(move || {
        unsafe { InitOpenvrClient() };

        if set_default_chap {
            // call this when inside a new thread. Calling this on the parent thread will crash SteamVR
            unsafe {
                SetChaperoneArea(2.0, 2.0);
            }
        }
    });
}

/// # Safety
/// `instance_ptr` is a valid pointer to a `TrackedDevice` instance
pub unsafe extern "C" fn register_buttons(instance_ptr: *mut c_void, device_id: u64) {
    let mapped_device_id = if device_id == *HAND_TRACKER_LEFT_ID {
        *HAND_LEFT_ID
    } else if device_id == *HAND_TRACKER_RIGHT_ID {
        *HAND_RIGHT_ID
    } else {
        device_id
    };

    for button_id in alvr_server_core::registered_button_set() {
        if let Some(info) = BUTTON_INFO.get(&button_id) {
            if info.device_id == mapped_device_id {
                unsafe { RegisterButton(instance_ptr, button_id) };
            }
        } else {
            error!("Cannot register unrecognized button ID {button_id}");
        }
    }
}

extern "C" fn send_haptics(device_id: u64, duration_s: f32, frequency: f32, amplitude: f32) {
    if let Ok(duration) = Duration::try_from_secs_f32(duration_s)
        && let Some(context) = &*SERVER_CORE_CONTEXT.read()
    {
        context.send_haptics(Haptics {
            device_id,
            duration,
            frequency,
            amplitude,
        });
    }
}

extern "C" fn set_video_config_nals(buffer_ptr: *const u8, len: i32, codec: i32) {
    let codec = if codec == 0 {
        CodecType::H264
    } else if codec == 1 {
        CodecType::Hevc
    } else {
        CodecType::AV1
    };

    let mut config_buffer = vec![0; len as usize];

    unsafe { ptr::copy_nonoverlapping(buffer_ptr, config_buffer.as_mut_ptr(), len as usize) };

    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.set_video_config_nals(config_buffer, codec);
    }
}

extern "C" fn send_video(timestamp_ns: u64, buffer_ptr: *mut u8, len: i32, is_idr: bool) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        let timestamp = Duration::from_nanos(timestamp_ns);
        let buffer = unsafe { std::slice::from_raw_parts(buffer_ptr, len as usize) };

        let Some(head_pose) = HEAD_POSE_QUEUE
            .lock()
            .iter()
            .find_map(|(ts, pose)| (*ts == timestamp).then_some(*pose))
        else {
            // We can't submit the frame without its pose
            return;
        };

        let local_views_params = LOCAL_VIEW_PARAMS.read();

        let global_view_params = [
            ViewParams {
                pose: head_pose * local_views_params[0].pose,
                fov: local_views_params[0].fov,
            },
            ViewParams {
                pose: head_pose * local_views_params[1].pose,
                fov: local_views_params[1].fov,
            },
        ];

        context.send_video_nal(timestamp, global_view_params, is_idr, buffer.to_vec());
    }
}

extern "C" fn get_dynamic_encoder_params() -> FfiDynamicEncoderParams {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read()
        && let Some(params) = context.get_dynamic_encoder_params()
    {
        FfiDynamicEncoderParams {
            updated: 1,
            bitrate_bps: params.bitrate_bps as u64,
            framerate: params.framerate,
        }
    } else {
        FfiDynamicEncoderParams::default()
    }
}

extern "C" fn report_composed(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_composed(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

extern "C" fn report_present(timestamp_ns: u64, offset_ns: u64) {
    if let Some(context) = &*SERVER_CORE_CONTEXT.read() {
        context.report_present(
            Duration::from_nanos(timestamp_ns),
            Duration::from_nanos(offset_ns),
        );
    }
}

extern "C" fn wait_for_vsync() {
    // Default 120Hz-ish wait if StatisticsManager isn't up.
    // We use 120Hz-ish so that SteamVR doesn't accidentally get
    // any weird ideas about our display Hz with its frame pacing.
    static PRE_HEADSET_STATS_WAIT_INTERVAL: Duration = Duration::from_millis(8);

    // NB: don't sleep while locking SERVER_DATA_MANAGER or SERVER_CORE_CONTEXT
    let sleep_duration = SERVER_CORE_CONTEXT
        .read()
        .as_ref()
        .and_then(|ctx| ctx.duration_until_next_vsync());

    if let Some(duration) = sleep_duration {
        if alvr_server_core::settings()
            .video
            .enforce_server_frame_pacing
        {
            thread::sleep(duration);
        } else {
            thread::yield_now();
        }
    } else {
        // StatsManager isn't up because the headset hasn't connected,
        // safety fallback to prevent deadlocking.
        thread::sleep(PRE_HEADSET_STATS_WAIT_INTERVAL);
    }
}

pub extern "C" fn shutdown_driver() {
    SERVER_CORE_CONTEXT.write().take();
}

/// This is the SteamVR/OpenVR entry point
/// # Safety
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HmdDriverFactory(
    interface_name: *const c_char,
    return_code: *mut i32,
) -> *mut c_void {
    let Ok(driver_dir) = alvr_server_io::get_driver_dir_from_registered() else {
        return ptr::null_mut();
    };
    let Some(filesystem_layout) =
        alvr_filesystem::filesystem_layout_from_openvr_driver_root_dir(&driver_dir)
    else {
        return ptr::null_mut();
    };

    let system = sysinfo::System::new_all();
    let dashboard_processes = system
        .processes_by_name(OsStr::new(&afs::dashboard_fname()))
        .collect::<Vec<_>>();

    static ONCE: Once = Once::new();
    ONCE.call_once(move || {
        alvr_server_core::initialize_environment(filesystem_layout.clone());

        let log_to_disk = alvr_server_core::settings().extra.logging.log_to_disk;

        alvr_server_core::init_logging(
            log_to_disk.then(|| filesystem_layout.session_log()),
            Some(filesystem_layout.crash_log()),
        );

        unsafe {
            g_sessionPath = CString::new(filesystem_layout.session().to_string_lossy().to_string())
                .unwrap()
                .into_raw();
            g_driverRootDir = CString::new(
                filesystem_layout
                    .openvr_driver_root_dir
                    .to_string_lossy()
                    .to_string(),
            )
            .unwrap()
            .into_raw();
        };

        graphics::initialize_shaders();

        unsafe {
            LogError = Some(alvr_server_core::alvr_error);
            LogWarn = Some(alvr_server_core::alvr_warn);
            LogInfo = Some(alvr_server_core::alvr_info);
            LogDebug = Some(alvr_server_core::alvr_dbg_server_impl);
            LogEncoder = Some(alvr_server_core::alvr_dbg_encoder);
            LogPeriodically = Some(alvr_server_core::alvr_log_periodically);
            PathStringToHash = Some(alvr_server_core::alvr_path_to_id);
            GetSerialNumber = Some(props::get_serial_number);
            SetOpenvrProps = Some(props::set_device_openvr_props);
            RegisterButtons = Some(register_buttons);
            DriverReadyIdle = Some(driver_ready_idle);
            HapticsSend = Some(send_haptics);
            SetVideoConfigNals = Some(set_video_config_nals);
            VideoSend = Some(send_video);
            GetDynamicEncoderParams = Some(get_dynamic_encoder_params);
            ReportComposed = Some(report_composed);
            ReportPresent = Some(report_present);
            WaitForVSync = Some(wait_for_vsync);
            ShutdownRuntime = Some(shutdown_driver);

            // When there is already a ALVR dashboard running, initialize the HMD device early to
            // avoid buggy SteamVR behavior
            // NB: we already bail out before if the dashboards don't belong to this streamer
            let early_hmd_initialization = !dashboard_processes.is_empty();

            CppInit(early_hmd_initialization, make_settings(None));
        }

        let (context, events_receiver) = ServerCoreContext::new();

        *SERVER_CORE_CONTEXT.write() = Some(context);

        spawn_event_loop(events_receiver);
    });

    unsafe { CppOpenvrEntryPoint(interface_name, return_code) }
}
