use crate::{
    bitrate::BitrateManager,
    hand_gestures::HandGestureManager,
    input_mapping::ButtonMappingManager,
    sockets::WelcomeSocket,
    statistics::StatisticsManager,
    tracking::{self, TrackingManager},
    ConnectionContext, ServerCoreEvent, ViewsConfig, FILESYSTEM_LAYOUT, SESSION_MANAGER,
};
use alvr_adb::{WiredConnection, WiredConnectionStatus};
use alvr_common::{
    con_bail, dbg_connection, debug, error,
    glam::{Quat, UVec2, Vec2, Vec3},
    info,
    parking_lot::{Condvar, Mutex, RwLock},
    settings_schema::Switch,
    warn, AnyhowToCon, ConResult, ConnectionError, ConnectionState, LifecycleState, Pose,
    BUTTON_INFO, CONTROLLER_PROFILE_INFO, QUEST_CONTROLLER_PROFILE_PATH,
};
use alvr_events::{AdbEvent, ButtonEvent, EventType};
use alvr_packets::{
    BatteryInfo, ClientConnectionResult, ClientControlPacket, ClientListAction, ClientStatistics,
    NegotiatedStreamingConfig, ReservedClientControlPacket, ServerControlPacket, Tracking,
    VideoPacketHeader, AUDIO, HAPTICS, STATISTICS, TRACKING, VIDEO,
};
use alvr_session::{
    BodyTrackingSinkConfig, CodecType, ControllersEmulationMode, FrameSize, H264Profile,
    OpenvrConfig, SessionConfig, SocketProtocol,
};
use alvr_sockets::{
    PeerType, ProtoControlSocket, StreamSocketBuilder, CONTROL_PORT, KEEPALIVE_INTERVAL,
    KEEPALIVE_TIMEOUT, WIRED_CLIENT_HOSTNAME,
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    process::Command,
    sync::{mpsc::RecvTimeoutError, Arc},
    thread,
    time::{Duration, Instant},
};

const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const HANDSHAKE_ACTION_TIMEOUT: Duration = Duration::from_secs(2);
pub const STREAMING_RECV_TIMEOUT: Duration = Duration::from_millis(500);

const MAX_UNREAD_PACKETS: usize = 10; // Applies per stream

pub struct VideoPacket {
    pub header: VideoPacketHeader,
    pub payload: Vec<u8>,
}

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

fn is_streaming(client_hostname: &str) -> bool {
    SESSION_MANAGER
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
    let mut use_separate_hand_trackers = false;
    let controllers_enabled = if let Switch::Enabled(config) = settings.headset.controllers {
        controller_is_tracker =
            matches!(config.emulation_mode, ControllersEmulationMode::ViveTracker);
        // These numbers don't mean anything, they're just for triggering SteamVR resets.
        // Gaps are included in the numbering to make adding other controllers
        // a bit easier though.
        _controller_profile = match config.emulation_mode {
            ControllersEmulationMode::RiftSTouch => 0,
            ControllersEmulationMode::Quest2Touch => 1,
            ControllersEmulationMode::Quest3Plus => 2,
            ControllersEmulationMode::QuestPro => 3,
            ControllersEmulationMode::ValveIndex => 20,
            ControllersEmulationMode::ViveWand => 40,
            ControllersEmulationMode::ViveTracker => 41,
            ControllersEmulationMode::Custom { .. } => 500,
        };
        use_separate_hand_trackers = config
            .hand_skeleton
            .as_option()
            .map(|c| c.steamvr_input_2_0)
            .unwrap_or(false);

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
    let body_tracking_has_legs = settings
        .headset
        .body_tracking
        .as_option()
        .and_then(|c| c.sources.body_tracking_fb.as_option().cloned())
        .map(|c| c.full_body)
        .unwrap_or(false);

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
        force_hdr_srgb_correction: settings.video.encoder_config.force_hdr_srgb_correction,
        clamp_hdr_extended_range: settings.video.encoder_config.clamp_hdr_extended_range,
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
        linux_async_compute: settings.extra.patches.linux_async_compute,
        linux_async_reprojection: settings.extra.patches.linux_async_reprojection,
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
        capture_frame_dir: settings.extra.capture.capture_frame_dir,
        amd_bitrate_corruption_fix: settings.video.bitrate.image_corruption_fix,
        use_separate_hand_trackers,
        _controller_profile,
        _server_impl_debug: settings.extra.logging.debug_groups.server_impl,
        _client_impl_debug: settings.extra.logging.debug_groups.client_impl,
        _server_core_debug: settings.extra.logging.debug_groups.server_core,
        _client_core_debug: settings.extra.logging.debug_groups.client_core,
        _conncection_debug: settings.extra.logging.debug_groups.connection,
        _sockets_debug: settings.extra.logging.debug_groups.sockets,
        _server_gfx_debug: settings.extra.logging.debug_groups.server_gfx,
        _client_gfx_debug: settings.extra.logging.debug_groups.client_gfx,
        _encoder_debug: settings.extra.logging.debug_groups.encoder,
        _decoder_debug: settings.extra.logging.debug_groups.decoder,
        ..old_config
    }
}

