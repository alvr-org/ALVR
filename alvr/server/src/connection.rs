use crate::{
    bitrate::BitrateManager,
    body_tracking::BodyTrackingSink,
    face_tracking::FaceTrackingSink,
    hand_gestures::{trigger_hand_gesture_actions, HandGestureManager, HAND_GESTURE_BUTTON_SET},
    haptics,
    input_mapping::ButtonMappingManager,
    sockets::WelcomeSocket,
    statistics::StatisticsManager,
    tracking::{self, TrackingManager},
    FfiFov, FfiViewsConfig, VideoPacket, BITRATE_MANAGER, DECODER_CONFIG, LIFECYCLE_STATE,
    SERVER_DATA_MANAGER, STATISTICS_MANAGER, VIDEO_MIRROR_SENDER, VIDEO_RECORDING_FILE,
};
use alvr_audio::AudioDevice;
use alvr_common::{
    con_bail, debug, error,
    glam::{UVec2, Vec2},
    info,
    once_cell::sync::Lazy,
    parking_lot::{Condvar, Mutex},
    settings_schema::Switch,
    warn, AnyhowToCon, ConResult, ConnectionError, ConnectionState, LifecycleState, OptLazy,
    BUTTON_INFO, CONTROLLER_PROFILE_INFO, DEVICE_ID_TO_PATH, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID,
    QUEST_CONTROLLER_PROFILE_PATH,
};
use alvr_events::{ButtonEvent, EventType, HapticsEvent, TrackingEvent};
use alvr_packets::{
    ClientConnectionResult, ClientControlPacket, ClientListAction, ClientStatistics, Haptics,
    NegotiatedStreamingConfig, ServerControlPacket, Tracking, VideoPacketHeader, AUDIO, HAPTICS,
    STATISTICS, TRACKING, VIDEO,
};
use alvr_session::{
    BodyTrackingConfig, BodyTrackingSinkConfig, CodecType, ControllersEmulationMode, FrameSize,
    H264Profile, OpenvrConfig, SessionConfig,
};
use alvr_sockets::{
    PeerType, ProtoControlSocket, StreamSender, StreamSocketBuilder, KEEPALIVE_INTERVAL,
    KEEPALIVE_TIMEOUT,
};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    net::IpAddr,
    process::Command,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{RecvTimeoutError, SyncSender, TrySendError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const HANDSHAKE_ACTION_TIMEOUT: Duration = Duration::from_secs(2);
const STREAMING_RECV_TIMEOUT: Duration = Duration::from_millis(500);

const MAX_UNREAD_PACKETS: usize = 10; // Applies per stream

static VIDEO_CHANNEL_SENDER: OptLazy<SyncSender<VideoPacket>> = alvr_common::lazy_mut_none();
static HAPTICS_SENDER: OptLazy<StreamSender<Haptics>> = alvr_common::lazy_mut_none();
static CONNECTION_THREADS: Lazy<Mutex<Vec<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(vec![]));
pub static CLIENTS_TO_BE_REMOVED: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

fn is_streaming(client_hostname: &str) -> bool {
    SERVER_DATA_MANAGER
        .read()
        .client_list()
        .get(client_hostname)
        .map(|c| c.connection_state == ConnectionState::Streaming)
        .unwrap_or(false)
}

