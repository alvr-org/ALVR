use crate::{
    buttons::BUTTON_PATH_FROM_ID, connection_utils, statistics::StatisticsManager,
    tracking::TrackingManager, AlvrButtonType_BUTTON_TYPE_BINARY,
    AlvrButtonType_BUTTON_TYPE_SCALAR, AlvrButtonValue, AlvrButtonValue__bindgen_ty_1,
    AlvrDeviceMotion, AlvrQuat, EyeFov, OculusHand, CLIENTS_UPDATED_NOTIFIER, HAPTICS_SENDER,
    RESTART_NOTIFIER, SERVER_DATA_MANAGER, STATISTICS_MANAGER, VIDEO_SENDER,
};
use alvr_audio::{AudioDevice, AudioDeviceType};
use alvr_common::{
    glam::{Quat, Vec2},
    prelude::*,
    semver::Version,
    HEAD_ID,
};
use alvr_events::{ButtonEvent, ButtonValue, EventType};
use alvr_session::{CodecType, FrameSize, OpenvrConfig};
use alvr_sockets::{
    spawn_cancelable, ClientConfigPacket, ClientControlPacket, ClientListAction, ClientStatistics,
    ControlSocketReceiver, ControlSocketSender, HeadsetInfoPacket, PeerType, ProtoControlSocket,
    ServerControlPacket, StreamSocketBuilder, Tracking, AUDIO, HAPTICS, STATISTICS, TRACKING,
    VIDEO,
};
use futures::future::{BoxFuture, Either};
use settings_schema::Switch;
use std::{
    future,
    net::IpAddr,
    process::Command,
    str::FromStr,
    sync::{mpsc as smpsc, Arc},
    thread,
    time::Duration,
};
use tokio::{
    sync::{mpsc as tmpsc, Mutex},
    time,
};

#[cfg(windows)]
use alvr_session::{OpenvrPropValue, OpenvrPropertyKey};

const CONTROL_CONNECT_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
const CLEANUP_PAUSE: Duration = Duration::from_millis(500);

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

fn mbits_to_bytes(value: u64) -> u32 {
    (value * 1024 * 1024 / 8) as u32
}

#[derive(Clone)]
struct ClientId {
    hostname: String,
    ip: IpAddr,
}

async fn client_discovery(auto_trust_clients: bool) -> StrResult<ClientId> {
    let (ip, handshake_packet) =
        connection_utils::search_client_loop(|handshake_packet| async move {
            SERVER_DATA_MANAGER.lock().update_client_list(
                handshake_packet.hostname.clone(),
                ClientListAction::AddIfMissing {
                    display_name: handshake_packet.device_name,
                },
                Some(&CLIENTS_UPDATED_NOTIFIER),
            );

            if let Some(connection_desc) = SERVER_DATA_MANAGER
                .lock()
                .session()
                .client_connections
                .get(&handshake_packet.hostname)
            {
                connection_desc.trusted || auto_trust_clients
            } else {
                false
            }
        })
        .await?;

    Ok(ClientId {
        hostname: handshake_packet.hostname,
        ip,
    })
}

struct ConnectionInfo {
    client_ip: IpAddr,
    version: Option<Version>,
    control_sender: ControlSocketSender<ServerControlPacket>,
    control_receiver: ControlSocketReceiver<ClientControlPacket>,
    microphone_sample_rate: u32,
}