// Alternate connection trials with manual IPs and clients discovered on the local network
pub fn handshake_loop(ctx: Arc<ConnectionContext>, lifecycle_state: Arc<RwLock<LifecycleState>>) {
    dbg_connection!("handshake_loop: Begin");

    let mut welcome_socket = match WelcomeSocket::new() {
        Ok(socket) => socket,
        Err(e) => {
            error!("Failed to create discovery socket: {e:?}");
            return;
        }
    };

    let mut wired_connection = None;

    while *lifecycle_state.read() != LifecycleState::ShuttingDown {
        dbg_connection!("handshake_loop: Try connect to wired device");

        let mut wired_client_ips = HashMap::new();
        if let Some((client_hostname, _)) =
            SESSION_MANAGER
                .read()
                .client_list()
                .iter()
                .find(|(hostname, info)| {
                    info.connection_state == ConnectionState::Disconnected
                        && hostname.as_str() == WIRED_CLIENT_HOSTNAME
                })
        {
            // Make sure the wired connection is created once and kept alive
            let wired_connection = if let Some(connection) = &wired_connection {
                connection
            } else {
                let connection = match WiredConnection::new(
                    FILESYSTEM_LAYOUT.get().unwrap(),
                    |downloaded, maybe_total| {
                        if let Some(total) = maybe_total {
                            alvr_events::send_event(EventType::Adb(AdbEvent {
                                download_progress: downloaded as f32 / total as f32,
                            }));
                        };
                    },
                ) {
                    Ok(connection) => connection,
                    Err(e) => {
                        error!("{e:?}");
                        thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
                        continue;
                    }
                };

                wired_connection = Some(connection);

                wired_connection.as_ref().unwrap()
            };

            let stream_port;
            let client_type;
            let client_autolaunch;
            {
                let session_manager_lock = SESSION_MANAGER.read();
                let connection = &session_manager_lock.settings().connection;
                stream_port = connection.stream_port;
                client_type = connection.wired_client_type.clone();
                client_autolaunch = connection.wired_client_autolaunch;
            }

            let status = match wired_connection.setup(
                CONTROL_PORT,
                stream_port,
                &client_type,
                client_autolaunch,
            ) {
                Ok(status) => status,
                Err(e) => {
                    error!("{e:?}");
                    thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
                    continue;
                }
            };

            if let WiredConnectionStatus::NotReady(m) = status {
                dbg_connection!("handshake_loop: Wired connection not ready: {m}");
                thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
                continue;
            }

            let client_ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
            wired_client_ips.insert(client_ip, client_hostname.to_owned());
        }

        if !wired_client_ips.is_empty()
            && try_connect(
                Arc::clone(&ctx),
                Arc::clone(&lifecycle_state),
                wired_client_ips,
            )
            .is_ok()
        {
            thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
            continue;
        }

        dbg_connection!("handshake_loop: Try connect to manual IPs");

        let available_manual_client_ips = {
            let mut manual_client_ips = HashMap::new();
            for (hostname, connection_info) in
                SESSION_MANAGER
                    .read()
                    .client_list()
                    .iter()
                    .filter(|(hostname, info)| {
                        info.connection_state == ConnectionState::Disconnected
                            && hostname.as_str() != WIRED_CLIENT_HOSTNAME
                    })
            {
                for ip in &connection_info.manual_ips {
                    manual_client_ips.insert(*ip, hostname.clone());
                }
            }
            manual_client_ips
        };

        if !available_manual_client_ips.is_empty()
            && try_connect(
                Arc::clone(&ctx),
                Arc::clone(&lifecycle_state),
                available_manual_client_ips,
            )
            .is_ok()
        {
            thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
            continue;
        }

        let discovery_config = SESSION_MANAGER
            .read()
            .settings()
            .connection
            .client_discovery
            .clone();
        if let Switch::Enabled(config) = discovery_config {
            dbg_connection!("handshake_loop: Discovering clients");

            let clients = match welcome_socket.recv_all() {
                Ok(clients) => clients,
                Err(e) => {
                    warn!("mDNS listening error: {e:?}");

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
                    let mut session_manager = SESSION_MANAGER.write();

                    session_manager.update_client_list(
                        client_hostname.clone(),
                        ClientListAction::AddIfMissing {
                            trusted: false,
                            manual_ips: vec![],
                        },
                    );

                    if config.auto_trust_clients {
                        session_manager
                            .update_client_list(client_hostname.clone(), ClientListAction::Trust);
                    }

                    session_manager
                        .client_list()
                        .get(&client_hostname)
                        .map(|c| c.trusted)
                        .unwrap_or(false)
                };

                // do not attempt connection if the client is already connected
                if trusted
                    && SESSION_MANAGER
                        .read()
                        .client_list()
                        .get(&client_hostname)
                        .map(|c| c.connection_state == ConnectionState::Disconnected)
                        .unwrap_or(false)
                {
                    if let Err(e) = try_connect(
                        Arc::clone(&ctx),
                        Arc::clone(&lifecycle_state),
                        [(client_ip, client_hostname.clone())].into_iter().collect(),
                    ) {
                        error!("Could not initiate connection for {client_hostname}: {e}");
                    }
                }

                thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
            }
        } else {
            thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
        }
    }

    alvr_common::dbg_connection!("handshake_loop: Joining connection threads");

    // At this point, LIFECYCLE_STATE == ShuttingDown, so all threads are already terminating
    for thread in ctx.connection_threads.lock().drain(..) {
        thread.join().ok();
    }

    alvr_common::dbg_connection!("handshake_loop: End");
}

