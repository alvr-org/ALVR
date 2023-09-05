use crate::{
    bitrate::BitrateManager,
    face_tracking::FaceTrackingSink,
    hand_gestures::{trigger_hand_gesture_actions, HandGestureManager},
    haptics,
    input_mapping::ButtonMappingManager,
    sockets::WelcomeSocket,
    statistics::StatisticsManager,
    tracking::{self, TrackingManager},
    FfiFov, FfiViewsConfig, VideoPacket, BITRATE_MANAGER, DECODER_CONFIG, SERVER_DATA_MANAGER,
    STATISTICS_MANAGER, VIDEO_MIRROR_SENDER, VIDEO_RECORDING_FILE,
};
use alvr_audio::AudioDevice;
use alvr_common::{
    con_bail, debug, error,
    glam::{UVec2, Vec2},
    info,
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    settings_schema::Switch,
    warn, AnyhowToCon, ConResult, ConnectionError, LazyMutOpt, RelaxedAtomic, ToCon, BUTTON_INFO,
    CONTROLLER_PROFILE_INFO, DEVICE_ID_TO_PATH, HEAD_ID, LEFT_HAND_ID,
    QUEST_CONTROLLER_PROFILE_PATH, RIGHT_HAND_ID,
};
use alvr_events::{ButtonEvent, EventType, HapticsEvent, TrackingEvent};
use alvr_packets::{
    ClientConnectionResult, ClientControlPacket, ClientListAction, ClientStatistics, Haptics,
    ServerControlPacket, StreamConfigPacket, Tracking, VideoPacketHeader, AUDIO, HAPTICS,
    STATISTICS, TRACKING, VIDEO,
};
use alvr_session::{CodecType, ConnectionState, ControllersEmulationMode, FrameSize, OpenvrConfig};
use alvr_sockets::{
    PeerType, ProtoControlSocket, StreamSender, StreamSocketBuilder, KEEPALIVE_INTERVAL,
    KEEPALIVE_TIMEOUT,
};
use std::{
    collections::HashMap,
    io::Write,
    net::IpAddr,
    process::Command,
    ptr::{self},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError, SyncSender, TrySendError},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const HANDSHAKE_ACTION_TIMEOUT: Duration = Duration::from_secs(2);
const STREAMING_RECV_TIMEOUT: Duration = Duration::from_millis(500);

const MAX_UNREAD_PACKETS: usize = 10; // Applies per stream

pub static SHOULD_CONNECT_TO_CLIENTS: Lazy<Arc<RelaxedAtomic>> =
    Lazy::new(|| Arc::new(RelaxedAtomic::new(false)));
pub static IS_STREAMING: Lazy<Arc<RelaxedAtomic>> =
    Lazy::new(|| Arc::new(RelaxedAtomic::new(false)));
static VIDEO_CHANNEL_SENDER: LazyMutOpt<SyncSender<VideoPacket>> = alvr_common::lazy_mut_none();
static HAPTICS_SENDER: LazyMutOpt<StreamSender<Haptics>> = alvr_common::lazy_mut_none();

pub enum ClientDisconnectRequest {
    Disconnect,
    ServerShutdown,
    ServerRestart,
}

pub static DISCONNECT_CLIENT_NOTIFIER: LazyMutOpt<mpsc::Sender<ClientDisconnectRequest>> =
    alvr_common::lazy_mut_none();

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