async fn client_handshake(
    trusted_discovered_client_id: Option<ClientId>,
) -> StrResult<ConnectionInfo> {
    let client_ips = if let Some(id) = trusted_discovered_client_id {
        vec![id.ip]
    } else {
        SERVER_DATA_MANAGER
            .lock()
            .session()
            .client_connections
            .iter()
            .fold(Vec::new(), |mut clients_info, (_, client)| {
                clients_info.extend(client.manual_ips.clone());
                clients_info
            })
    };

    let (mut proto_socket, client_ip) = loop {
        if let Ok(pair) =
            ProtoControlSocket::connect_to(PeerType::AnyClient(client_ips.clone())).await
        {
            break pair;
        }

        debug!("Timeout while searching for client. Retrying");
        time::sleep(CONTROL_CONNECT_RETRY_PAUSE).await;
    };

    let (headset_info, server_ip) = proto_socket
        .recv::<(HeadsetInfoPacket, IpAddr)>()
        .await
        .map_err(err!())?;

    let settings = SERVER_DATA_MANAGER.lock().session().to_settings();

    let (eye_width, eye_height) = match settings.video.render_resolution {
        FrameSize::Scale(scale) => (
            headset_info.recommended_eye_width as f32 * scale,
            headset_info.recommended_eye_height as f32 * scale,
        ),
        FrameSize::Absolute { width, height } => (width as f32 / 2_f32, height as f32),
    };
    let video_eye_width = align32(eye_width);
    let video_eye_height = align32(eye_height);

    let (eye_width, eye_height) = match settings.video.recommended_target_resolution {
        FrameSize::Scale(scale) => (
            headset_info.recommended_eye_width as f32 * scale,
            headset_info.recommended_eye_height as f32 * scale,
        ),
        FrameSize::Absolute { width, height } => (width as f32 / 2_f32, height as f32),
    };
    let target_eye_width = align32(eye_width);
    let target_eye_height = align32(eye_height);

    let fps = {
        let mut best_match = 0_f32;
        let mut min_diff = f32::MAX;
        for rr in &headset_info.available_refresh_rates {
            let diff = (*rr - settings.video.preferred_fps).abs();
            if diff < min_diff {
                best_match = *rr;
                min_diff = diff;
            }
        }
        best_match
    };

    if !headset_info
        .available_refresh_rates
        .contains(&settings.video.preferred_fps)
    {
        warn!("Chosen refresh rate not supported. Using {fps}Hz");
    }

    let dashboard_url = format!(
        "http://{server_ip}:{}/",
        settings.connection.web_server_port
    );

    let game_audio_sample_rate = if let Switch::Enabled(game_audio_desc) = settings.audio.game_audio
    {
        let game_audio_device = AudioDevice::new(
            Some(settings.audio.linux_backend),
            game_audio_desc.device_id,
            AudioDeviceType::Output,
        )?;

        if let Switch::Enabled(microphone_desc) = settings.audio.microphone {
            let microphone_device = AudioDevice::new(
                Some(settings.audio.linux_backend),
                microphone_desc.input_device_id,
                AudioDeviceType::VirtualMicrophoneInput,
            )?;
            #[cfg(not(target_os = "linux"))]
            if alvr_audio::is_same_device(&game_audio_device, &microphone_device) {
                return fmt_e!("Game audio and microphone cannot point to the same device!");
            }
        }

        game_audio_device.input_sample_rate()?
    } else {
        0
    };

    let version = Version::from_str(&headset_info.reserved).ok();

    let client_config = ClientConfigPacket {
        session_desc: {
            let mut session = SERVER_DATA_MANAGER.lock().session().clone();
            if cfg!(target_os = "linux") {
                session.session_settings.video.foveated_rendering.enabled = false;
            }

            serde_json::to_string(&session).map_err(err!())?
        },
        dashboard_url,
        eye_resolution_width: video_eye_width,
        eye_resolution_height: video_eye_height,
        fps,
        game_audio_sample_rate,
        reserved: "".into(),
        server_version: version.clone(),
    };
    proto_socket.send(&client_config).await?;

    let (mut control_sender, control_receiver) = proto_socket.split();

    let session_settings = SERVER_DATA_MANAGER
        .lock()
        .session()
        .session_settings
        .clone();

    let new_openvr_config = OpenvrConfig {
        universe_id: settings.headset.universe_id,
        headset_serial_number: settings.headset.serial_number,
        headset_tracking_system_name: settings.headset.tracking_system_name,
        headset_model_number: settings.headset.model_number,
        headset_driver_version: settings.headset.driver_version,
        headset_manufacturer_name: settings.headset.manufacturer_name,
        headset_render_model_name: settings.headset.render_model_name,
        headset_registered_device_type: settings.headset.registered_device_type,
        eye_resolution_width: video_eye_width,
        eye_resolution_height: video_eye_height,
        target_eye_resolution_width: target_eye_width,
        target_eye_resolution_height: target_eye_height,
        seconds_from_vsync_to_photons: settings.video.seconds_from_vsync_to_photons,
        force_3dof: settings.headset.force_3dof,
        tracking_ref_only: settings.headset.tracking_ref_only,
        enable_vive_tracker_proxy: settings.headset.enable_vive_tracker_proxy,
        aggressive_keyframe_resend: settings.connection.aggressive_keyframe_resend,
        adapter_index: settings.video.adapter_index,
        codec: matches!(settings.video.codec, CodecType::HEVC) as _,
        refresh_rate: fps as _,
        use_10bit_encoder: settings.video.use_10bit_encoder,
        force_sw_encoding: settings.video.force_sw_encoding,
        sw_thread_count: settings.video.sw_thread_count,
        encode_bitrate_mbs: settings.video.encode_bitrate_mbs,
        enable_adaptive_bitrate: session_settings.video.adaptive_bitrate.enabled,
        bitrate_maximum: session_settings
            .video
            .adaptive_bitrate
            .content
            .bitrate_maximum,
        latency_target: session_settings
            .video
            .adaptive_bitrate
            .content
            .latency_target,
        latency_use_frametime: session_settings
            .video
            .adaptive_bitrate
            .content
            .latency_use_frametime
            .enabled,
        latency_target_maximum: session_settings
            .video
            .adaptive_bitrate
            .content
            .latency_use_frametime
            .content
            .latency_target_maximum,
        latency_target_offset: session_settings
            .video
            .adaptive_bitrate
            .content
            .latency_use_frametime
            .content
            .latency_target_offset,
        latency_threshold: session_settings
            .video
            .adaptive_bitrate
            .content
            .latency_threshold,
        bitrate_up_rate: session_settings
            .video
            .adaptive_bitrate
            .content
            .bitrate_up_rate,
        bitrate_down_rate: session_settings
            .video
            .adaptive_bitrate
            .content
            .bitrate_down_rate,
        bitrate_light_load_threshold: session_settings
            .video
            .adaptive_bitrate
            .content
            .bitrate_light_load_threshold,
        controllers_tracking_system_name: session_settings
            .headset
            .controllers
            .content
            .tracking_system_name
            .clone(),
        controllers_manufacturer_name: session_settings
            .headset
            .controllers
            .content
            .manufacturer_name
            .clone(),
        controllers_model_number: session_settings
            .headset
            .controllers
            .content
            .model_number
            .clone(),
        render_model_name_left_controller: session_settings
            .headset
            .controllers
            .content
            .render_model_name_left
            .clone(),
        render_model_name_right_controller: session_settings
            .headset
            .controllers
            .content
            .render_model_name_right
            .clone(),
        controllers_serial_number: session_settings
            .headset
            .controllers
            .content
            .serial_number
            .clone(),
        controllers_type_left: session_settings
            .headset
            .controllers
            .content
            .ctrl_type_left
            .clone(),
        controllers_type_right: session_settings
            .headset
            .controllers
            .content
            .ctrl_type_right
            .clone(),
        controllers_registered_device_type: session_settings
            .headset
            .controllers
            .content
            .registered_device_type
            .clone(),
        controllers_input_profile_path: session_settings
            .headset
            .controllers
            .content
            .input_profile_path
            .clone(),
        controllers_mode_idx: session_settings.headset.controllers.content.mode_idx,
        controllers_enabled: session_settings.headset.controllers.enabled,
        position_offset: settings.headset.position_offset,
        linear_velocity_cutoff: session_settings
            .headset
            .controllers
            .content
            .linear_velocity_cutoff,
        angular_velocity_cutoff: session_settings
            .headset
            .controllers
            .content
            .angular_velocity_cutoff,
        position_offset_left: session_settings
            .headset
            .controllers
            .content
            .position_offset_left,
        rotation_offset_left: session_settings
            .headset
            .controllers
            .content
            .rotation_offset_left,
        haptics_intensity: session_settings
            .headset
            .controllers
            .content
            .haptics_intensity,
        haptics_amplitude_curve: session_settings
            .headset
            .controllers
            .content
            .haptics_amplitude_curve,
        haptics_min_duration: session_settings
            .headset
            .controllers
            .content
            .haptics_min_duration,
        haptics_low_duration_amplitude_multiplier: session_settings
            .headset
            .controllers
            .content
            .haptics_low_duration_amplitude_multiplier,
        haptics_low_duration_range: session_settings
            .headset
            .controllers
            .content
            .haptics_low_duration_range,
        use_headset_tracking_system: session_settings
            .headset
            .controllers
            .content
            .use_headset_tracking_system,
        enable_foveated_rendering: session_settings.video.foveated_rendering.enabled,
        foveation_center_size_x: session_settings
            .video
            .foveated_rendering
            .content
            .center_size_x,
        foveation_center_size_y: session_settings
            .video
            .foveated_rendering
            .content
            .center_size_y,
        foveation_center_shift_x: session_settings
            .video
            .foveated_rendering
            .content
            .center_shift_x,
        foveation_center_shift_y: session_settings
            .video
            .foveated_rendering
            .content
            .center_shift_y,
        foveation_edge_ratio_x: session_settings
            .video
            .foveated_rendering
            .content
            .edge_ratio_x,
        foveation_edge_ratio_y: session_settings
            .video
            .foveated_rendering
            .content
            .edge_ratio_y,
        enable_color_correction: session_settings.video.color_correction.enabled,
        brightness: session_settings.video.color_correction.content.brightness,
        contrast: session_settings.video.color_correction.content.contrast,
        saturation: session_settings.video.color_correction.content.saturation,
        gamma: session_settings.video.color_correction.content.gamma,
        sharpening: session_settings.video.color_correction.content.sharpening,
        enable_fec: session_settings.connection.enable_fec,
        linux_async_reprojection: session_settings.extra.patches.linux_async_reprojection,
    };

    if SERVER_DATA_MANAGER.lock().session().openvr_config != new_openvr_config {
        SERVER_DATA_MANAGER.lock().session_mut().openvr_config = new_openvr_config;

        control_sender
            .send(&ServerControlPacket::Restarting)
            .await
            .ok();

        crate::notify_restart_driver();

        // waiting for execution canceling
        future::pending::<()>().await;
    }

    Ok(ConnectionInfo {
        client_ip,
        version,
        control_sender,
        control_receiver,
        microphone_sample_rate: headset_info.microphone_sample_rate,
    })
}

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard;

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        unsafe { crate::DeinitializeStreaming() };

        let settings = SERVER_DATA_MANAGER.lock().session().to_settings();

        let on_disconnect_script = settings.connection.on_disconnect_script;
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