fn try_connect(
    ctx: Arc<ConnectionContext>,
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    mut client_ips: HashMap<IpAddr, String>,
) -> ConResult {
    dbg_connection!("try_connect: Finding client and creating control socket");

    let (proto_socket, client_ip) = ProtoControlSocket::connect_to(
        Duration::from_secs(1),
        PeerType::AnyClient(client_ips.keys().cloned().collect()),
    )?;

    let Some(client_hostname) = client_ips.remove(&client_ip) else {
        con_bail!("unreachable");
    };

    dbg_connection!("try_connect: Pushing new client connection thread");

    ctx.connection_threads.lock().push(thread::spawn({
        let ctx = Arc::clone(&ctx);
        move || {
            if let Err(e) = connection_pipeline(
                Arc::clone(&ctx),
                lifecycle_state,
                proto_socket,
                client_hostname.clone(),
                client_ip,
            ) {
                error!("Handshake error for {client_hostname}: {e}");
            }

            let mut clients_to_be_removed = ctx.clients_to_be_removed.lock();

            let action = if clients_to_be_removed.contains(&client_hostname) {
                clients_to_be_removed.remove(&client_hostname);

                ClientListAction::RemoveEntry
            } else {
                ClientListAction::SetConnectionState(ConnectionState::Disconnected)
            };
            SESSION_MANAGER
                .write()
                .update_client_list(client_hostname, action);
        }
    }));

    Ok(())
}