pub fn contruct_openvr_config(session: &SessionConfig) -> OpenvrConfig {
    let old_config = session.openvr_config.clone();
    let settings = session.to_settings();

    let mut controller_is_tracker = false;
    let mut _controller_profile = 0;
    let controllers_enabled = if let Switch::Enabled(config) = settings.headset.controllers {
        controller_is_tracker =
            matches!(config.emulation_mode, ControllersEmulationMode::ViveTracker);
        _controller_profile = match config.emulation_mode {
            ControllersEmulationMode::RiftSTouch => 0,
            ControllersEmulationMode::Quest2Touch => 1,
            ControllersEmulationMode::Quest3Plus => 2,
            ControllersEmulationMode::ValveIndex => 3,
            ControllersEmulationMode::ViveWand => 4,
            ControllersEmulationMode::ViveTracker => 5,
            ControllersEmulationMode::Custom { .. } => 6,
        };

        true
    } else {
        false
    };

    let body_tracking_vive_enabled =
        if let Switch::Enabled(config) = &settings.headset.body_tracking {
            matches!(config.sink, BodyTrackingSinkConfig::FakeViveTracker)
        } else {
            false
        };

    // Should be true if using full body tracking
    let body_tracking_has_legs = if let Switch::Enabled(config) = &settings.headset.body_tracking {
        if let Switch::Enabled(body_source_settings) = &config.sources.body_tracking_full_body_meta
        {
            body_source_settings.enable_full_body
        } else {
            false
        }
    } else {
        false
    };

    let mut foveation_center_size_x = 0.0;
    let mut foveation_center_size_y = 0.0;
    let mut foveation_center_shift_x = 0.0;
    let mut foveation_center_shift_y = 0.0;
    let mut foveation_edge_ratio_x = 0.0;
    let mut foveation_edge_ratio_y = 0.0;
    let enable_foveated_encoding = if let Switch::Enabled(config) = settings.video.foveated_encoding
    {
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
        minimum_idr_interval_ms: settings.connection.minimum_idr_interval_ms,
        adapter_index: settings.video.adapter_index,
        codec: settings.video.preferred_codec as _,
        h264_profile: settings.video.encoder_config.h264_profile as u32,
        rate_control_mode: settings.video.encoder_config.rate_control_mode as u32,
        filler_data: settings.video.encoder_config.filler_data,
        entropy_coding: settings.video.encoder_config.entropy_coding as u32,
        use_10bit_encoder: settings.video.encoder_config.use_10bit,
        use_full_range_encoding: settings.video.encoder_config.use_full_range,
        enable_pre_analysis: amf_controls.enable_pre_analysis,
        enable_vbaq: amf_controls.enable_vbaq,
        enable_hmqb: amf_controls.enable_hmqb,
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
        body_tracking_vive_enabled,
        body_tracking_has_legs,
        enable_foveated_encoding,
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
        linux_async_compute: settings.patches.linux_async_compute,
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
    let mut welcome_socket = match WelcomeSocket::new() {
        Ok(socket) => socket,
        Err(e) => {
            error!("Failed to create discovery socket: {e:?}");
            return;
        }
    };

    while *LIFECYCLE_STATE.write() != LifecycleState::ShuttingDown {
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
            let clients = match welcome_socket.recv_all() {
                Ok(clients) => clients,
                Err(e) => {
                    warn!("UDP handshake listening error: {e:?}");

                    thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
                    continue;
                }
            };

            if clients.is_empty() {
                thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
                continue;
            }

            for (client_hostname, client_ip) in clients {
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
                        .map(|c| c.trusted)
                        .unwrap_or(false)
                };

                // do not attempt connection if the client is already connected
                if trusted
                    && SERVER_DATA_MANAGER
                        .read()
                        .client_list()
                        .get(&client_hostname)
                        .map(|c| c.connection_state == ConnectionState::Disconnected)
                        .unwrap_or(false)
                {
                    if let Err(e) =
                        try_connect([(client_ip, client_hostname.clone())].into_iter().collect())
                    {
                        error!("Could not initiate connection for {client_hostname}: {e}");
                    }
                }

                thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
            }
        } else {
            thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
        }
    }

    // At this point, LIFECYCLE_STATE == ShuttingDown, so all threads are already terminating
    for thread in CONNECTION_THREADS.lock().drain(..) {
        thread.join().ok();
    }
}