async fn connection_pipeline() -> StrResult {
    let mut trusted_discovered_client_id = None;
    let connection_info = loop {
        let client_discovery_config = SERVER_DATA_MANAGER
            .lock()
            .session()
            .to_settings()
            .connection
            .client_discovery;

        let try_connection_future: BoxFuture<Either<StrResult<ClientId>, _>> =
            if let (Switch::Enabled(config), None) =
                (client_discovery_config, &trusted_discovered_client_id)
            {
                Box::pin(async move {
                    let either = futures::future::select(
                        Box::pin(client_discovery(config.auto_trust_clients)),
                        Box::pin(client_handshake(None)),
                    )
                    .await;

                    match either {
                        Either::Left((res, _)) => Either::Left(res),
                        Either::Right((res, _)) => Either::Right(res),
                    }
                })
            } else {
                Box::pin(async {
                    Either::Right(client_handshake(trusted_discovered_client_id.clone()).await)
                })
            };

        tokio::select! {
            res = try_connection_future => {
                match res {
                    Either::Left(Ok(client_ip)) => {
                        trusted_discovered_client_id = Some(client_ip);
                    }
                    Either::Left(Err(e)) => {
                        error!("Client discovery failed: {e}");
                        return Ok(())
                    }
                    Either::Right(Ok(connection_info)) => {
                        break connection_info;
                    }
                    Either::Right(Err(e)) => {
                        // do not treat handshake problems as an hard error
                        warn!("Handshake: {e}");
                        return Ok(());
                    }
                }
            }
            _ = CLIENTS_UPDATED_NOTIFIER.notified() => return Ok(()),
        };

        time::sleep(CLEANUP_PAUSE).await;
    };

    let ConnectionInfo {
        client_ip,
        version: _,
        control_sender,
        mut control_receiver,
        microphone_sample_rate,
    } = connection_info;
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

    let settings = SERVER_DATA_MANAGER.lock().session().to_settings();

    let stream_socket = tokio::select! {
        res = StreamSocketBuilder::connect_to_client(
            client_ip,
            settings.connection.stream_port,
            settings.connection.stream_protocol,
            mbits_to_bytes(settings.video.encode_bitrate_mbs)
        ) => res?,
        _ = time::sleep(Duration::from_secs(5)) => {
            return fmt_e!("Timeout while setting up streams");
        }
    };
    let stream_socket = Arc::new(stream_socket);

    *STATISTICS_MANAGER.lock() = Some(StatisticsManager::new(
        settings.connection.statistics_history_size as _,
    ));

    alvr_events::send_event(EventType::ClientConnected);

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

    unsafe { crate::InitializeStreaming() };
    let _stream_guard = StreamCloseGuard;

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.game_audio {
        let device = AudioDevice::new(
            Some(settings.audio.linux_backend),
            desc.device_id,
            AudioDeviceType::Output,
        )?;
        let sender = stream_socket.request_stream(AUDIO).await?;
        let mute_when_streaming = desc.mute_when_streaming;

        Box::pin(async move {
            #[cfg(windows)]
            unsafe {
                let device_id = alvr_audio::get_windows_device_id(&device)?;
                crate::SetOpenvrProperty(
                    *HEAD_ID,
                    crate::to_cpp_openvr_prop(
                        OpenvrPropertyKey::AudioDefaultPlaybackDeviceId,
                        OpenvrPropValue::String(device_id),
                    ),
                )
            }

            alvr_audio::record_audio_loop(device, 2, mute_when_streaming, sender).await?;

            #[cfg(windows)]
            {
                let default_device = AudioDevice::new(
                    None,
                    alvr_session::AudioDeviceId::Default,
                    AudioDeviceType::Output,
                )?;
                let default_device_id = alvr_audio::get_windows_device_id(&default_device)?;

                unsafe {
                    crate::SetOpenvrProperty(
                        *HEAD_ID,
                        crate::to_cpp_openvr_prop(
                            OpenvrPropertyKey::AudioDefaultPlaybackDeviceId,
                            OpenvrPropValue::String(default_device_id),
                        ),
                    )
                }
            }

            Ok(())
        })
    } else {
        Box::pin(future::pending())
    };

    let microphone_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.microphone {
        let input_device = AudioDevice::new(
            Some(settings.audio.linux_backend),
            desc.input_device_id,
            AudioDeviceType::VirtualMicrophoneInput,
        )?;
        let receiver = stream_socket.subscribe_to_stream(AUDIO).await?;

        #[cfg(windows)]
        {
            let microphone_device = AudioDevice::new(
                None,
                desc.output_device_id,
                AudioDeviceType::VirtualMicrophoneOutput {
                    matching_input_device_name: input_device.name()?,
                },
            )?;
            let microphone_device_id = alvr_audio::get_windows_device_id(&microphone_device)?;
            unsafe {
                crate::SetOpenvrProperty(
                    *HEAD_ID,
                    crate::to_cpp_openvr_prop(
                        OpenvrPropertyKey::AudioDefaultRecordingDeviceId,
                        OpenvrPropValue::String(microphone_device_id),
                    ),
                )
            }
        }

        Box::pin(alvr_audio::play_audio_loop(
            input_device,
            1,
            microphone_sample_rate,
            desc.buffering_config,
            receiver,
        ))
    } else {
        Box::pin(future::pending())
    };

    let video_send_loop = {
        let mut socket_sender = stream_socket.request_stream(VIDEO).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *VIDEO_SENDER.lock() = Some(data_sender);

            while let Some((header, data)) = data_receiver.recv().await {
                let mut buffer = socket_sender.new_buffer(&header, data.len())?;
                buffer.get_mut().extend(data);
                socket_sender.send_buffer(buffer).await.ok();
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
                socket_sender
                    .send_buffer(socket_sender.new_buffer(&haptics, 0)?)
                    .await
                    .ok();
            }

            Ok(())
        }
    };

    let (playspace_sync_sender, playspace_sync_receiver) = smpsc::channel::<Vec2>();

    let is_tracking_ref_only = settings.headset.tracking_ref_only;
    if !is_tracking_ref_only {
        // use a separate thread because SetChaperone() is blocking
        thread::spawn(move || {
            while let Ok(packet) = playspace_sync_receiver.recv() {
                let width = f32::max(packet.x, 2.0);
                let height = f32::max(packet.y, 2.0);
                unsafe { crate::SetChaperone(width, height) };
            }
        });
    }

    fn to_tracking_quat(quat: Quat) -> AlvrQuat {
        AlvrQuat {
            x: quat.x,
            y: quat.y,
            z: quat.z,
            w: quat.w,
        }
    }

    let tracking_receive_loop = {
        let mut receiver = stream_socket
            .subscribe_to_stream::<Tracking>(TRACKING)
            .await?;
        async move {
            let controller_prediction_multiplier = settings
                .headset
                .controllers
                .clone()
                .into_option()
                .map(|c| c.prediction_multiplier)
                .unwrap_or_default();
            let tracking_manager = TrackingManager::new(settings.headset);
            loop {
                let tracking = receiver.recv().await?.header;

                let mut device_motions = vec![];
                for (id, motion) in tracking.device_motions {
                    let motion = if id == *HEAD_ID {
                        tracking_manager.map_head(motion)
                    } else if let Some(motion) = tracking_manager.map_controller(motion) {
                        motion
                    } else {
                        continue;
                    };
                    device_motions.push((id, motion));
                }

                let raw_motions = device_motions
                    .into_iter()
                    .map(|(id, motion)| AlvrDeviceMotion {
                        deviceID: id,
                        orientation: to_tracking_quat(motion.orientation),
                        position: motion.position.to_array(),
                        linearVelocity: motion.linear_velocity.to_array(),
                        angularVelocity: motion.angular_velocity.to_array(),
                    })
                    .collect::<Vec<_>>();

                let left_oculus_hand = if let Some(arr) = tracking.left_hand_skeleton {
                    let vec = arr.into_iter().map(to_tracking_quat).collect::<Vec<_>>();
                    let mut array = [AlvrQuat::default(); 19];
                    array.copy_from_slice(&vec);

                    OculusHand {
                        enabled: true,
                        boneRotations: array,
                    }
                } else {
                    OculusHand {
                        enabled: false,
                        ..Default::default()
                    }
                };

                let right_oculus_hand = if let Some(arr) = tracking.right_hand_skeleton {
                    let vec = arr.into_iter().map(to_tracking_quat).collect::<Vec<_>>();
                    let mut array = [AlvrQuat::default(); 19];
                    array.copy_from_slice(&vec);

                    OculusHand {
                        enabled: true,
                        boneRotations: array,
                    }
                } else {
                    OculusHand {
                        enabled: false,
                        ..Default::default()
                    }
                };

                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    stats.report_tracking_received(tracking.target_timestamp);

                    let prediction_s = stats.average_total_latency().as_secs_f32()
                        * controller_prediction_multiplier;

                    unsafe {
                        crate::SetTracking(
                            tracking.target_timestamp.as_nanos() as _,
                            prediction_s,
                            raw_motions.as_ptr(),
                            raw_motions.len() as _,
                            left_oculus_hand,
                            right_oculus_hand,
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
                let client_stats = receiver.recv().await?.header;

                if let Some(stats) = &mut *STATISTICS_MANAGER.lock() {
                    let game_frame_interval =
                        Duration::from_nanos(unsafe { crate::GetGameFrameIntervalNs() });
                    let network_latency =
                        stats.report_statistics(client_stats, game_frame_interval);
                    unsafe { crate::ReportNetworkLatency(network_latency.as_micros() as _) };
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
                    alvr_events::send_event(EventType::ClientDisconnected);
                    info!("Client disconnected. Cause: {e}");
                    break Ok(());
                }
                time::sleep(NETWORK_KEEPALIVE_INTERVAL).await;
            }
        }
    };

    let control_loop = async move {
        loop {
            match control_receiver.recv().await {
                Ok(ClientControlPacket::PlayspaceSync(packet)) => {
                    if !is_tracking_ref_only {
                        playspace_sync_sender.send(packet).ok();
                    }
                }
                Ok(ClientControlPacket::RequestIdr) => unsafe { crate::RequestIDR() },
                Ok(ClientControlPacket::VideoErrorReport) => unsafe {
                    crate::VideoErrorReportReceive()
                },
                Ok(ClientControlPacket::ViewsConfig(config)) => unsafe {
                    crate::SetViewsConfig(crate::ViewsConfigData {
                        fov: [
                            EyeFov {
                                left: config.fov[0].left,
                                right: config.fov[0].right,
                                top: config.fov[0].top,
                                bottom: config.fov[0].bottom,
                            },
                            EyeFov {
                                left: config.fov[1].left,
                                right: config.fov[1].right,
                                top: config.fov[1].top,
                                bottom: config.fov[1].bottom,
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
                Ok(ClientControlPacket::Button { path_id, value }) => {
                    if settings.extra.log_button_presses {
                        alvr_events::send_event(EventType::Button(ButtonEvent {
                            path: BUTTON_PATH_FROM_ID
                                .get(&path_id)
                                .cloned()
                                .unwrap_or_else(|| format!("Unknown (ID: {:#16x})", path_id)),
                            value: value.clone(),
                        }));
                    }

                    let value = match value {
                        ButtonValue::Binary(value) => AlvrButtonValue {
                            type_: AlvrButtonType_BUTTON_TYPE_BINARY,
                            __bindgen_anon_1: AlvrButtonValue__bindgen_ty_1 { binary: value },
                        },

                        ButtonValue::Scalar(value) => AlvrButtonValue {
                            type_: AlvrButtonType_BUTTON_TYPE_SCALAR,
                            __bindgen_anon_1: AlvrButtonValue__bindgen_ty_1 { scalar: value },
                        },
                    };

                    unsafe { crate::SetButton(path_id, value) };
                }
                Ok(_) => (),
                Err(e) => {
                    alvr_events::send_event(EventType::ClientDisconnected);
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
            alvr_events::send_event(EventType::ClientDisconnected);
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

pub async fn connection_lifecycle_loop() {
    loop {
        tokio::join!(
            async {
                alvr_common::show_err(connection_pipeline().await);

                // let any running task or socket shutdown
                time::sleep(CLEANUP_PAUSE).await;
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL),
        );
    }
}
