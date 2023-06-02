use crate::{
    bitrate::BitrateManager,
    buttons::BUTTON_PATH_FROM_ID,
    face_tracking::FaceTrackingSink,
    haptics,
    sockets::WelcomeSocket,
    statistics::StatisticsManager,
    tracking::{self, TrackingManager},
    FfiButtonValue, FfiFov, FfiViewsConfig, VideoPacket, BITRATE_MANAGER, CONTROL_CHANNEL_SENDER,
    DECODER_CONFIG, DISCONNECT_CLIENT_NOTIFIER, HAPTICS_SENDER, RESTART_NOTIFIER,
    SERVER_DATA_MANAGER, STATISTICS_MANAGER, VIDEO_RECORDING_FILE, VIDEO_SENDER,
};
use alvr_audio::AudioDevice;
use alvr_common::{
    glam::{UVec2, Vec2},
    once_cell::sync::Lazy,
    parking_lot,
    prelude::*,
    settings_schema::Switch,
    RelaxedAtomic, DEVICE_ID_TO_PATH, HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_events::{ButtonEvent, EventType, HapticsEvent, TrackingEvent};
use alvr_packets::{
    ButtonValue, ClientConnectionResult, ClientControlPacket, ClientListAction, ClientStatistics,
    ServerControlPacket, StreamConfigPacket, Tracking, AUDIO, HAPTICS, STATISTICS, TRACKING, VIDEO,
};
use alvr_session::{CodecType, ControllersEmulationMode, FrameSize, OpenvrConfig};
use alvr_sockets::{
    spawn_cancelable, ControlSocketReceiver, ControlSocketSender, PeerType, ProtoControlSocket,
    StreamSocketBuilder, KEEPALIVE_INTERVAL,
};
use futures::future::BoxFuture;
use std::{
    collections::{HashMap, HashSet},
    future,
    net::IpAddr,
    process::Command,
    ptr,
    sync::{mpsc as smpsc, Arc},
    thread,
    time::Duration,
};
use tokio::{
    runtime::Runtime,
    sync::{mpsc as tmpsc, Mutex},
    time,
};

const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);

pub static SHOULD_CONNECT_TO_CLIENTS: Lazy<Arc<RelaxedAtomic>> =
    Lazy::new(|| Arc::new(RelaxedAtomic::new(false)));
static CONNECTED_CLIENT_HOSTNAMES: Lazy<parking_lot::Mutex<HashSet<String>>> =
    Lazy::new(|| parking_lot::Mutex::new(HashSet::new()));
static STREAMING_CLIENT_HOSTNAME: Lazy<parking_lot::Mutex<Option<String>>> =
    Lazy::new(|| parking_lot::Mutex::new(None));

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