fn try_connect(mut client_ips: HashMap<IpAddr, String>) -> ConResult {
    let (proto_socket, client_ip) = ProtoControlSocket::connect_to(
        Duration::from_secs(1),
        PeerType::AnyClient(client_ips.keys().cloned().collect()),
    )?;

    let Some(client_hostname) = client_ips.remove(&client_ip) else {
        con_bail!("unreachable");
    };

    CONNECTION_THREADS.lock().push(thread::spawn(move || {
        if let Err(e) = connection_pipeline(proto_socket, client_hostname.clone(), client_ip) {
            error!("Handshake error for {client_hostname}: {e}");
        }

        let mut clients_to_be_removed = CLIENTS_TO_BE_REMOVED.lock();

        let action = if clients_to_be_removed.contains(&client_hostname) {
            clients_to_be_removed.remove(&client_hostname);

            ClientListAction::RemoveEntry
        } else {
            ClientListAction::SetConnectionState(ConnectionState::Disconnected)
        };
        SERVER_DATA_MANAGER
            .write()
            .update_client_list(client_hostname, action);
    }));

    Ok(())
}

fn connection_pipeline(
    mut proto_socket: ProtoControlSocket,
    client_hostname: String,
    client_ip: IpAddr,
) -> ConResult {
    // This session lock will make sure settings cannot be changed while connecting and no other
    // client can connect (until handshake is finished)
    let mut server_data_lock = SERVER_DATA_MANAGER.write();

    server_data_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::SetConnectionState(ConnectionState::Connecting),
    );
    server_data_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::UpdateCurrentIp(Some(client_ip)),
    );

    let disconnect_notif = Arc::new(Condvar::new());

    let connection_result = match proto_socket.recv(HANDSHAKE_ACTION_TIMEOUT) {
        Ok(r) => r,
        Err(ConnectionError::TryAgain(e)) => {
            debug!("Failed to recive client connection packet. This is normal for USB connection.\n{e}");

            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let maybe_streaming_caps = if let ClientConnectionResult::ConnectionAccepted {
        client_protocol_id,
        display_name,
        streaming_capabilities,
        ..
    } = connection_result
    {
        server_data_lock.update_client_list(
            client_hostname.clone(),
            ClientListAction::SetDisplayName(display_name),
        );

        if client_protocol_id != alvr_common::protocol_id_u64() {
            warn!(
                "Trusted client is incompatible! Expected protocol ID: {}, found: {}",
                alvr_common::protocol_id_u64(),
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
        alvr_packets::decode_video_streaming_capabilities(&streaming_caps).to_con()?
    } else {
        con_bail!("Only streaming clients are supported for now");
    };

    let settings = server_data_lock.settings().clone();

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
        for rate in &streaming_caps.supported_refresh_rates {
            let diff = (*rate - settings.video.preferred_fps).abs();
            if diff < min_diff {
                best_match = *rate;
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

    let enable_foveated_encoding = if let Switch::Enabled(config) = settings.video.foveated_encoding
    {
        let enable = streaming_caps.supports_foveated_encoding || config.force_enable;

        if !enable {
            warn!("Foveated encoding is not supported by the client.");
        }

        enable
    } else {
        false
    };

    let encoder_profile = if settings.video.encoder_config.h264_profile == H264Profile::High {
        let profile = if streaming_caps.encoder_high_profile {
            H264Profile::High
        } else {
            H264Profile::Main
        };

        if profile != H264Profile::High {
            warn!("High profile encoding is not supported by the client.");
        }

        profile
    } else {
        settings.video.encoder_config.h264_profile
    };

    let enable_10_bits_encoding = if settings.video.encoder_config.use_10bit {
        let enable = streaming_caps.encoder_10_bits;

        if !enable {
            warn!("10 bits encoding is not supported by the client.");
        }

        enable
    } else {
        false
    };

    let codec = if settings.video.preferred_codec == CodecType::AV1 {
        let codec = if streaming_caps.encoder_av1 {
            CodecType::AV1
        } else {
            CodecType::Hevc
        };

        if codec != CodecType::AV1 {
            warn!("AV1 encoding is not supported by the client.");
        }

        codec
    } else {
        settings.video.preferred_codec
    };

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

    let stream_config_packet = alvr_packets::encode_stream_config(
        server_data_lock.session(),
        &NegotiatedStreamingConfig {
            view_resolution: stream_view_resolution,
            refresh_rate_hint: fps,
            game_audio_sample_rate,
            enable_foveated_encoding,
        },
    )
    .to_con()?;
    proto_socket.send(&stream_config_packet).to_con()?;

    let (mut control_sender, mut control_receiver) =
        proto_socket.split(STREAMING_RECV_TIMEOUT).to_con()?;

    let mut new_openvr_config = contruct_openvr_config(server_data_lock.session());
    new_openvr_config.eye_resolution_width = stream_view_resolution.x;
    new_openvr_config.eye_resolution_height = stream_view_resolution.y;
    new_openvr_config.target_eye_resolution_width = target_view_resolution.x;
    new_openvr_config.target_eye_resolution_height = target_view_resolution.y;
    new_openvr_config.refresh_rate = fps as _;
    new_openvr_config.enable_foveated_encoding = enable_foveated_encoding;
    new_openvr_config.h264_profile = encoder_profile as _;
    new_openvr_config.use_10bit_encoder = enable_10_bits_encoding;
    new_openvr_config.codec = codec as _;

    if server_data_lock.session().openvr_config != new_openvr_config {
        server_data_lock.session_mut().openvr_config = new_openvr_config;

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
        settings.connection.dscp,
        settings.connection.server_send_buffer_bytes,
        settings.connection.server_recv_buffer_bytes,
        settings.connection.packet_size as _,
    )?;

    let mut video_sender = stream_socket.request_stream(VIDEO);
    let game_audio_sender = stream_socket.request_stream(AUDIO);
    let mut microphone_receiver = stream_socket.subscribe_to_stream(AUDIO, MAX_UNREAD_PACKETS);
    let mut tracking_receiver =
        stream_socket.subscribe_to_stream::<Tracking>(TRACKING, MAX_UNREAD_PACKETS);
    let haptics_sender = stream_socket.request_stream(HAPTICS);
    let mut statics_receiver =
        stream_socket.subscribe_to_stream::<ClientStatistics>(STATISTICS, MAX_UNREAD_PACKETS);

    let (video_channel_sender, video_channel_receiver) =
        std::sync::mpsc::sync_channel(settings.connection.max_queued_server_video_frames);
    *VIDEO_CHANNEL_SENDER.lock() = Some(video_channel_sender);
    *HAPTICS_SENDER.lock() = Some(haptics_sender);

    let video_send_thread = thread::spawn({
        let client_hostname = client_hostname.clone();
        move || {
            while is_streaming(&client_hostname) {
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
        }
    });

    let game_audio_thread = if let Switch::Enabled(config) = settings.audio.game_audio {
        let client_hostname = client_hostname.clone();
        thread::spawn(move || {
            while is_streaming(&client_hostname) {
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
                    Arc::new({
                        let client_hostname = client_hostname.clone();
                        move || is_streaming(&client_hostname)
                    }),
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

        let client_hostname = client_hostname.clone();
        thread::spawn(move || {
            alvr_common::show_err(alvr_audio::play_audio_loop(
                {
                    let client_hostname = client_hostname.clone();
                    move || is_streaming(&client_hostname)
                },
                &sink,
                1,
                streaming_caps.microphone_sample_rate,
                config.buffering,
                &mut microphone_receiver,
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

        let mut gestures_button_mapping_manager =
            settings.headset.controllers.as_option().map(|config| {
                ButtonMappingManager::new_automatic(
                    &HAND_GESTURE_BUTTON_SET,
                    &config.button_mapping_config,
                )
            });

        let client_hostname = client_hostname.clone();
        move || {
            let mut face_tracking_sink =
                settings
                    .headset
                    .face_tracking
                    .into_option()
                    .and_then(|config| {
                        FaceTrackingSink::new(config.sink, settings.connection.osc_local_port).ok()
                    });

            let mut body_tracking_sink =
                settings
                    .headset
                    .body_tracking
                    .into_option()
                    .and_then(|config| {
                        BodyTrackingSink::new(config.sink, settings.connection.osc_local_port).ok()
                    });

            while is_streaming(&client_hostname) {
                let data = match tracking_receiver.recv(STREAMING_RECV_TIMEOUT) {
                    Ok(tracking) => tracking,
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(ConnectionError::Other(_)) => return,
                };
                let Ok(tracking) = data.get_header() else {
                    return;
                };

                let controllers_config = {
                    let data_lock = SERVER_DATA_MANAGER.read();
                    data_lock
                        .settings()
                        .headset
                        .controllers
                        .clone()
                        .into_option()
                };

                let track_controllers = controllers_config
                    .as_ref()
                    .map(|c| c.tracked)
                    .unwrap_or(false);

                let motions;
                let left_hand_skeleton;
                let right_hand_skeleton;
                {
                    let mut tracking_manager_lock = tracking_manager.lock();
                    let data_manager_lock = SERVER_DATA_MANAGER.read();
                    let headset_config = &data_manager_lock.settings().headset;

                    motions = tracking_manager_lock.transform_motions(
                        headset_config,
                        &tracking.device_motions,
                        [
                            tracking.hand_skeletons[0].is_some(),
                            tracking.hand_skeletons[1].is_some(),
                        ],
                    );

                    left_hand_skeleton = tracking.hand_skeletons[0].map(|s| {
                        tracking::to_openvr_hand_skeleton(headset_config, *HAND_LEFT_ID, s)
                    });
                    right_hand_skeleton = tracking.hand_skeletons[1].map(|s| {
                        tracking::to_openvr_hand_skeleton(headset_config, *HAND_RIGHT_ID, s)
                    });
                }

                // Note: using the raw unrecentered head
                let local_eye_gazes = tracking
                    .device_motions
                    .iter()
                    .find(|(id, _)| *id == *HEAD_ID)
                    .map(|(_, m)| tracking::to_local_eyes(m.pose, tracking.face_data.eye_gazes))
                    .unwrap_or_default();

                {
                    let data_manager_lock = SERVER_DATA_MANAGER.read();
                    if data_manager_lock.settings().logging.log_tracking {
                        alvr_events::send_event(EventType::Tracking(Box::new(TrackingEvent {
                            device_motions: motions
                                .iter()
                                .filter_map(|(id, motion)| {
                                    Some(((*DEVICE_ID_TO_PATH.get(id)?).into(), *motion))
                                })
                                .collect(),
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

                let track_body = {
                    let data_manager_lock = SERVER_DATA_MANAGER.read();
                    matches!(
                        data_manager_lock.settings().headset.body_tracking,
                        Switch::Enabled(BodyTrackingConfig { tracked: true, .. })
                    )
                };

                if track_body {
                    if let Some(sink) = &mut body_tracking_sink {
                        let tracking_manager_lock = tracking_manager.lock();
                        sink.send_tracking(&tracking.device_motions, &tracking_manager_lock);
                    }
                }

                let ffi_motions = motions
                    .into_iter()
                    .map(|(id, motion)| tracking::to_ffi_motion(id, motion))
                    .collect::<Vec<_>>();

                let ffi_body_trackers: Option<Vec<crate::FfiBodyTracker>> = {
                    let tracking_manager_lock = tracking_manager.lock();
                    tracking::to_ffi_body_trackers(
                        &tracking.device_motions,
                        &tracking_manager_lock,
                        track_body,
                    )
                };

                let enable_skeleton = controllers_config
                    .as_ref()
                    .map(|c| c.enable_skeleton)
                    .unwrap_or(false);
                let ffi_left_hand_skeleton = enable_skeleton
                    .then_some(left_hand_skeleton)
                    .flatten()
                    .map(tracking::to_ffi_skeleton);
                let ffi_right_hand_skeleton = enable_skeleton
                    .then_some(right_hand_skeleton)
                    .flatten()
                    .map(tracking::to_ffi_skeleton);

                // Handle hand gestures
                if let (Some(gestures_config), Some(gestures_button_mapping_manager)) = (
                    controllers_config
                        .as_ref()
                        .and_then(|c| c.gestures.as_option()),
                    &mut gestures_button_mapping_manager,
                ) {
                    let mut hand_gesture_manager_lock = hand_gesture_manager.lock();

                    if let Some(hand_skeleton) = tracking.hand_skeletons[0] {
                        trigger_hand_gesture_actions(
                            gestures_button_mapping_manager,
                            *HAND_LEFT_ID,
                            &hand_gesture_manager_lock.get_active_gestures(
                                hand_skeleton,
                                gestures_config,
                                *HAND_LEFT_ID,
                            ),
                            gestures_config.only_touch,
                        );
                    }
                    if let Some(hand_skeleton) = tracking.hand_skeletons[1] {
                        trigger_hand_gesture_actions(
                            gestures_button_mapping_manager,
                            *HAND_RIGHT_ID,
                            &hand_gesture_manager_lock.get_active_gestures(
                                hand_skeleton,
                                gestures_config,
                                *HAND_RIGHT_ID,
                            ),
                            gestures_config.only_touch,
                        );
                    }
                }

                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_tracking_received(tracking.target_timestamp);

                    unsafe {
                        crate::SetTracking(
                            tracking.target_timestamp.as_nanos() as _,
                            stats.tracker_pose_time_offset().as_secs_f32(),
                            ffi_motions.as_ptr(),
                            ffi_motions.len() as _,
                            if let Some(skeleton) = &ffi_left_hand_skeleton {
                                skeleton
                            } else {
                                ptr::null()
                            },
                            if let Some(skeleton) = &ffi_right_hand_skeleton {
                                skeleton
                            } else {
                                ptr::null()
                            },
                            track_controllers.into(),
                            if let Some(body_trackers) = &ffi_body_trackers {
                                body_trackers.as_ptr()
                            } else {
                                ptr::null()
                            },
                            if let Some(body_trackers) = &ffi_body_trackers {
                                body_trackers.len() as _
                            } else {
                                0
                            },
                        )
                    };
                }
            }
        }
    });

    let statistics_thread = thread::spawn({
        let client_hostname = client_hostname.clone();
        move || {
            while is_streaming(&client_hostname) {
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

                    let server_data_lock = SERVER_DATA_MANAGER.read();
                    BITRATE_MANAGER.lock().report_frame_latencies(
                        &server_data_lock.settings().video.bitrate.mode,
                        timestamp,
                        network_latency,
                        decoder_latency,
                    );
                }
            }
        }
    });

    let control_sender = Arc::new(Mutex::new(control_sender));

    let keepalive_thread = thread::spawn({
        let control_sender = Arc::clone(&control_sender);
        let disconnect_notif = Arc::clone(&disconnect_notif);
        let client_hostname = client_hostname.clone();
        move || {
            while is_streaming(&client_hostname) {
                if let Err(e) = control_sender.lock().send(&ServerControlPacket::KeepAlive) {
                    info!("Client disconnected. Cause: {e:?}");

                    disconnect_notif.notify_one();

                    return;
                }

                thread::sleep(KEEPALIVE_INTERVAL);
            }
        }
    });

    let control_receive_thread = thread::spawn({
        let mut controller_button_mapping_manager = server_data_lock
            .settings()
            .headset
            .controllers
            .as_option()
            .map(|config| {
                if let Some(mappings) = &config.button_mappings {
                    ButtonMappingManager::new_manual(mappings)
                } else {
                    ButtonMappingManager::new_automatic(
                        &CONTROLLER_PROFILE_INFO
                            .get(&alvr_common::hash_string(QUEST_CONTROLLER_PROFILE_PATH))
                            .unwrap()
                            .button_set,
                        &config.button_mapping_config,
                    )
                }
            });

        let disconnect_notif = Arc::clone(&disconnect_notif);
        let control_sender = Arc::clone(&control_sender);
        let client_hostname = client_hostname.clone();
        move || {
            unsafe { crate::InitOpenvrClient() };

            let mut disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            while is_streaming(&client_hostname) {
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
                            let data_manager_lock = SERVER_DATA_MANAGER.read();
                            let config = &data_manager_lock.settings().headset;
                            tracking_manager.lock().recenter(
                                config.position_recentering_mode,
                                config.rotation_recentering_mode,
                            );

                            let area = packet.unwrap_or(Vec2::new(2.0, 2.0));
                            unsafe { crate::SetChaperoneArea(area.x, area.y) };
                        }
                    }
                    ClientControlPacket::RequestIdr => {
                        if let Some(config) = DECODER_CONFIG.lock().clone() {
                            control_sender
                                .lock()
                                .send(&ServerControlPacket::DecoderConfig(config))
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
                                if let Some(mappings) = &config.button_mappings {
                                    Some(ButtonMappingManager::new_manual(mappings))
                                } else {
                                    Some(ButtonMappingManager::new_automatic(
                                        &profile_info.button_set,
                                        &config.button_mapping_config,
                                    ))
                                }
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
            unsafe { crate::ShutdownOpenvrClient() };

            disconnect_notif.notify_one()
        }
    });

    let stream_receive_thread = thread::spawn({
        let disconnect_notif = Arc::clone(&disconnect_notif);
        let client_hostname = client_hostname.clone();
        move || {
            while is_streaming(&client_hostname) {
                match stream_socket.recv() {
                    Ok(()) => (),
                    Err(ConnectionError::TryAgain(_)) => continue,
                    Err(e) => {
                        info!("Client disconnected. Cause: {e}");

                        disconnect_notif.notify_one();

                        return;
                    }
                }
            }
        }
    });

    let lifecycle_check_thread = thread::spawn({
        let disconnect_notif = Arc::clone(&disconnect_notif);
        let client_hostname = client_hostname.clone();
        move || {
            while SERVER_DATA_MANAGER
                .read()
                .client_list()
                .get(&client_hostname)
                .map(|c| c.connection_state == ConnectionState::Streaming)
                .unwrap_or(false)
                && *LIFECYCLE_STATE.read() == LifecycleState::Resumed
            {
                thread::sleep(STREAMING_RECV_TIMEOUT);
            }

            disconnect_notif.notify_one()
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
        crate::create_recording_file(server_data_lock.settings());
    }

    unsafe { crate::InitializeStreaming() };

    server_data_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::SetConnectionState(ConnectionState::Streaming),
    );

    alvr_common::wait_rwlock(&disconnect_notif, &mut server_data_lock);

    // This requests shutdown from threads
    *VIDEO_CHANNEL_SENDER.lock() = None;
    *HAPTICS_SENDER.lock() = None;

    *VIDEO_RECORDING_FILE.lock() = None;

    unsafe { crate::DeinitializeStreaming() };

    server_data_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::SetConnectionState(ConnectionState::Disconnecting),
    );

    let on_disconnect_script = server_data_lock
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

    // Allow threads to shutdown correctly
    drop(server_data_lock);

    // Ensure shutdown of threads
    video_send_thread.join().ok();
    game_audio_thread.join().ok();
    microphone_thread.join().ok();
    tracking_receive_thread.join().ok();
    statistics_thread.join().ok();
    control_receive_thread.join().ok();
    stream_receive_thread.join().ok();
    keepalive_thread.join().ok();
    lifecycle_check_thread.join().ok();

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
                    crate::create_recording_file(SERVER_DATA_MANAGER.read().settings());
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