pub fn contruct_openvr_config() -> OpenvrConfig {
    let data_manager_lock = SERVER_DATA_MANAGER.read();
    let old_config = data_manager_lock.session().openvr_config.clone();
    let settings = data_manager_lock.settings().clone();

    let mut controller_is_tracker = false;
    let mut _controller_profile = 0;
    let controllers_enabled = if let Switch::Enabled(config) = settings.headset.controllers {
        controller_is_tracker =
            matches!(config.emulation_mode, ControllersEmulationMode::ViveTracker);
        _controller_profile = config.emulation_mode as i32;

        true
    } else {
        false
    };

    let mut foveation_center_size_x = 0.0;
    let mut foveation_center_size_y = 0.0;
    let mut foveation_center_shift_x = 0.0;
    let mut foveation_center_shift_y = 0.0;
    let mut foveation_edge_ratio_x = 0.0;
    let mut foveation_edge_ratio_y = 0.0;
    let enable_foveated_rendering =
        if let Switch::Enabled(config) = settings.video.foveated_rendering {
            foveation_center_size_x = config.center_size_x;
            foveation_center_size_y = config.center_size_y;
            foveation_center_shift_x = config.center_shift_x;
            foveation_center_shift_y = config.center_shift_y;
            foveation_edge_ratio_x = config.edge_ratio_x;
            foveation_edge_ratio_y = config.edge_ratio_y;

            true
        } else {
            false
        };

    let mut brightness = 0.0;
    let mut contrast = 0.0;
    let mut saturation = 0.0;
    let mut gamma = 0.0;
    let mut sharpening = 0.0;
    let enable_color_correction = if let Switch::Enabled(config) = settings.video.color_correction {
        brightness = config.brightness;
        contrast = config.contrast;
        saturation = config.saturation;
        gamma = config.gamma;
        sharpening = config.sharpening;
        true
    } else {
        false
    };

    let nvenc_overrides = settings.video.encoder_config.nvenc;
    let amf_controls = settings.video.encoder_config.amf;

    OpenvrConfig {
        tracking_ref_only: settings.headset.tracking_ref_only,
        enable_vive_tracker_proxy: settings.headset.enable_vive_tracker_proxy,
        aggressive_keyframe_resend: settings.connection.aggressive_keyframe_resend,
        adapter_index: settings.video.adapter_index,
        codec: matches!(settings.video.preferred_codec, CodecType::Hevc) as _,
        rate_control_mode: settings.video.encoder_config.rate_control_mode as u32,
        filler_data: settings.video.encoder_config.filler_data,
        entropy_coding: settings.video.encoder_config.entropy_coding as u32,
        use_10bit_encoder: settings.video.encoder_config.use_10bit,
        enable_vbaq: amf_controls.enable_vbaq,
        use_preproc: amf_controls.use_preproc,
        preproc_sigma: amf_controls.preproc_sigma,
        preproc_tor: amf_controls.preproc_tor,
        nvenc_quality_preset: nvenc_overrides.quality_preset as u32,
        amd_encoder_quality_preset: amf_controls.quality_preset as u32,
        force_sw_encoding: settings
            .video
            .encoder_config
            .software
            .force_software_encoding,
        sw_thread_count: settings.video.encoder_config.software.thread_count,
        controllers_enabled,
        controller_is_tracker,
        enable_foveated_rendering,
        foveation_center_size_x,
        foveation_center_size_y,
        foveation_center_shift_x,
        foveation_center_shift_y,
        foveation_edge_ratio_x,
        foveation_edge_ratio_y,
        enable_color_correction,
        brightness,
        contrast,
        saturation,
        gamma,
        sharpening,
        linux_async_reprojection: settings.patches.linux_async_reprojection,
        nvenc_tuning_preset: nvenc_overrides.tuning_preset as u32,
        nvenc_multi_pass: nvenc_overrides.multi_pass as u32,
        nvenc_adaptive_quantization_mode: nvenc_overrides.adaptive_quantization_mode as u32,
        nvenc_low_delay_key_frame_scale: nvenc_overrides.low_delay_key_frame_scale,
        nvenc_refresh_rate: nvenc_overrides.refresh_rate,
        enable_intra_refresh: nvenc_overrides.enable_intra_refresh,
        intra_refresh_period: nvenc_overrides.intra_refresh_period,
        intra_refresh_count: nvenc_overrides.intra_refresh_count,
        max_num_ref_frames: nvenc_overrides.max_num_ref_frames,
        gop_length: nvenc_overrides.gop_length,
        p_frame_strategy: nvenc_overrides.p_frame_strategy,
        nvenc_rate_control_mode: nvenc_overrides.rate_control_mode,
        rc_buffer_size: nvenc_overrides.rc_buffer_size,
        rc_initial_delay: nvenc_overrides.rc_initial_delay,
        rc_max_bitrate: nvenc_overrides.rc_max_bitrate,
        rc_average_bitrate: nvenc_overrides.rc_average_bitrate,
        nvenc_enable_weighted_prediction: nvenc_overrides.enable_weighted_prediction,
        capture_frame_dir: settings.capture.capture_frame_dir,
        amd_bitrate_corruption_fix: settings.video.bitrate.image_corruption_fix,
        _controller_profile,
        ..old_config
    }
}

// Alternate connection trials with manual IPs and clients discovered on the local network
pub fn handshake_loop() {
    let mut welcome_socket = match WelcomeSocket::new(RETRY_CONNECT_MIN_INTERVAL) {
        Ok(socket) => socket,
        Err(e) => {
            error!("Failed to create discovery socket: {e:?}");
            return;
        }
    };

    while SHOULD_CONNECT_TO_CLIENTS.value() {
        let available_manual_client_ips = {
            let mut manual_client_ips = HashMap::new();
            for (hostname, connection_info) in SERVER_DATA_MANAGER
                .read()
                .client_list()
                .iter()
                .filter(|(_, info)| info.connection_state == ConnectionState::Disconnected)
            {
                for ip in &connection_info.manual_ips {
                    manual_client_ips.insert(*ip, hostname.clone());
                }
            }
            manual_client_ips
        };

        if !available_manual_client_ips.is_empty()
            && try_connect(available_manual_client_ips).is_ok()
        {
            thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
            continue;
        }

        let discovery_config = SERVER_DATA_MANAGER
            .read()
            .settings()
            .connection
            .client_discovery
            .clone();
        if let Switch::Enabled(config) = discovery_config {
            let (client_hostname, client_ip) = match welcome_socket.recv() {
                Ok(pair) => pair,
                Err(e) => {
                    if let ConnectionError::Other(e) = e {
                        warn!("UDP handshake listening error: {e:?}");
                    }

                    continue;
                }
            };

            let trusted = {
                let mut data_manager = SERVER_DATA_MANAGER.write();

                data_manager.update_client_list(
                    client_hostname.clone(),
                    ClientListAction::AddIfMissing {
                        trusted: false,
                        manual_ips: vec![],
                    },
                );

                if config.auto_trust_clients {
                    data_manager
                        .update_client_list(client_hostname.clone(), ClientListAction::Trust);
                }

                data_manager
                    .client_list()
                    .get(&client_hostname)
                    .unwrap()
                    .trusted
            };

            // do not attempt connection if the client is already connected
            if trusted
                && SERVER_DATA_MANAGER
                    .read()
                    .client_list()
                    .get(&client_hostname)
                    .unwrap()
                    .connection_state
                    == ConnectionState::Disconnected
            {
                if let Err(e) =
                    try_connect([(client_ip, client_hostname.clone())].into_iter().collect())
                {
                    error!("Handshake error for {client_hostname}: {e}");
                }
            }
        }

        thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
    }
}