// Alternate connection trials with manual IPs and clients discovered on the local network
pub fn handshake_loop() -> IntResult {
    let mut welcome_socket = WelcomeSocket::new().map_err(to_int_e!())?;

    loop {
        check_interrupt!(SHOULD_CONNECT_TO_CLIENTS.value());

        let manual_client_ips = {
            let connected_hostnames_lock = CONNECTED_CLIENT_HOSTNAMES.lock();
            let mut manual_client_ips = HashMap::new();
            for (hostname, connection_info) in SERVER_DATA_MANAGER.read().client_list() {
                if !connected_hostnames_lock.contains(hostname) {
                    for ip in &connection_info.manual_ips {
                        manual_client_ips.insert(*ip, hostname.clone());
                    }
                }
            }
            manual_client_ips
        };

        if !manual_client_ips.is_empty() && try_connect(manual_client_ips).is_ok() {
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
            let (client_hostname, client_ip) = match welcome_socket.recv_non_blocking() {
                Ok(pair) => pair,
                Err(e) => {
                    if let InterruptibleError::Other(e) = e {
                        warn!("UDP handshake listening error: {e}");
                    }

                    thread::sleep(RETRY_CONNECT_MIN_INTERVAL);
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
            if trusted && !CONNECTED_CLIENT_HOSTNAMES.lock().contains(&client_hostname) {
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

pub fn contruct_openvr_config() -> OpenvrConfig {
    let data_manager_lock = SERVER_DATA_MANAGER.read();
    let old_config = data_manager_lock.session().openvr_config.clone();
    let settings = data_manager_lock.settings().clone();

    let mut controllers_mode_idx = 0;
    let mut override_trigger_threshold = false;
    let mut trigger_threshold = 0.0;
    let mut override_grip_threshold = false;
    let mut grip_threshold = 0.0;
    let controllers_enabled = if let Switch::Enabled(config) = settings.headset.controllers {
        controllers_mode_idx = match config.emulation_mode {
            ControllersEmulationMode::RiftSTouch => 1,
            ControllersEmulationMode::ValveIndex => 3,
            ControllersEmulationMode::ViveWand => 5,
            ControllersEmulationMode::Quest2Touch => 7,
            ControllersEmulationMode::ViveTracker => 9,
        };
        override_trigger_threshold =
            if let Switch::Enabled(value) = config.trigger_threshold_override {
                trigger_threshold = value;
                true
            } else {
                false
            };
        override_grip_threshold = if let Switch::Enabled(value) = config.grip_threshold_override {
            grip_threshold = value;
            true
        } else {
            false
        };
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
        controllers_mode_idx,
        override_trigger_threshold,
        trigger_threshold,
        override_grip_threshold,
        grip_threshold,
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
        ..old_config
    }
}

fn try_connect(mut client_ips: HashMap<IpAddr, String>) -> IntResult {
    let runtime = Runtime::new().map_err(to_int_e!())?;

    let (mut proto_socket, client_ip) = runtime
        .block_on(async {
            let get_proto_socket = ProtoControlSocket::connect_to(PeerType::AnyClient(
                client_ips.keys().cloned().collect(),
            ));
            tokio::select! {
                proto_socket = get_proto_socket => proto_socket,
                _ = time::sleep(Duration::from_secs(1)) => {
                    fmt_e!("Control socket failed to connect")
                }
            }
        })
        .map_err(to_int_e!())?;

    // Safety: this never panics because client_ip is picked from client_ips keys
    let client_hostname = client_ips.remove(&client_ip).unwrap();

    SERVER_DATA_MANAGER.write().update_client_list(
        client_hostname.clone(),
        ClientListAction::UpdateCurrentIp(Some(client_ip)),
    );

    let maybe_streaming_caps = if let ClientConnectionResult::ConnectionAccepted {
        client_protocol_id,
        display_name,
        streaming_capabilities,
        ..
    } = runtime.block_on(proto_socket.recv()).map_err(to_int_e!())?
    {
        if client_protocol_id != alvr_common::protocol_id() {
            warn!(
                "Trusted client is incompatible! Expected protocol ID: {}, found: {}",
                alvr_common::protocol_id(),
                client_protocol_id,
            );
        }

        SERVER_DATA_MANAGER.write().update_client_list(
            client_hostname.clone(),
            ClientListAction::SetDisplayName(display_name),
        );

        streaming_capabilities
    } else {
        debug!("Found client in standby. Retrying");
        return Ok(());
    };

    let streaming_caps = if let Some(streaming_caps) = maybe_streaming_caps {
        if let Some(hostname) = &*STREAMING_CLIENT_HOSTNAME.lock() {
            return int_fmt_e!("Streaming client {hostname} is already connected!");
        } else {
            streaming_caps
        }
    } else {
        return int_fmt_e!("Only streaming clients are supported for now");
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

    let game_audio_sample_rate = if let Switch::Enabled(game_audio_config) =
        settings.audio.game_audio
    {
        let game_audio_device = AudioDevice::new_output(
            Some(settings.audio.linux_backend),
            game_audio_config.device.as_ref(),
        )
        .map_err(to_int_e!())?;

        #[cfg(not(target_os = "linux"))]
        if let Switch::Enabled(microphone_desc) = settings.audio.microphone {
            let (sink, source) = AudioDevice::new_virtual_microphone_pair(
                Some(settings.audio.linux_backend),
                microphone_desc.devices,
            )
            .map_err(to_int_e!())?;
            if alvr_audio::is_same_device(&game_audio_device, &sink)
                || alvr_audio::is_same_device(&game_audio_device, &source)
            {
                return int_fmt_e!("Game audio and microphone cannot point to the same device!");
            }
        }

        game_audio_device.input_sample_rate().map_err(to_int_e!())?
    } else {
        0
    };

    let client_config = StreamConfigPacket {
        session: {
            let session = SERVER_DATA_MANAGER.read().session().clone();
            serde_json::to_string(&session).map_err(to_int_e!())?
        },
        negotiated: serde_json::json!({
            "view_resolution": stream_view_resolution,
            "refresh_rate_hint": fps,
            "game_audio_sample_rate": game_audio_sample_rate,
        })
        .to_string(),
    };
    runtime
        .block_on(proto_socket.send(&client_config))
        .map_err(to_int_e!())?;

    let (mut control_sender, control_receiver) = proto_socket.split();

    let mut new_openvr_config = contruct_openvr_config();
    new_openvr_config.eye_resolution_width = stream_view_resolution.x;
    new_openvr_config.eye_resolution_height = stream_view_resolution.y;
    new_openvr_config.target_eye_resolution_width = target_view_resolution.x;
    new_openvr_config.target_eye_resolution_height = target_view_resolution.y;
    new_openvr_config.refresh_rate = fps as _;

    if SERVER_DATA_MANAGER.read().session().openvr_config != new_openvr_config {
        SERVER_DATA_MANAGER.write().session_mut().openvr_config = new_openvr_config;

        runtime
            .block_on(control_sender.send(&ServerControlPacket::Restarting))
            .ok();

        crate::notify_restart_driver();
    }

    CONNECTED_CLIENT_HOSTNAMES
        .lock()
        .insert(client_hostname.clone());

    *STREAMING_CLIENT_HOSTNAME.lock() = Some(client_hostname.clone());

    thread::spawn(move || {
        runtime.block_on({
            let client_hostname = client_hostname.clone();
            async move {
                // this is a bridge between sync and async, skips the needs for a notifier
                let shutdown_detector = async {
                    while SHOULD_CONNECT_TO_CLIENTS.value() {
                        time::sleep(Duration::from_secs(1)).await;
                    }
                };

                tokio::select! {
                    res = connection_pipeline(
                        client_hostname,
                        client_ip,
                        control_sender,
                        control_receiver,
                        streaming_caps.microphone_sample_rate,
                        fps,
                    ) => {
                        if let Err(e) = res {
                            warn!("Connection interrupted: {e:?}");
                        }
                    },
                    _ = DISCONNECT_CLIENT_NOTIFIER.notified() => (),
                    _ = shutdown_detector => (),
                };
            }
        });

        {
            let mut streaming_hostname_lock = STREAMING_CLIENT_HOSTNAME.lock();
            if let Some(hostname) = streaming_hostname_lock.clone() {
                if hostname == client_hostname {
                    *streaming_hostname_lock = None
                }
            }
        }

        CONNECTED_CLIENT_HOSTNAMES.lock().remove(&client_hostname);
    });

    Ok(())
}

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard(Arc<RelaxedAtomic>);

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        self.0.set(false);

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
    }
}

async fn connection_pipeline(
    client_hostname: String,
    client_ip: IpAddr,
    control_sender: ControlSocketSender<ServerControlPacket>,
    mut control_receiver: ControlSocketReceiver<ClientControlPacket>,
    microphone_sample_rate: u32,
    refresh_rate: f32,
) -> StrResult {
    let control_sender = Arc::new(Mutex::new(control_sender));

    control_sender
        .lock()
        .await
        .send(&ServerControlPacket::StartStream)
        .await?;

    match control_receiver.recv().await {
        Ok(ClientControlPacket::StreamReady) => {}
        Ok(_) => {
            return fmt_e!("Got unexpected packet waiting for stream ack");
        }
        Err(e) => {
            return fmt_e!("Error while waiting for stream ack: {e}");
        }
    }

    let settings = SERVER_DATA_MANAGER.read().settings().clone();

    let stream_socket = tokio::select! {
        res = StreamSocketBuilder::connect_to_client(
            client_ip,
            settings.connection.stream_port,
            settings.connection.stream_protocol,
            settings.connection.server_send_buffer_bytes,
            settings.connection.server_recv_buffer_bytes,
            settings.connection.packet_size as _,
        ) => res?,
        _ = time::sleep(Duration::from_secs(5)) => {
            return fmt_e!("Timeout while setting up streams");
        }
    };
    let stream_socket = Arc::new(stream_socket);

    *STATISTICS_MANAGER.lock() = Some(StatisticsManager::new(
        settings.connection.statistics_history_size as _,
        Duration::from_secs_f32(1.0 / refresh_rate),
        if let Switch::Enabled(config) = &settings.headset.controllers {
            config.steamvr_pipeline_frames
        } else {
            0.0
        },
    ));

    *BITRATE_MANAGER.lock() = BitrateManager::new(
        settings.connection.statistics_history_size as _,
        refresh_rate,
    );

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

    if settings.capture.save_video_stream {
        crate::create_recording_file();
    }

    unsafe { crate::InitializeStreaming() };

    let is_streaming = Arc::new(RelaxedAtomic::new(true));
    let _stream_guard = StreamCloseGuard(Arc::clone(&is_streaming));

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(config) = settings.audio.game_audio {
        let sender = stream_socket.request_stream(AUDIO).await?;
        Box::pin(async move {
            loop {
                let device = match AudioDevice::new_output(
                    Some(settings.audio.linux_backend),
                    config.device.as_ref(),
                ) {
                    Ok(data) => data,
                    Err(e) => {
                        warn!("New audio device Failed : {e}");
                        time::sleep(RETRY_CONNECT_MIN_INTERVAL).await;
                        continue;
                    }
                };
                let mute_when_streaming = config.mute_when_streaming;

                #[cfg(windows)]
                if let Ok(id) = alvr_audio::get_windows_device_id(&device) {
                    unsafe {
                        crate::SetOpenvrProperty(
                            *alvr_common::HEAD_ID,
                            crate::openvr_props::to_ffi_openvr_prop(
                                alvr_session::OpenvrPropertyKey::AudioDefaultPlaybackDeviceId,
                                alvr_session::OpenvrPropValue::String(id),
                            ),
                        )
                    }
                } else {
                    continue;
                };

                let new_sender = sender.clone();
                if let Err(e) =
                    alvr_audio::record_audio_loop(device, 2, mute_when_streaming, new_sender).await
                {
                    warn!("Audio task exit with error : {e}")
                }

                #[cfg(windows)]
                if let Ok(id) =
                    alvr_audio::get_windows_device_id(&AudioDevice::new_output(None, None)?)
                {
                    unsafe {
                        crate::SetOpenvrProperty(
                            *alvr_common::HEAD_ID,
                            crate::openvr_props::to_ffi_openvr_prop(
                                alvr_session::OpenvrPropertyKey::AudioDefaultPlaybackDeviceId,
                                alvr_session::OpenvrPropValue::String(id),
                            ),
                        )
                    }
                }
            }
        })
    } else {
        Box::pin(future::pending())
    };
    let microphone_loop: BoxFuture<_> = if let Switch::Enabled(config) = settings.audio.microphone {
        #[allow(unused_variables)]
        let (sink, source) = AudioDevice::new_virtual_microphone_pair(
            Some(settings.audio.linux_backend),
            config.devices,
        )?;
        let receiver = stream_socket.subscribe_to_stream(AUDIO).await?;

        #[cfg(windows)]
        if let Ok(id) = alvr_audio::get_windows_device_id(&source) {
            unsafe {
                crate::SetOpenvrProperty(
                    *alvr_common::HEAD_ID,
                    crate::openvr_props::to_ffi_openvr_prop(
                        alvr_session::OpenvrPropertyKey::AudioDefaultRecordingDeviceId,
                        alvr_session::OpenvrPropValue::String(id),
                    ),
                )
            }
        }

        Box::pin(alvr_audio::play_audio_loop(
            sink,
            1,
            microphone_sample_rate,
            config.buffering,
            receiver,
        ))
    } else {
        Box::pin(future::pending())
    };

    let video_send_loop = {
        let mut socket_sender = stream_socket.request_stream(VIDEO).await?;
        async move {
            let (data_sender, mut data_receiver) =
                tmpsc::channel(settings.connection.max_queued_server_video_frames);
            *VIDEO_SENDER.lock() = Some(data_sender);

            while let Some(VideoPacket { header, payload }) = data_receiver.recv().await {
                socket_sender.send(&header, payload).await.ok();
            }

            Ok(())
        }
    };

    let haptics_send_loop = {
        let mut socket_sender = stream_socket.request_stream(HAPTICS).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *HAPTICS_SENDER.lock() = Some(data_sender);

            while let Some(haptics) = data_receiver.recv().await {
                let haptics_config = {
                    let data_manager_lock = SERVER_DATA_MANAGER.read();

                    if data_manager_lock.settings().logging.log_haptics {
                        alvr_events::send_event(EventType::Haptics(HapticsEvent {
                            path: DEVICE_ID_TO_PATH
                                .get(&haptics.device_id)
                                .map(|p| (*p).to_owned())
                                .unwrap_or_else(|| {
                                    format!("Unknown (ID: {:#16x})", haptics.device_id)
                                }),
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

                if let Some(config) = haptics_config {
                    socket_sender
                        .send(&haptics::map_haptics(&config, haptics), vec![])
                        .await
                        .ok();
                }
            }

            Ok(())
        }
    };

    let (playspace_sync_sender, playspace_sync_receiver) = smpsc::channel::<Option<Vec2>>();

    let is_tracking_ref_only = settings.headset.tracking_ref_only;
    if !is_tracking_ref_only {
        // use a separate thread because SetChaperone() is blocking
        thread::spawn(move || {
            while let Ok(packet) = playspace_sync_receiver.recv() {
                if let Some(area) = packet {
                    unsafe { crate::SetChaperone(area.x, area.y) };
                } else {
                    unsafe { crate::SetChaperone(2.0, 2.0) };
                }
            }
        });
    }

    let tracking_manager = Arc::new(Mutex::new(TrackingManager::new()));

    let tracking_receive_loop = {
        let mut receiver = stream_socket
            .subscribe_to_stream::<Tracking>(TRACKING)
            .await?;
        let tracking_manager = Arc::clone(&tracking_manager);
        async move {
            let face_tracking_sink = if let Switch::Enabled(config) = settings.headset.face_tracking
            {
                Some(FaceTrackingSink::new(
                    config.sink,
                    settings.connection.osc_local_port,
                )?)
            } else {
                None
            };

            let mut track_controllers = 0u32;
            if let Switch::Enabled(config) = settings.headset.controllers {
                track_controllers = config.tracked.into();
            }

            loop {
                let tracking = receiver.recv_header_only().await?;

                let mut tracking_manager_lock = tracking_manager.lock().await;

                let motions;
                let left_hand_skeleton;
                let right_hand_skeleton;
                {
                    let data_manager_lock = SERVER_DATA_MANAGER.read();
                    let config = &data_manager_lock.settings().headset;
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
                    let data_manager_lock = SERVER_DATA_MANAGER.read();
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

                if let Some(sink) = &face_tracking_sink {
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
                            track_controllers,
                        )
                    };
                }
            }
        }
    };

    let statistics_receive_loop = {
        let mut receiver = stream_socket
            .subscribe_to_stream::<ClientStatistics>(STATISTICS)
            .await?;
        async move {
            loop {
                let client_stats = receiver.recv_header_only().await?;

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
        }
    };

    let keepalive_loop = {
        let control_sender = Arc::clone(&control_sender);
        async move {
            loop {
                let res = control_sender
                    .lock()
                    .await
                    .send(&ServerControlPacket::KeepAlive)
                    .await;
                if let Err(e) = res {
                    info!("Client disconnected. Cause: {e}");
                    break Ok(());
                }
                time::sleep(KEEPALIVE_INTERVAL).await;
            }
        }
    };

    let (control_channel_sender, mut control_channel_receiver) = tmpsc::unbounded_channel();
    *CONTROL_CHANNEL_SENDER.lock() = Some(control_channel_sender);

    let control_send_loop = {
        let control_sender = Arc::clone(&control_sender);
        async move {
            while let Some(packet) = control_channel_receiver.recv().await {
                control_sender.lock().await.send(&packet).await?;
            }

            Ok(())
        }
    };

    let control_loop = async move {
        loop {
            match control_receiver.recv().await {
                Ok(ClientControlPacket::PlayspaceSync(packet)) => {
                    if !is_tracking_ref_only {
                        playspace_sync_sender.send(packet).ok();

                        let data_manager_lock = SERVER_DATA_MANAGER.read();
                        let config = &data_manager_lock.settings().headset;
                        tracking_manager.lock().await.recenter(
                            config.position_recentering_mode,
                            config.rotation_recentering_mode,
                        );
                    }
                }
                Ok(ClientControlPacket::RequestIdr) => {
                    if let Some(sender) = &*CONTROL_CHANNEL_SENDER.lock() {
                        if let Some(config) = &*DECODER_CONFIG.lock() {
                            sender
                                .send(ServerControlPacket::InitializeDecoder(config.clone()))
                                .ok();
                        }
                    }
                    unsafe { crate::RequestIDR() }
                }
                Ok(ClientControlPacket::VideoErrorReport) => {
                    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                        stats.report_packet_loss();
                    }
                    unsafe { crate::VideoErrorReportReceive() };
                }
                Ok(ClientControlPacket::ViewsConfig(config)) => unsafe {
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
                Ok(ClientControlPacket::Battery(packet)) => unsafe {
                    crate::SetBattery(packet.device_id, packet.gauge_value, packet.is_plugged);

                    if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                        stats.report_battery(packet.device_id, packet.gauge_value);
                    }
                },
                Ok(ClientControlPacket::Buttons(entries)) => {
                    {
                        let data_manager_lock = SERVER_DATA_MANAGER.read();
                        if data_manager_lock.settings().logging.log_button_presses {
                            alvr_events::send_event(EventType::Buttons(
                                entries
                                    .iter()
                                    .map(|e| ButtonEvent {
                                        path: BUTTON_PATH_FROM_ID
                                            .get(&e.path_id)
                                            .cloned()
                                            .unwrap_or_else(|| {
                                                format!("Unknown (ID: {:#16x})", e.path_id)
                                            }),
                                        value: e.value,
                                    })
                                    .collect(),
                            ));
                        }
                    }

                    for entry in entries {
                        let value = match entry.value {
                            ButtonValue::Binary(value) => FfiButtonValue {
                                type_: crate::FfiButtonType_BUTTON_TYPE_BINARY,
                                __bindgen_anon_1: crate::FfiButtonValue__bindgen_ty_1 {
                                    binary: value.into(),
                                },
                            },

                            ButtonValue::Scalar(value) => FfiButtonValue {
                                type_: crate::FfiButtonType_BUTTON_TYPE_SCALAR,
                                __bindgen_anon_1: crate::FfiButtonValue__bindgen_ty_1 {
                                    scalar: value,
                                },
                            },
                        };

                        unsafe { crate::SetButton(entry.path_id, value) };
                    }
                }
                Ok(ClientControlPacket::Log { level, message }) => {
                    info!("Client {client_hostname}: [{level:?}] {message}")
                }
                Ok(_) => (),
                Err(e) => {
                    info!("Client disconnected. Cause: {e}");
                    break;
                }
            }
        }

        Ok(())
    };

    let receive_loop = async move { stream_socket.receive_loop().await };

    tokio::select! {
        // Spawn new tasks and let the runtime manage threading
        res = spawn_cancelable(receive_loop) => {
            if let Err(e) = res {
                info!("Client disconnected. Cause: {e}" );
            }

            Ok(())
        },
        res = spawn_cancelable(game_audio_loop) => res,
        res = spawn_cancelable(microphone_loop) => res,
        res = spawn_cancelable(video_send_loop) => res,
        res = spawn_cancelable(statistics_receive_loop) => res,
        res = spawn_cancelable(haptics_send_loop) => res,
        res = spawn_cancelable(tracking_receive_loop) => res,

        // Leave these loops on the current task
        res = keepalive_loop => res,
        res = control_loop => res,
        res = control_send_loop => res,

        _ = RESTART_NOTIFIER.notified() => {
            control_sender
                .lock()
                .await
                .send(&ServerControlPacket::Restarting)
                .await
                .ok();

            Ok(())
        }
    }
}