fn connection_pipeline(
    ctx: Arc<ConnectionContext>,
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    mut proto_socket: ProtoControlSocket,
    client_hostname: String,
    client_ip: IpAddr,
) -> ConResult {
    dbg_connection!("connection_pipeline: Begin");

    // This session lock will make sure settings and client list cannot be changed while connecting
    // to thos client, no other client can connect until handshake is finished. It will then be
    // temporarily relocked while shutting down the threads.
    let mut session_manager_lock = SESSION_MANAGER.write();

    dbg_connection!("connection_pipeline: Setting client state in session");
    session_manager_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::SetConnectionState(ConnectionState::Connecting),
    );
    session_manager_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::UpdateCurrentIp(Some(client_ip)),
    );

    let disconnect_notif = Arc::new(Condvar::new());

    dbg_connection!("connection_pipeline: Getting client status packet");
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
        session_manager_lock.update_client_list(
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

    dbg_connection!("connection_pipeline: setting up negotiated streaming config");

    let initial_settings = session_manager_lock.settings().clone();

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
        initial_settings.video.transcoding_view_resolution.clone(),
        streaming_caps.default_view_resolution,
    );

    let target_view_resolution = get_view_res(
        initial_settings
            .video
            .emulated_headset_view_resolution
            .clone(),
        streaming_caps.default_view_resolution,
    );

    let fps = {
        let mut best_match = 0_f32;
        let mut min_diff = f32::MAX;
        for rate in &streaming_caps.supported_refresh_rates {
            let diff = (*rate - initial_settings.video.preferred_fps).abs();
            if diff < min_diff {
                best_match = *rate;
                min_diff = diff;
            }
        }
        best_match
    };

    if !streaming_caps
        .supported_refresh_rates
        .contains(&initial_settings.video.preferred_fps)
    {
        warn!("Chosen refresh rate not supported. Using {fps}Hz");
    }

    let enable_foveated_encoding =
        if let Switch::Enabled(config) = &initial_settings.video.foveated_encoding {
            let enable = streaming_caps.supports_foveated_encoding || config.force_enable;

            if !enable {
                warn!("Foveated encoding is not supported by the client.");
            }

            enable
        } else {
            false
        };

    let encoder_profile = if initial_settings.video.encoder_config.h264_profile == H264Profile::High
    {
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
        initial_settings.video.encoder_config.h264_profile
    };

    let mut enable_10_bits_encoding = if initial_settings
        .video
        .encoder_config
        .server_overrides_use_10bit
    {
        initial_settings.video.encoder_config.use_10bit
    } else {
        streaming_caps.prefer_10bit
    };

    if enable_10_bits_encoding && !streaming_caps.encoder_10_bits {
        warn!("10 bits encoding is not supported by the client.");
        enable_10_bits_encoding = false
    }

    let use_full_range = if initial_settings
        .video
        .encoder_config
        .server_overrides_use_full_range
    {
        initial_settings.video.encoder_config.use_full_range
    } else {
        streaming_caps.prefer_full_range
    };

    let enable_hdr = if initial_settings
        .video
        .encoder_config
        .server_overrides_enable_hdr
    {
        initial_settings.video.encoder_config.enable_hdr
    } else {
        streaming_caps.prefer_hdr
    };

    let encoding_gamma = if initial_settings
        .video
        .encoder_config
        .server_overrides_encoding_gamma
    {
        initial_settings.video.encoder_config.encoding_gamma
    } else {
        streaming_caps.preferred_encoding_gamma
    };

    let codec = if initial_settings.video.preferred_codec == CodecType::AV1 {
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
        initial_settings.video.preferred_codec
    };

    #[cfg_attr(target_os = "linux", allow(unused_variables))]
    let game_audio_sample_rate = if let Switch::Enabled(game_audio_config) =
        &initial_settings.audio.game_audio
    {
        #[cfg(not(target_os = "linux"))]
        {
            let game_audio_device =
                alvr_audio::AudioDevice::new_output(game_audio_config.device.as_ref()).to_con()?;
            if let Switch::Enabled(microphone_config) = &initial_settings.audio.microphone {
                let (sink, source) = alvr_audio::AudioDevice::new_virtual_microphone_pair(
                    microphone_config.devices.clone(),
                )
                .to_con()?;
                if matches!(
                    microphone_config.devices,
                    alvr_session::MicrophoneDevicesConfig::VBCable
                ) {
                    // VoiceMeeter and Custom devices may have arbitrary internal routing.
                    // Therefore, we cannot detect the loopback issue without knowing the routing.
                    if alvr_audio::is_same_device(&game_audio_device, &sink)
                        || alvr_audio::is_same_device(&game_audio_device, &source)
                    {
                        con_bail!("Game audio and microphone cannot point to the same device!");
                    }
                }
                // else:
                // Stream played via VA-CABLE-X will be directly routed to VA-CABLE-X's virtual microphone.
                // Game audio will loop back to the game microphone if they are set to the same VA-CABLE-X device.
            }

            game_audio_device.input_sample_rate().to_con()?
        }
        #[cfg(target_os = "linux")]
        44100
    } else {
        0
    };

    let wired = client_ip.is_loopback();

    dbg_connection!("connection_pipeline: send streaming config");
    let stream_config_packet = alvr_packets::encode_stream_config(
        session_manager_lock.session(),
        &NegotiatedStreamingConfig {
            view_resolution: stream_view_resolution,
            refresh_rate_hint: fps,
            game_audio_sample_rate,
            enable_foveated_encoding,
            use_multimodal_protocol: streaming_caps.multimodal_protocol,
            encoding_gamma,
            enable_hdr,
            wired,
        },
    )
    .to_con()?;
    proto_socket.send(&stream_config_packet).to_con()?;

    let (mut control_sender, mut control_receiver) =
        proto_socket.split(STREAMING_RECV_TIMEOUT).to_con()?;

    let mut new_openvr_config = contruct_openvr_config(session_manager_lock.session());
    new_openvr_config.eye_resolution_width = stream_view_resolution.x;
    new_openvr_config.eye_resolution_height = stream_view_resolution.y;
    new_openvr_config.target_eye_resolution_width = target_view_resolution.x;
    new_openvr_config.target_eye_resolution_height = target_view_resolution.y;
    new_openvr_config.refresh_rate = fps as _;
    new_openvr_config.enable_foveated_encoding = enable_foveated_encoding;
    new_openvr_config.h264_profile = encoder_profile as _;
    new_openvr_config.use_10bit_encoder = enable_10_bits_encoding;
    new_openvr_config.use_full_range_encoding = use_full_range;
    new_openvr_config.enable_hdr = enable_hdr;
    new_openvr_config.encoding_gamma = encoding_gamma;
    new_openvr_config.codec = codec as _;

    if session_manager_lock.session().openvr_config != new_openvr_config {
        session_manager_lock.session_mut().openvr_config = new_openvr_config;

        control_sender.send(&ServerControlPacket::Restarting).ok();

        crate::notify_restart_driver();
    }

    dbg_connection!("connection_pipeline: Send StartStream packet");
    control_sender
        .send(&ServerControlPacket::StartStream)
        .to_con()?;

    let signal = control_receiver.recv(HANDSHAKE_ACTION_TIMEOUT)?;
    if !matches!(signal, ClientControlPacket::StreamReady) {
        con_bail!("Got unexpected packet waiting for stream ack");
    }
    dbg_connection!("connection_pipeline: Got StreamReady packet");

    *ctx.statistics_manager.write() = Some(StatisticsManager::new(
        initial_settings.connection.statistics_history_size,
        Duration::from_secs_f32(1.0 / fps),
        if let Switch::Enabled(config) = &initial_settings.headset.controllers {
            config.steamvr_pipeline_frames
        } else {
            0.0
        },
    ));

    *ctx.bitrate_manager.lock() =
        BitrateManager::new(initial_settings.video.bitrate.history_size, fps);

    let stream_protocol = if wired {
        SocketProtocol::Tcp
    } else {
        initial_settings.connection.stream_protocol
    };

    dbg_connection!("connection_pipeline: StreamSocket connect_to_client");
    let mut stream_socket = StreamSocketBuilder::connect_to_client(
        HANDSHAKE_ACTION_TIMEOUT,
        client_ip,
        initial_settings.connection.stream_port,
        stream_protocol,
        initial_settings.connection.dscp,
        initial_settings.connection.server_send_buffer_bytes,
        initial_settings.connection.server_recv_buffer_bytes,
        initial_settings.connection.packet_size as _,
    )?;

    let mut video_sender = stream_socket.request_stream(VIDEO);
    let game_audio_sender: alvr_sockets::StreamSender<()> = stream_socket.request_stream(AUDIO);
    let mut microphone_receiver: alvr_sockets::StreamReceiver<()> =
        stream_socket.subscribe_to_stream(AUDIO, MAX_UNREAD_PACKETS);
    let tracking_receiver =
        stream_socket.subscribe_to_stream::<Tracking>(TRACKING, MAX_UNREAD_PACKETS);
    let haptics_sender = stream_socket.request_stream(HAPTICS);
    let mut statics_receiver =
        stream_socket.subscribe_to_stream::<ClientStatistics>(STATISTICS, MAX_UNREAD_PACKETS);

    let (video_channel_sender, video_channel_receiver) =
        std::sync::mpsc::sync_channel(initial_settings.connection.max_queued_server_video_frames);
    *ctx.video_channel_sender.lock() = Some(video_channel_sender);
    *ctx.haptics_sender.lock() = Some(haptics_sender);

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

    #[cfg_attr(target_os = "linux", allow(unused_variables))]
    let game_audio_thread = if let Switch::Enabled(config) =
        initial_settings.audio.game_audio.clone()
    {
        #[cfg(windows)]
        let ctx = Arc::clone(&ctx);

        let client_hostname = client_hostname.clone();
        thread::spawn(move || {
            while is_streaming(&client_hostname) {
                #[cfg(target_os = "linux")]
                if let Err(e) = alvr_audio::linux::record_audio_blocking_pipewire(
                    Arc::new({
                        let client_hostname = client_hostname.clone();
                        move || is_streaming(&client_hostname)
                    }),
                    game_audio_sender.clone(),
                    2,
                    game_audio_sample_rate,
                ) {
                    error!("Audio record error: {e:?}");
                }

                #[cfg(not(target_os = "linux"))]
                {
                    let device = match alvr_audio::AudioDevice::new_output(config.device.as_ref()) {
                        Ok(data) => data,
                        Err(e) => {
                            warn!("New audio device failed: {e:?}");
                            thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
                            continue;
                        }
                    };

                    #[cfg(windows)]
                    if let Ok(id) = alvr_audio::get_windows_device_id(&device) {
                        let prop = alvr_session::OpenvrProperty {
                            key: alvr_session::OpenvrPropKey::AudioDefaultPlaybackDeviceIdString,
                            value: id,
                        };
                        ctx.events_sender
                            .send(ServerCoreEvent::SetOpenvrProperty {
                                device_id: *alvr_common::HEAD_ID,
                                prop,
                            })
                            .ok();
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
                    if let Ok(id) = alvr_audio::AudioDevice::new_output(None)
                        .and_then(|d| alvr_audio::get_windows_device_id(&d))
                    {
                        let prop = alvr_session::OpenvrProperty {
                            key: alvr_session::OpenvrPropKey::AudioDefaultPlaybackDeviceIdString,
                            value: id,
                        };
                        ctx.events_sender
                            .send(ServerCoreEvent::SetOpenvrProperty {
                                device_id: *alvr_common::HEAD_ID,
                                prop,
                            })
                            .ok();
                    }
                }
            }
        })
    } else {
        thread::spawn(|| ())
    };

    let microphone_thread =
        if let Switch::Enabled(config) = initial_settings.audio.microphone.clone() {
            #[cfg(not(target_os = "linux"))]
            #[allow(unused_variables)]
            let (sink, source) =
                alvr_audio::AudioDevice::new_virtual_microphone_pair(config.devices).to_con()?;

            #[cfg(windows)]
            if let Ok(id) = alvr_audio::get_windows_device_id(&source) {
                ctx.events_sender
                    .send(ServerCoreEvent::SetOpenvrProperty {
                        device_id: *alvr_common::HEAD_ID,
                        prop: alvr_session::OpenvrProperty {
                            key: alvr_session::OpenvrPropKey::AudioDefaultRecordingDeviceIdString,
                            value: id,
                        },
                    })
                    .ok();
            }

            let client_hostname = client_hostname.clone();
            thread::spawn(move || {
                #[cfg(not(target_os = "linux"))]
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
                #[cfg(target_os = "linux")]
                alvr_common::show_err(alvr_audio::linux::play_microphone_loop_pipewire(
                    {
                        let client_hostname = client_hostname.clone();
                        move || is_streaming(&client_hostname)
                    },
                    1,
                    streaming_caps.microphone_sample_rate,
                    config.buffering,
                    &mut microphone_receiver,
                ));
            })
        } else {
            thread::spawn(|| ())
        };

    *ctx.tracking_manager.write() = TrackingManager::new();
    let hand_gesture_manager = Arc::new(Mutex::new(HandGestureManager::new()));

    let tracking_receive_thread = thread::spawn({
        let ctx = Arc::clone(&ctx);
        let hand_gesture_manager = Arc::clone(&hand_gesture_manager);
        let initial_settings = initial_settings.clone();
        let client_hostname = client_hostname.clone();
        move || {
            tracking::tracking_loop(
                &ctx,
                initial_settings,
                streaming_caps.multimodal_protocol,
                hand_gesture_manager,
                tracking_receiver,
                || is_streaming(&client_hostname),
            );
        }
    });

    let statistics_thread = thread::spawn({
        let ctx = Arc::clone(&ctx);
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

                if let Some(stats) = &mut *ctx.statistics_manager.write() {
                    let timestamp = client_stats.target_timestamp;
                    let decoder_latency = client_stats.video_decode;
                    let (network_latency, game_latency) = stats.report_statistics(client_stats);

                    ctx.events_sender
                        .send(ServerCoreEvent::GameRenderLatencyFeedback(game_latency))
                        .ok();

                    let session_manager_lock = SESSION_MANAGER.read();
                    ctx.bitrate_manager.lock().report_frame_latencies(
                        &session_manager_lock.settings().video.bitrate.mode,
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
        let ctx = Arc::clone(&ctx);

        let controllers_config = session_manager_lock
            .settings()
            .headset
            .controllers
            .as_option();
        let mut controller_button_mapping_manager = controllers_config.map(|config| {
            if let Some(mappings) = &config.button_mappings {
                ButtonMappingManager::new_manual(mappings)
            } else {
                ButtonMappingManager::new_automatic(
                    &CONTROLLER_PROFILE_INFO
                        .get(&alvr_common::hash_string(QUEST_CONTROLLER_PROFILE_PATH))
                        .unwrap()
                        .button_set,
                    &config.emulation_mode,
                    &config.button_mapping_config,
                )
            }
        });
        let controllers_emulation_mode =
            controllers_config.map(|config| config.emulation_mode.clone());

        let disconnect_notif = Arc::clone(&disconnect_notif);
        let control_sender = Arc::clone(&control_sender);
        let client_hostname = client_hostname.clone();
        move || {
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
                        if !initial_settings.headset.tracking_ref_only {
                            let session_manager_lock = SESSION_MANAGER.read();
                            let config = &session_manager_lock.settings().headset;
                            ctx.tracking_manager.write().recenter(
                                config.position_recentering_mode,
                                config.rotation_recentering_mode,
                            );

                            let area = packet.unwrap_or(Vec2::new(2.0, 2.0));
                            let wh = area.x * area.y;
                            if wh.is_finite() && wh > 0.0 {
                                info!("Received new playspace with size: {}", area);
                                ctx.events_sender
                                    .send(ServerCoreEvent::PlayspaceSync(area))
                                    .ok();
                            } else {
                                warn!("Received invalid playspace size: {}", area);
                                ctx.events_sender
                                    .send(ServerCoreEvent::PlayspaceSync(Vec2::new(2.0, 2.0)))
                                    .ok();
                            }
                        }
                    }
                    ClientControlPacket::RequestIdr => {
                        if let Some(config) = ctx.decoder_config.lock().clone() {
                            control_sender
                                .lock()
                                .send(&ServerControlPacket::DecoderConfig(config))
                                .ok();
                        }
                        ctx.events_sender.send(ServerCoreEvent::RequestIDR).ok();
                    }
                    ClientControlPacket::VideoErrorReport => {
                        // legacy endpoint. todo: remove
                        if let Some(stats) = &mut *ctx.statistics_manager.write() {
                            stats.report_packet_loss();
                        }
                        ctx.events_sender.send(ServerCoreEvent::RequestIDR).ok();
                    }
                    ClientControlPacket::ViewsConfig(config) => {
                        ctx.events_sender
                            .send(ServerCoreEvent::ViewsConfig(ViewsConfig {
                                local_view_transforms: [
                                    Pose {
                                        position: Vec3::new(-config.ipd_m / 2., 0., 0.),
                                        orientation: Quat::IDENTITY,
                                    },
                                    Pose {
                                        position: Vec3::new(config.ipd_m / 2., 0., 0.),
                                        orientation: Quat::IDENTITY,
                                    },
                                ],
                                fov: config.fov,
                            }))
                            .ok();
                    }
                    ClientControlPacket::Battery(packet) => {
                        ctx.events_sender
                            .send(ServerCoreEvent::Battery(BatteryInfo {
                                device_id: packet.device_id,
                                gauge_value: packet.gauge_value,
                                is_plugged: packet.is_plugged,
                            }))
                            .ok();

                        if let Some(stats) = &mut *ctx.statistics_manager.write() {
                            stats.report_battery(
                                packet.device_id,
                                packet.gauge_value,
                                packet.is_plugged,
                            );
                        }
                    }
                    ClientControlPacket::Buttons(entries) => {
                        {
                            let session_manager_lock = SESSION_MANAGER.read();
                            if session_manager_lock
                                .settings()
                                .extra
                                .logging
                                .log_button_presses
                            {
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
                            let button_entries = entries
                                .iter()
                                .flat_map(|entry| manager.map_button(entry))
                                .collect::<Vec<_>>();

                            if !button_entries.is_empty() {
                                ctx.events_sender
                                    .send(ServerCoreEvent::Buttons(button_entries))
                                    .ok();
                            }
                        };
                    }
                    ClientControlPacket::ActiveInteractionProfile { profile_id, .. } => {
                        controller_button_mapping_manager = if let Switch::Enabled(config) =
                            &SESSION_MANAGER.read().settings().headset.controllers
                        {
                            if let Some(mappings) = &config.button_mappings {
                                Some(ButtonMappingManager::new_manual(mappings))
                            } else if let (Some(profile_info), Some(emulation_mode)) = (
                                CONTROLLER_PROFILE_INFO.get(&profile_id),
                                &controllers_emulation_mode,
                            ) {
                                Some(ButtonMappingManager::new_automatic(
                                    &profile_info.button_set,
                                    emulation_mode,
                                    &config.button_mapping_config,
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    }
                    ClientControlPacket::Log { level, message } => {
                        info!("Client {client_hostname}: [{level:?}] {message}")
                    }
                    ClientControlPacket::Reserved(json_string) => {
                        let reserved: ReservedClientControlPacket =
                            match serde_json::from_str(&json_string) {
                                Ok(reserved) => reserved,
                                Err(e) => {
                                    info!(
                                    "Failed to parse reserved packet: {e}. Packet: {json_string}"
                                );
                                    continue;
                                }
                            };

                        match reserved {
                            ReservedClientControlPacket::CustomInteractionProfile {
                                input_ids,
                                ..
                            } => {
                                controller_button_mapping_manager = if let Switch::Enabled(config) =
                                    &SESSION_MANAGER.read().settings().headset.controllers
                                {
                                    if let Some(mappings) = &config.button_mappings {
                                        Some(ButtonMappingManager::new_manual(mappings))
                                    } else {
                                        controllers_emulation_mode.as_ref().map(|emulation_mode| {
                                            ButtonMappingManager::new_automatic(
                                                &input_ids,
                                                emulation_mode,
                                                &config.button_mapping_config,
                                            )
                                        })
                                    }
                                } else {
                                    None
                                };
                            }
                        }
                    }
                    _ => (),
                }

                disconnection_deadline = Instant::now() + KEEPALIVE_TIMEOUT;
            }

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
            while SESSION_MANAGER
                .read()
                .client_list()
                .get(&client_hostname)
                .map(|c| c.connection_state == ConnectionState::Streaming)
                .unwrap_or(false)
                && *lifecycle_state.read() == LifecycleState::Resumed
            {
                thread::sleep(STREAMING_RECV_TIMEOUT);
            }

            disconnect_notif.notify_one()
        }
    });

    {
        let on_connect_script = initial_settings.connection.on_connect_script;

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

    if initial_settings.extra.capture.startup_video_recording {
        info!("Creating recording file");
        crate::create_recording_file(&ctx, session_manager_lock.settings());
    }

    session_manager_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::SetConnectionState(ConnectionState::Streaming),
    );

    ctx.events_sender
        .send(ServerCoreEvent::ClientConnected)
        .ok();

    dbg_connection!("connection_pipeline: handshake finished; unlocking streams");
    alvr_common::wait_rwlock(&disconnect_notif, &mut session_manager_lock);
    dbg_connection!("connection_pipeline: Begin connection shutdown");

    // This requests shutdown from threads
    *ctx.video_channel_sender.lock() = None;
    *ctx.haptics_sender.lock() = None;

    *ctx.video_recording_file.lock() = None;

    session_manager_lock.update_client_list(
        client_hostname.clone(),
        ClientListAction::SetConnectionState(ConnectionState::Disconnecting),
    );

    let on_disconnect_script = session_manager_lock
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
    drop(session_manager_lock);

    // Ensure shutdown of threads
    dbg_connection!("connection_pipeline: Shutdown threads");
    video_send_thread.join().ok();
    game_audio_thread.join().ok();
    microphone_thread.join().ok();
    tracking_receive_thread.join().ok();
    statistics_thread.join().ok();
    control_receive_thread.join().ok();
    stream_receive_thread.join().ok();
    keepalive_thread.join().ok();
    lifecycle_check_thread.join().ok();

    ctx.events_sender
        .send(ServerCoreEvent::ClientDisconnected)
        .ok();

    dbg_connection!("connection_pipeline: End");

    Ok(())
}