fn try_connect(mut client_ips: HashMap<IpAddr, String>) -> ConResult {
    let (mut proto_socket, client_ip) = ProtoControlSocket::connect_to(
        Duration::from_secs(1),
        PeerType::AnyClient(client_ips.keys().cloned().collect()),
    )?;

    let (disconnect_sender, disconnect_receiver) = mpsc::channel();
    *DISCONNECT_CLIENT_NOTIFIER.lock() = Some(disconnect_sender);

    // Safety: this never panics because client_ip is picked from client_ips keys
    let client_hostname = client_ips.remove(&client_ip).unwrap();

    struct DropGuard {
        hostname: String,
    }
    impl Drop for DropGuard {
        fn drop(&mut self) {
            let mut data_manager_lock = SERVER_DATA_MANAGER.write();
            if let Some(entry) = data_manager_lock.client_list().get(&self.hostname) {
                if entry.connection_state
                    == (ConnectionState::Disconnecting {
                        should_be_removed: true,
                    })
                {
                    data_manager_lock
                        .update_client_list(self.hostname.clone(), ClientListAction::RemoveEntry);

                    return;
                }
            }

            data_manager_lock.update_client_list(
                self.hostname.clone(),
                ClientListAction::SetConnectionState(ConnectionState::Disconnected),
            );

            *DISCONNECT_CLIENT_NOTIFIER.lock() = None;
        }
    }
    let _connection_drop_guard = DropGuard {
        hostname: client_hostname.clone(),
    };

    SERVER_DATA_MANAGER.write().update_client_list(
        client_hostname.clone(),
        ClientListAction::SetConnectionState(ConnectionState::Connecting),
    );

    SERVER_DATA_MANAGER.write().update_client_list(
        client_hostname.clone(),
        ClientListAction::UpdateCurrentIp(Some(client_ip)),
    );

    let maybe_streaming_caps = if let ClientConnectionResult::ConnectionAccepted {
        client_protocol_id,
        display_name,
        streaming_capabilities,
        ..
    } = proto_socket.recv(HANDSHAKE_ACTION_TIMEOUT)?
    {
        SERVER_DATA_MANAGER.write().update_client_list(
            client_hostname.clone(),
            ClientListAction::SetDisplayName(display_name),
        );

        if client_protocol_id != alvr_common::protocol_id() {
            warn!(
                "Trusted client is incompatible! Expected protocol ID: {}, found: {}",
                alvr_common::protocol_id(),
                client_protocol_id,
            );

            return Ok(());
        }

        streaming_capabilities
    } else {
        debug!("Found client in standby. Retrying");
        return Ok(());
    };

    let streaming_caps = if let Some(streaming_caps) = maybe_streaming_caps {
        streaming_caps
    } else {
        con_bail!("Only streaming clients are supported for now");
    };

    let settings = SERVER_DATA_MANAGER.read().settings().clone();

    fn get_view_res(config: FrameSize, default_res: UVec2) -> UVec2 {
        let res = match config {
            FrameSize::Scale(scale) => default_res.as_vec2() * scale,
            FrameSize::Absolute { width, height } => {
                let width = width as f32;
                Vec2::new(
                    width,
                    height.map(|h| h as f32).unwrap_or_else(|| {
                        let default_res = default_res.as_vec2();
                        width * default_res.y / default_res.x
                    }),
                )
            }
        };

        UVec2::new(align32(res.x), align32(res.y))
    }

    let stream_view_resolution = get_view_res(
        settings.video.transcoding_view_resolution,
        streaming_caps.default_view_resolution,
    );

    let target_view_resolution = get_view_res(
        settings.video.emulated_headset_view_resolution,
        streaming_caps.default_view_resolution,
    );

    let fps = {
        let mut best_match = 0_f32;
        let mut min_diff = f32::MAX;
        for rr in &streaming_caps.supported_refresh_rates {
            let diff = (*rr - settings.video.preferred_fps).abs();
            if diff < min_diff {
                best_match = *rr;
                min_diff = diff;
            }
        }
        best_match
    };

    if !streaming_caps
        .supported_refresh_rates
        .contains(&settings.video.preferred_fps)
    {
        warn!("Chosen refresh rate not supported. Using {fps}Hz");
    }

    let game_audio_sample_rate =
        if let Switch::Enabled(game_audio_config) = &settings.audio.game_audio {
            let game_audio_device = AudioDevice::new_output(
                Some(settings.audio.linux_backend),
                game_audio_config.device.as_ref(),
            )
            .to_con()?;

            #[cfg(not(target_os = "linux"))]
            if let Switch::Enabled(microphone_desc) = &settings.audio.microphone {
                let (sink, source) = AudioDevice::new_virtual_microphone_pair(
                    Some(settings.audio.linux_backend),
                    microphone_desc.devices.clone(),
                )
                .to_con()?;
                if alvr_audio::is_same_device(&game_audio_device, &sink)
                    || alvr_audio::is_same_device(&game_audio_device, &source)
                {
                    con_bail!("Game audio and microphone cannot point to the same device!");
                }
            }

            game_audio_device.input_sample_rate().to_con()?
        } else {
            0
        };

    let client_config = StreamConfigPacket {
        session: {
            let session = SERVER_DATA_MANAGER.read().session().clone();
            serde_json::to_string(&session).to_con()?
        },
        negotiated: serde_json::json!({
            "view_resolution": stream_view_resolution,
            "refresh_rate_hint": fps,
            "game_audio_sample_rate": game_audio_sample_rate,
        })
        .to_string(),
    };
    proto_socket.send(&client_config).to_con()?;

    let (mut control_sender, mut control_receiver) =
        proto_socket.split(STREAMING_RECV_TIMEOUT).to_con()?;

    let mut new_openvr_config = contruct_openvr_config();
    new_openvr_config.eye_resolution_width = stream_view_resolution.x;
    new_openvr_config.eye_resolution_height = stream_view_resolution.y;
    new_openvr_config.target_eye_resolution_width = target_view_resolution.x;
    new_openvr_config.target_eye_resolution_height = target_view_resolution.y;
    new_openvr_config.refresh_rate = fps as _;

    if SERVER_DATA_MANAGER.read().session().openvr_config != new_openvr_config {
        SERVER_DATA_MANAGER.write().session_mut().openvr_config = new_openvr_config;

        control_sender.send(&ServerControlPacket::Restarting).ok();

        crate::notify_restart_driver();
    }

    control_sender
        .send(&ServerControlPacket::StartStream)
        .to_con()?;

    let signal = control_receiver.recv(HANDSHAKE_ACTION_TIMEOUT)?;
    if !matches!(signal, ClientControlPacket::StreamReady) {
        con_bail!("Got unexpected packet waiting for stream ack");
    }

    *STATISTICS_MANAGER.lock() = Some(StatisticsManager::new(
        settings.connection.statistics_history_size,
        Duration::from_secs_f32(1.0 / fps),
        if let Switch::Enabled(config) = &settings.headset.controllers {
            config.steamvr_pipeline_frames
        } else {
            0.0
        },
    ));

    *BITRATE_MANAGER.lock() = BitrateManager::new(settings.video.bitrate.history_size, fps);

    let mut stream_socket = StreamSocketBuilder::connect_to_client(
        HANDSHAKE_ACTION_TIMEOUT,
        client_ip,
        settings.connection.stream_port,
        settings.connection.stream_protocol,
        settings.connection.server_send_buffer_bytes,
        settings.connection.server_recv_buffer_bytes,
        settings.connection.packet_size as _,
    )?;

    let mut video_sender = stream_socket.request_stream(VIDEO);
    let game_audio_sender = stream_socket.request_stream(AUDIO);
    let microphone_receiver = stream_socket.subscribe_to_stream(AUDIO, MAX_UNREAD_PACKETS);
    let mut tracking_receiver =
        stream_socket.subscribe_to_stream::<Tracking>(TRACKING, MAX_UNREAD_PACKETS);
    let haptics_sender = stream_socket.request_stream(HAPTICS);
    let mut statics_receiver =
        stream_socket.subscribe_to_stream::<ClientStatistics>(STATISTICS, MAX_UNREAD_PACKETS);

    // Note: from here on, the function MUST be infallible. Failure to respect this might leave
    // lingering objects that prevent reconnection.
    IS_STREAMING.set(true);

    let (video_channel_sender, video_channel_receiver) =
        std::sync::mpsc::sync_channel(settings.connection.max_queued_server_video_frames);
    *VIDEO_CHANNEL_SENDER.lock() = Some(video_channel_sender);
    *HAPTICS_SENDER.lock() = Some(haptics_sender);

    let video_send_thread = thread::spawn(move || {
        while IS_STREAMING.value() {
            let VideoPacket { header, payload } =
                match video_channel_receiver.recv_timeout(STREAMING_RECV_TIMEOUT) {
                    Ok(packet) => packet,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => return,
                };

            let mut buffer = video_sender.get_buffer(&header).unwrap();
            // todo: make encoder write to socket buffers directly to avoid copy
            buffer
                .get_range_mut(0, payload.len())
                .copy_from_slice(&payload);
            video_sender.send(buffer).ok();
        }
    });

    let game_audio_thread = if let Switch::Enabled(config) = settings.audio.game_audio {
        thread::spawn(move || {
            while IS_STREAMING.value() {
                let device = match AudioDevice::new_output(
                    Some(settings.audio.linux_backend),
                    config.device.as_ref(),
                ) {
                    Ok(data) => data,
                    Err(e) => {
                        warn!("New audio device failed: {e:?}");
                        thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
                        continue;
                    }
                };

                #[cfg(windows)]
                if let Ok(id) = alvr_audio::get_windows_device_id(&device) {
                    unsafe {
                        crate::SetOpenvrProperty(
                            *alvr_common::HEAD_ID,
                            crate::openvr_props::to_ffi_openvr_prop(
                                alvr_session::OpenvrProperty::AudioDefaultPlaybackDeviceId(id),
                            ),
                        )
                    }
                } else {
                    continue;
                };

                if let Err(e) = alvr_audio::record_audio_blocking(
                    Arc::clone(&IS_STREAMING),
                    game_audio_sender.clone(),
                    &device,
                    2,
                    config.mute_when_streaming,
                ) {
                    error!("Audio record error: {e:?}");
                }

                #[cfg(windows)]
                if let Ok(id) = AudioDevice::new_output(None, None)
                    .and_then(|d| alvr_audio::get_windows_device_id(&d))
                {
                    unsafe {
                        crate::SetOpenvrProperty(
                            *alvr_common::HEAD_ID,
                            crate::openvr_props::to_ffi_openvr_prop(
                                alvr_session::OpenvrProperty::AudioDefaultPlaybackDeviceId(id),
                            ),
                        )
                    }
                }
            }
        })
    } else {
        thread::spawn(|| ())
    };

    let microphone_thread = if let Switch::Enabled(config) = settings.audio.microphone {
        #[allow(unused_variables)]
        let (sink, source) = AudioDevice::new_virtual_microphone_pair(
            Some(settings.audio.linux_backend),
            config.devices,
        )
        .to_con()?;

        #[cfg(windows)]
        if let Ok(id) = alvr_audio::get_windows_device_id(&source) {
            unsafe {
                crate::SetOpenvrProperty(
                    *alvr_common::HEAD_ID,
                    crate::openvr_props::to_ffi_openvr_prop(
                        alvr_session::OpenvrProperty::AudioDefaultRecordingDeviceId(id),
                    ),
                )
            }
        }

        thread::spawn(move || {
            alvr_common::show_err(alvr_audio::play_audio_loop(
                Arc::clone(&IS_STREAMING),
                sink,
                1,
                streaming_caps.microphone_sample_rate,
                config.buffering,
                microphone_receiver,
            ));
        })
    } else {
        thread::spawn(|| ())
    };

    let tracking_manager = Arc::new(Mutex::new(TrackingManager::new()));
    let hand_gesture_manager = Arc::new(Mutex::new(HandGestureManager::new()));

    let tracking_receive_thread = thread::spawn({
        let tracking_manager = Arc::clone(&tracking_manager);
        let hand_gesture_manager = Arc::clone(&hand_gesture_manager);

        let mut gestures_button_mapping_manager = if let Switch::Enabled(config) =
            &SERVER_DATA_MANAGER.read().settings().headset.controllers
        {
            if let Switch::Enabled(_) = &config.hand_tracking.use_gestures {
                Some(ButtonMappingManager::new_automatic(
                    &CONTROLLER_PROFILE_INFO
                        .get(&alvr_common::hash_string(QUEST_CONTROLLER_PROFILE_PATH))
                        .unwrap()
                        .button_set,
                    &config.button_mapping_config,
                ))
            } else {
                None
            }
        } else {
            None
        };

        move || {
            let mut face_tracking_sink =
                settings
                    .headset
                    .face_tracking
                    .into_option()
                    .and_then(|config| {
                        FaceTrackingSink::new(config.sink, settings.connection.osc_local_port).ok()
                    });

            let mut track_controllers = 0u32;
            if let Switch::Enabled(config) = &settings.headset.controllers {
                track_controllers = config.tracked.into();
            }

            while IS_STREAMING.value() {
                let data = match tracking_receiver.recv(STREAMING_RECV_TIMEOUT) {
                    Ok(tracking) => tracking,
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(ConnectionError::Other(_)) => return,
                };
                let Ok(tracking) = data.get_header() else {
                    return;
                };

                let data_manager_lock = SERVER_DATA_MANAGER.read();
                let config = &data_manager_lock.settings().headset;

                let mut tracking_manager_lock = tracking_manager.lock();

                let motions;
                let left_hand_skeleton;
                let right_hand_skeleton;
                {
                    motions = tracking_manager_lock.transform_motions(
                        config,
                        &tracking.device_motions,
                        [
                            tracking.hand_skeletons[0].is_some(),
                            tracking.hand_skeletons[1].is_some(),
                        ],
                    );

                    left_hand_skeleton = tracking.hand_skeletons[0]
                        .map(|s| tracking::to_openvr_hand_skeleton(config, *LEFT_HAND_ID, s));
                    right_hand_skeleton = tracking.hand_skeletons[1]
                        .map(|s| tracking::to_openvr_hand_skeleton(config, *RIGHT_HAND_ID, s));
                }

                // Note: using the raw unrecentered head
                let local_eye_gazes = tracking
                    .device_motions
                    .iter()
                    .find(|(id, _)| *id == *HEAD_ID)
                    .map(|(_, m)| tracking::to_local_eyes(m.pose, tracking.face_data.eye_gazes))
                    .unwrap_or_default();

                {
                    if data_manager_lock.settings().logging.log_tracking {
                        alvr_events::send_event(EventType::Tracking(Box::new(TrackingEvent {
                            head_motion: motions
                                .iter()
                                .find(|(id, _)| *id == *HEAD_ID)
                                .map(|(_, m)| *m),
                            controller_motions: [
                                motions
                                    .iter()
                                    .find(|(id, _)| *id == *LEFT_HAND_ID)
                                    .map(|(_, m)| *m),
                                motions
                                    .iter()
                                    .find(|(id, _)| *id == *RIGHT_HAND_ID)
                                    .map(|(_, m)| *m),
                            ],
                            hand_skeletons: [left_hand_skeleton, right_hand_skeleton],
                            eye_gazes: local_eye_gazes,
                            fb_face_expression: tracking.face_data.fb_face_expression.clone(),
                            htc_eye_expression: tracking.face_data.htc_eye_expression.clone(),
                            htc_lip_expression: tracking.face_data.htc_lip_expression.clone(),
                        })))
                    }
                }

                if let Some(sink) = &mut face_tracking_sink {
                    let mut face_data = tracking.face_data;
                    face_data.eye_gazes = local_eye_gazes;

                    sink.send_tracking(face_data);
                }

                let ffi_motions = motions
                    .into_iter()
                    .map(|(id, motion)| tracking::to_ffi_motion(id, motion))
                    .collect::<Vec<_>>();
                let ffi_left_hand_skeleton = left_hand_skeleton.map(tracking::to_ffi_skeleton);
                let ffi_right_hand_skeleton = right_hand_skeleton.map(tracking::to_ffi_skeleton);

                drop(tracking_manager_lock);

                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_tracking_received(tracking.target_timestamp);

                    // Handle hand gestures
                    if let Switch::Enabled(controllers_config) = &settings.headset.controllers {
                        if let Switch::Enabled(gestures_config) =
                            &controllers_config.hand_tracking.use_gestures
                        {
                            let mut hand_gesture_manager_lock = hand_gesture_manager.lock();

                            if tracking.hand_skeletons[0].is_some() {
                                trigger_hand_gesture_actions(
                                    gestures_button_mapping_manager.as_mut().unwrap(),
                                    *LEFT_HAND_ID,
                                    &hand_gesture_manager_lock.get_active_gestures(
                                        tracking.hand_skeletons[0].unwrap(),
                                        gestures_config.clone(),
                                        *LEFT_HAND_ID,
                                    ),
                                );
                            }
                            if tracking.hand_skeletons[1].is_some() {
                                trigger_hand_gesture_actions(
                                    gestures_button_mapping_manager.as_mut().unwrap(),
                                    *RIGHT_HAND_ID,
                                    &hand_gesture_manager_lock.get_active_gestures(
                                        tracking.hand_skeletons[1].unwrap(),
                                        gestures_config.clone(),
                                        *RIGHT_HAND_ID,
                                    ),
                                );
                            }

                            drop(hand_gesture_manager_lock);
                        }
                    }

                    let mut hand_skeletons_enabled = false;
                    if let Switch::Enabled(controllers) = &config.controllers {
                        hand_skeletons_enabled = controllers.hand_tracking.enable_skeleton;
                    }

                    unsafe {
                        crate::SetTracking(
                            tracking.target_timestamp.as_nanos() as _,
                            stats.tracker_pose_time_offset().as_secs_f32(),
                            ffi_motions.as_ptr(),
                            ffi_motions.len() as _,
                            if let Some(skeleton) = &ffi_left_hand_skeleton {
                                if hand_skeletons_enabled {
                                    skeleton
                                } else {
                                    ptr::null()
                                }
                            } else {
                                ptr::null()
                            },
                            if let Some(skeleton) = &ffi_right_hand_skeleton {
                                if hand_skeletons_enabled {
                                    skeleton
                                } else {
                                    ptr::null()
                                }
                            } else {
                                ptr::null()
                            },
                            track_controllers,
                        )
                    };
                }
            }
        }
    });

    let statistics_thread = thread::spawn(move || {
        while IS_STREAMING.value() {
            let data = match statics_receiver.recv(STREAMING_RECV_TIMEOUT) {
                Ok(stats) => stats,
                Err(ConnectionError::TryAgain(_)) => continue,
                Err(ConnectionError::Other(_)) => return,
            };
            let Ok(client_stats) = data.get_header() else {
                return;
            };

            if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                let timestamp = client_stats.target_timestamp;
                let decoder_latency = client_stats.video_decode;
                let network_latency = stats.report_statistics(client_stats);

                BITRATE_MANAGER.lock().report_frame_latencies(
                    &SERVER_DATA_MANAGER.read().settings().video.bitrate.mode,
                    timestamp,
                    network_latency,
                    decoder_latency,
                );
            }
        }
    });

    let control_sender = Arc::new(Mutex::new(control_sender));

    let keepalive_thread = thread::spawn({
        let control_sender = Arc::clone(&control_sender);
        let client_hostname = client_hostname.clone();
        move || {
            while IS_STREAMING.value() {
                if let Err(e) = control_sender.lock().send(&ServerControlPacket::KeepAlive) {
                    info!("Client disconnected. Cause: {e:?}");

                    SERVER_DATA_MANAGER.write().update_client_list(
                        client_hostname,
                        ClientListAction::SetConnectionState(ConnectionState::Disconnecting {
                            should_be_removed: false,
                        }),
                    );
                    if let Some(notifier) = &*DISCONNECT_CLIENT_NOTIFIER.lock() {
                        notifier.send(ClientDisconnectRequest::Disconnect).ok();
                    }

                    return;
                }

                thread::sleep(KEEPALIVE_INTERVAL);
            }
        }
    });

    let control_receive_thread = thread::spawn({
        let mut controller_button_mapping_manager = if let Switch::Enabled(config) =
            &SERVER_DATA_MANAGER.read().settings().headset.controllers
        {
            Some(ButtonMappingManager::new_automatic(
                &CONTROLLER_PROFILE_INFO
                    .get(&alvr_common::hash_string(QUEST_CONTROLLER_PROFILE_PATH))
                    .unwrap()
                    .button_set,
                &config.button_mapping_config,
            ))
        } else {
            None
        };
        // todo: gestures_button_mapping_manager...

        let control_sender = Arc::clone(&control_sender);
        let client_hostname = client_hostname.clone();
        move || {
            let mut disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            while IS_STREAMING.value() {
                let packet = match control_receiver.recv(STREAMING_RECV_TIMEOUT) {
                    Ok(packet) => packet,
                    Err(ConnectionError::TryAgain(_)) => {
                        if Instant::now() > disconnection_deadline {
                            info!("Client disconnected. Timeout");
                            break;
                        } else {
                            continue;
                        }
                    }
                    Err(e) => {
                        info!("Client disconnected. Cause: {e}");
                        break;
                    }
                };

                match packet {
                    ClientControlPacket::PlayspaceSync(packet) => {
                        if !settings.headset.tracking_ref_only {
                            let area = packet.unwrap_or(Vec2::new(2.0, 2.0));
                            unsafe { crate::SetChaperone(area.x, area.y) };

                            let data_manager_lock = SERVER_DATA_MANAGER.read();
                            let config = &data_manager_lock.settings().headset;
                            tracking_manager.lock().recenter(
                                config.position_recentering_mode,
                                config.rotation_recentering_mode,
                            );
                        }
                    }
                    ClientControlPacket::RequestIdr => {
                        if let Some(config) = DECODER_CONFIG.lock().clone() {
                            control_sender
                                .lock()
                                .send(&ServerControlPacket::InitializeDecoder(config))
                                .ok();
                        }
                        unsafe { crate::RequestIDR() }
                    }
                    ClientControlPacket::VideoErrorReport => {
                        // legacy endpoint. todo: remove
                        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                            stats.report_packet_loss();
                        }
                        unsafe { crate::VideoErrorReportReceive() };
                    }
                    ClientControlPacket::ViewsConfig(config) => unsafe {
                        crate::SetViewsConfig(FfiViewsConfig {
                            fov: [
                                FfiFov {
                                    left: config.fov[0].left,
                                    right: config.fov[0].right,
                                    up: config.fov[0].up,
                                    down: config.fov[0].down,
                                },
                                FfiFov {
                                    left: config.fov[1].left,
                                    right: config.fov[1].right,
                                    up: config.fov[1].up,
                                    down: config.fov[1].down,
                                },
                            ],
                            ipd_m: config.ipd_m,
                        });
                    },
                    ClientControlPacket::Battery(packet) => unsafe {
                        crate::SetBattery(packet.device_id, packet.gauge_value, packet.is_plugged);

                        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                            stats.report_battery(
                                packet.device_id,
                                packet.gauge_value,
                                packet.is_plugged,
                            );
                        }
                    },
                    ClientControlPacket::Buttons(entries) => {
                        {
                            let data_manager_lock = SERVER_DATA_MANAGER.read();
                            if data_manager_lock.settings().logging.log_button_presses {
                                alvr_events::send_event(EventType::Buttons(
                                    entries
                                        .iter()
                                        .map(|e| ButtonEvent {
                                            path: BUTTON_INFO
                                                .get(&e.path_id)
                                                .map(|info| info.path.to_owned())
                                                .unwrap_or_else(|| {
                                                    format!("Unknown (ID: {:#16x})", e.path_id)
                                                }),
                                            value: e.value,
                                        })
                                        .collect(),
                                ));
                            }
                        }

                        if let Some(manager) = &mut controller_button_mapping_manager {
                            for entry in entries {
                                manager.report_button(entry.path_id, entry.value);
                            }
                        };
                    }
                    ClientControlPacket::ActiveInteractionProfile {
                        device_id: _,
                        profile_id,
                    } => {
                        controller_button_mapping_manager =
                            if let (Switch::Enabled(config), Some(profile_info)) = (
                                &SERVER_DATA_MANAGER.read().settings().headset.controllers,
                                CONTROLLER_PROFILE_INFO.get(&profile_id),
                            ) {
                                Some(ButtonMappingManager::new_automatic(
                                    &profile_info.button_set,
                                    &config.button_mapping_config,
                                ))
                            } else {
                                None
                            };
                    }
                    ClientControlPacket::Log { level, message } => {
                        info!("Client {client_hostname}: [{level:?}] {message}")
                    }
                    _ => (),
                }

                disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            }

            SERVER_DATA_MANAGER.write().update_client_list(
                client_hostname,
                ClientListAction::SetConnectionState(ConnectionState::Disconnecting {
                    should_be_removed: false,
                }),
            );
            if let Some(notifier) = &*DISCONNECT_CLIENT_NOTIFIER.lock() {
                notifier.send(ClientDisconnectRequest::Disconnect).ok();
            }
        }
    });

    let stream_receive_thread = thread::spawn({
        let client_hostname = client_hostname.clone();
        move || {
            while IS_STREAMING.value() {
                match stream_socket.recv() {
                    Ok(()) => (),
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(e) => {
                        info!("Client disconnected. Cause: {e}");

                        SERVER_DATA_MANAGER.write().update_client_list(
                            client_hostname,
                            ClientListAction::SetConnectionState(ConnectionState::Disconnecting {
                                should_be_removed: false,
                            }),
                        );

                        if let Some(notifier) = &*DISCONNECT_CLIENT_NOTIFIER.lock() {
                            notifier.send(ClientDisconnectRequest::Disconnect).ok();
                        }

                        return;
                    }
                }
            }
        }
    });

    let lifecycle_check_thread = thread::spawn(|| {
        while IS_STREAMING.value() && SHOULD_CONNECT_TO_CLIENTS.value() {
            thread::sleep(STREAMING_RECV_TIMEOUT);
        }

        if let Some(notifier) = &*DISCONNECT_CLIENT_NOTIFIER.lock() {
            notifier.send(ClientDisconnectRequest::Disconnect).ok();
        }
    });

    {
        let on_connect_script = settings.connection.on_connect_script;

        if !on_connect_script.is_empty() {
            info!("Running on connect script (connect): {on_connect_script}");
            if let Err(e) = Command::new(&on_connect_script)
                .env("ACTION", "connect")
                .spawn()
            {
                warn!("Failed to run connect script: {e}");
            }
        }
    }

    if settings.capture.startup_video_recording {
        crate::create_recording_file();
    }

    unsafe { crate::InitializeStreaming() };

    SERVER_DATA_MANAGER.write().update_client_list(
        client_hostname,
        ClientListAction::SetConnectionState(ConnectionState::Streaming),
    );

    thread::spawn(move || {
        let _connection_drop_guard = _connection_drop_guard;

        let res = disconnect_receiver.recv();
        if matches!(res, Ok(ClientDisconnectRequest::ServerRestart)) {
            control_sender
                .lock()
                .send(&ServerControlPacket::Restarting)
                .ok();
        }

        // This requests shutdown from threads
        IS_STREAMING.set(false);
        *VIDEO_CHANNEL_SENDER.lock() = None;
        *HAPTICS_SENDER.lock() = None;

        *VIDEO_RECORDING_FILE.lock() = None;

        unsafe { crate::DeinitializeStreaming() };

        let on_disconnect_script = SERVER_DATA_MANAGER
            .read()
            .settings()
            .connection
            .on_disconnect_script
            .clone();
        if !on_disconnect_script.is_empty() {
            info!("Running on disconnect script (disconnect): {on_disconnect_script}");
            if let Err(e) = Command::new(&on_disconnect_script)
                .env("ACTION", "disconnect")
                .spawn()
            {
                warn!("Failed to run disconnect script: {e}");
            }
        }

        // ensure shutdown of threads
        video_send_thread.join().ok();
        game_audio_thread.join().ok();
        microphone_thread.join().ok();
        tracking_receive_thread.join().ok();
        statistics_thread.join().ok();
        control_receive_thread.join().ok();
        stream_receive_thread.join().ok();
        keepalive_thread.join().ok();
        lifecycle_check_thread.join().ok();
    });

    Ok(())
}

pub extern "C" fn send_video(timestamp_ns: u64, buffer_ptr: *mut u8, len: i32, is_idr: bool) {
    // start in the corrupts state, the client didn't receive the initial IDR yet.
    static STREAM_CORRUPTED: AtomicBool = AtomicBool::new(true);
    static LAST_IDR_INSTANT: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now()));

    if let Some(sender) = &*VIDEO_CHANNEL_SENDER.lock() {
        let buffer_size = len as usize;

        if is_idr {
            STREAM_CORRUPTED.store(false, Ordering::SeqCst);
        }

        if let Switch::Enabled(config) = &SERVER_DATA_MANAGER
            .read()
            .settings()
            .capture
            .rolling_video_files
        {
            if Instant::now() > *LAST_IDR_INSTANT.lock() + Duration::from_secs(config.duration_s) {
                unsafe { crate::RequestIDR() };

                if is_idr {
                    crate::create_recording_file();
                    *LAST_IDR_INSTANT.lock() = Instant::now();
                }
            }
        }

        let timestamp = Duration::from_nanos(timestamp_ns);

        let mut payload = vec![0; buffer_size];

        // use copy_nonoverlapping (aka memcpy) to avoid freeing memory allocated by C++
        unsafe {
            ptr::copy_nonoverlapping(buffer_ptr, payload.as_mut_ptr(), buffer_size);
        }

        if !STREAM_CORRUPTED.load(Ordering::SeqCst)
            || !SERVER_DATA_MANAGER
                .read()
                .settings()
                .connection
                .avoid_video_glitching
        {
            if let Some(sender) = &*VIDEO_MIRROR_SENDER.lock() {
                sender.send(payload.clone()).ok();
            }

            if let Some(file) = &mut *VIDEO_RECORDING_FILE.lock() {
                file.write_all(&payload).ok();
            }

            if matches!(
                sender.try_send(VideoPacket {
                    header: VideoPacketHeader { timestamp, is_idr },
                    payload,
                }),
                Err(TrySendError::Full(_))
            ) {
                STREAM_CORRUPTED.store(true, Ordering::SeqCst);
                unsafe { crate::RequestIDR() };
                warn!("Dropping video packet. Reason: Can't push to network");
            }
        } else {
            warn!("Dropping video packet. Reason: Waiting for IDR frame");
        }

        if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
            let encoder_latency =
                stats.report_frame_encoded(Duration::from_nanos(timestamp_ns), buffer_size);

            BITRATE_MANAGER
                .lock()
                .report_frame_encoded(timestamp, encoder_latency, buffer_size);
        }
    }
}

pub extern "C" fn send_haptics(device_id: u64, duration_s: f32, frequency: f32, amplitude: f32) {
    let haptics = Haptics {
        device_id,
        duration: Duration::from_secs_f32(f32::max(duration_s, 0.0)),
        frequency,
        amplitude,
    };

    let haptics_config = {
        let data_manager_lock = SERVER_DATA_MANAGER.read();

        if data_manager_lock.settings().logging.log_haptics {
            alvr_events::send_event(EventType::Haptics(HapticsEvent {
                path: DEVICE_ID_TO_PATH
                    .get(&haptics.device_id)
                    .map(|p| (*p).to_owned())
                    .unwrap_or_else(|| format!("Unknown (ID: {:#16x})", haptics.device_id)),
                duration: haptics.duration,
                frequency: haptics.frequency,
                amplitude: haptics.amplitude,
            }))
        }

        data_manager_lock
            .settings()
            .headset
            .controllers
            .as_option()
            .and_then(|c| c.haptics.as_option().cloned())
    };

    if let (Some(config), Some(sender)) = (haptics_config, &mut *HAPTICS_SENDER.lock()) {
        sender
            .send_header(&haptics::map_haptics(&config, haptics))
            .ok();
    }
}
