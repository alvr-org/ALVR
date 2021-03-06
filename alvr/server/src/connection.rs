use crate::{
    openvr, ClientListAction, CLIENTS_UPDATED_NOTIFIER, MAYBE_LEGACY_SENDER, RESTART_NOTIFIER,
    SESSION_MANAGER,
};
use alvr_common::{audio::AudioDevice, data::*, logging::*, sockets::*, *};
use audio::AudioDeviceType;
use futures::future::BoxFuture;
use nalgebra::Translation3;
use settings_schema::Switch;
use std::{
    collections::HashMap,
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

const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(2);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

fn mbits_to_bytes(value: u64) -> u32 {
    (value * 1024 * 1024 / 8) as u32
}

async fn client_discovery() -> StrResult {
    let res = search_client_loop(|client_ip, handshake_packet| async move {
        crate::update_client_list(
            handshake_packet.hostname.clone(),
            ClientListAction::AddIfMissing {
                device_name: handshake_packet.device_name,
                ip: client_ip,
                certificate_pem: Some(handshake_packet.certificate_pem),
            },
        )
        .await;

        if let Some(connection_desc) = SESSION_MANAGER
            .lock()
            .get()
            .client_connections
            .get(&handshake_packet.hostname)
        {
            connection_desc.trusted
        } else {
            false
        }
    })
    .await;

    Err(res.err().unwrap_or_else(|| "".into()))
}

struct ConnectionInfo {
    client_ip: IpAddr,
    version: Option<Version>,
    control_sender: ControlSocketSender<ServerControlPacket>,
    control_receiver: ControlSocketReceiver<ClientControlPacket>,
}

async fn client_handshake() -> StrResult<ConnectionInfo> {
    let auto_trust_clients = SESSION_MANAGER
        .lock()
        .get()
        .to_settings()
        .connection
        .auto_trust_clients;
    let clients_info = SESSION_MANAGER
        .lock()
        .get()
        .client_connections
        .iter()
        .filter(|(_, client)| client.trusted || auto_trust_clients)
        .fold(HashMap::new(), |mut clients_info, (hostname, client)| {
            let id = PublicIdentity {
                hostname: hostname.clone(),
                certificate_pem: client.certificate_pem.clone(),
            };
            clients_info.extend(client.manual_ips.iter().map(|&ip| (ip, id.clone())));
            clients_info.insert(client.last_local_ip, id);
            clients_info
        });

    let maybe_pending_connection =
        sockets::begin_connecting_to_client(&clients_info.keys().cloned().collect::<Vec<_>>())
            .await;
    let PendingClientConnection {
        pending_socket,
        client_ip,
        server_ip,
        headset_info,
    } = maybe_pending_connection?;

    let settings = SESSION_MANAGER.lock().get().to_settings();

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
        warn!("Chosen refresh rate not supported. Using {}Hz", fps);
    }

    let dashboard_url = format!(
        "http://{}:{}/",
        server_ip, settings.connection.web_server_port
    );

    let game_audio_sample_rate = if let Switch::Enabled(game_audio_desc) = settings.audio.game_audio
    {
        let game_audio_device =
            AudioDevice::new(game_audio_desc.device_id, AudioDeviceType::Output)?;

        if let Switch::Enabled(microphone_desc) = settings.audio.microphone {
            let microphone_device = AudioDevice::new(
                microphone_desc.input_device_id,
                AudioDeviceType::VirtualMicrophoneInput,
            )?;
            if audio::is_same_device(&game_audio_device, &microphone_device) {
                return fmt_e!("Game audio and microphone cannot point to the same device!");
            }
        }

        trace_err!(audio::get_sample_rate(&game_audio_device))?
    } else {
        0
    };

    let version = Version::from_str(&headset_info.reserved).ok();

    let client_config = ClientConfigPacket {
        session_desc: trace_err!(serde_json::to_string(SESSION_MANAGER.lock().get()))?,
        dashboard_url,
        eye_resolution_width: video_eye_width,
        eye_resolution_height: video_eye_height,
        fps,
        game_audio_sample_rate,
        reserved: format!("{}", *ALVR_VERSION),
    };

    let (mut control_sender, control_receiver) =
        sockets::finish_connecting_to_client(pending_socket, client_config).await?;

    let session_settings = SESSION_MANAGER.lock().get().session_settings.clone();

    let controller_pose_offset = match settings.headset.controllers {
        Switch::Enabled(content) => {
            if content.clientside_prediction {
                0.
            } else {
                content.pose_time_offset
            }
        }
        Switch::Disabled => 0.,
    };

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
        aggressive_keyframe_resend: settings.connection.aggressive_keyframe_resend,
        adapter_index: settings.video.adapter_index,
        codec: matches!(settings.video.codec, CodecType::HEVC) as _,
        refresh_rate: fps as _,
        use_10bit_encoder: settings.video.use_10bit_encoder,
        encode_bitrate_mbs: settings.video.encode_bitrate_mbs,
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
        controllers_type: session_settings
            .headset
            .controllers
            .content
            .ctrl_type
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
        tracking_frame_offset: settings.headset.tracking_frame_offset,
        controller_pose_offset,
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
        enable_foveated_rendering: session_settings.video.foveated_rendering.enabled,
        foveation_strength: session_settings.video.foveated_rendering.content.strength,
        foveation_shape: session_settings.video.foveated_rendering.content.shape,
        foveation_vertical_offset: session_settings
            .video
            .foveated_rendering
            .content
            .vertical_offset,
        enable_color_correction: session_settings.video.color_correction.enabled,
        brightness: session_settings.video.color_correction.content.brightness,
        contrast: session_settings.video.color_correction.content.contrast,
        saturation: session_settings.video.color_correction.content.saturation,
        gamma: session_settings.video.color_correction.content.gamma,
        sharpening: session_settings.video.color_correction.content.sharpening,
    };

    if SESSION_MANAGER.lock().get().openvr_config != new_openvr_config {
        SESSION_MANAGER
            .lock()
            .get_mut(None, SessionUpdateType::Other)
            .openvr_config = new_openvr_config;

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
    })
}

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard;

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            crate::DeinitializeStreaming()
        };

        let settings = SESSION_MANAGER.lock().get().to_settings();

        let on_disconnect_script = settings.connection.on_disconnect_script;
        if !on_disconnect_script.is_empty() {
            info!(
                "Running on disconnect script (disconnect): {}",
                on_disconnect_script
            );
            if let Err(e) = Command::new(&on_disconnect_script)
                .env("ACTION", "disconnect")
                .spawn()
            {
                warn!("Failed to run disconnect script: {}", e);
            }
        }
    }
}

async fn connection_pipeline() -> StrResult {
    let connection_info = tokio::select! {
        maybe_info = client_handshake() => {
            match maybe_info {
                Ok(info) => info,
                Err(e) => {
                    // treat handshake problems not as an hard error
                    warn!("Handshake: {}", e);
                    return Ok(());
                }
            }
        }
        Err(e) = client_discovery() => {
            error!("Client discovery failed: {}", e);
            return Ok(());
        }
        _ = CLIENTS_UPDATED_NOTIFIER.notified() => return Ok(()),
        else => unreachable!(),
    };

    let ConnectionInfo {
        client_ip,
        version,
        control_sender,
        mut control_receiver,
    } = connection_info;
    let control_sender = Arc::new(Mutex::new(control_sender));

    control_sender
        .lock()
        .await
        .send(&ServerControlPacket::StartStream)
        .await?;

    if version
        .map(|v| v >= Version::from((15, 1, 0)))
        .unwrap_or(false)
    {
        match control_receiver.recv().await {
            Ok(ClientControlPacket::StreamReady) => {}
            Ok(_) => {
                return fmt_e!("Got unexpected packet waiting for stream ack");
            }
            Err(e) => {
                return fmt_e!("Error while waiting for stream ack: {}", e);
            }
        }
    }

    let settings = SESSION_MANAGER.lock().get().to_settings();

    let mut stream_socket = tokio::select! {
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

    log_id(LogId::ClientConnected);

    {
        let on_connect_script = settings.connection.on_connect_script;

        if !on_connect_script.is_empty() {
            info!("Running on connect script (connect): {}", on_connect_script);
            if let Err(e) = Command::new(&on_connect_script)
                .env("ACTION", "connect")
                .spawn()
            {
                warn!("Failed to run connect script: {}", e);
            }
        }
    }

    #[cfg(windows)]
    unsafe {
        crate::InitializeStreaming()
    };
    let _stream_guard = StreamCloseGuard;

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.game_audio {
        let device = AudioDevice::new(desc.device_id, AudioDeviceType::Output)?;
        let sample_rate = audio::get_sample_rate(&device)?;
        let sender = stream_socket.request_stream(AUDIO).await?;
        let mute_when_streaming = desc.mute_when_streaming;

        Box::pin(async move {
            #[cfg(windows)]
            openvr::set_game_output_audio_device_id(audio::get_windows_device_id(&device)?);

            audio::record_audio_loop(device, 2, sample_rate, mute_when_streaming, sender).await?;

            #[cfg(windows)]
            {
                let default_device =
                    AudioDevice::new(AudioDeviceId::Default, AudioDeviceType::Output)?;
                let default_device_id = audio::get_windows_device_id(&default_device)?;
                openvr::set_game_output_audio_device_id(default_device_id);
            }

            Ok(())
        })
    } else {
        Box::pin(future::pending())
    };

    let microphone_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.microphone {
        let input_device = AudioDevice::new(
            desc.input_device_id,
            AudioDeviceType::VirtualMicrophoneInput,
        )?;
        let receiver = stream_socket.subscribe_to_stream(AUDIO).await?;

        #[cfg(windows)]
        {
            let microphone_device = AudioDevice::new(
                desc.output_device_id,
                AudioDeviceType::VirtualMicrophoneOutput,
            )?;
            let microphone_device_id = audio::get_windows_device_id(&microphone_device)?;
            openvr::set_headset_microphone_audio_device_id(microphone_device_id);
        }

        Box::pin(audio::play_audio_loop(
            input_device,
            1,
            desc.sample_rate,
            desc.config,
            receiver,
        ))
    } else {
        Box::pin(future::pending())
    };

    let legacy_send_loop = {
        let mut socket_sender = stream_socket.request_stream(LEGACY).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *MAYBE_LEGACY_SENDER.lock() = Some(data_sender);

            while let Some(data) = data_receiver.recv().await {
                let mut buffer = socket_sender.new_buffer(&(), data.len())?;
                buffer.get_mut().extend(data);
                socket_sender.send_buffer(buffer).await.ok();
            }

            Ok(())
        }
    };

    let legacy_receive_loop = {
        let mut receiver = stream_socket.subscribe_to_stream::<()>(LEGACY).await?;
        async move {
            loop {
                let mut data = receiver.recv().await?.buffer;

                #[cfg(windows)]
                unsafe {
                    crate::LegacyReceive(data.as_mut_ptr(), data.len() as _)
                };
            }
        }
    };

    let (playspace_sync_sender, playspace_sync_receiver) = smpsc::channel::<PlayspaceSyncPacket>();
    // use a separate thread because SetChaperone() is blocking
    thread::spawn(move || {
        while let Ok(packet) = playspace_sync_receiver.recv() {
            let transform = Translation3::from(packet.position.coords) * packet.rotation;
            // transposition is done to switch from column major to row major
            let matrix_transp = transform.to_matrix().transpose();

            let perimeter_points = if let Some(perimeter_points) = packet.perimeter_points {
                perimeter_points.iter().map(|p| [p[0], p[1]]).collect()
            } else {
                vec![]
            };

            #[cfg(windows)]
            unsafe {
                crate::SetChaperone(
                    matrix_transp.as_ptr(),
                    packet.area_width,
                    packet.area_height,
                    perimeter_points.as_ptr() as _,
                    perimeter_points.len() as _,
                )
            };
        }
    });

    let keepalive_loop = {
        let control_sender = control_sender.clone();
        async move {
            loop {
                let res = control_sender
                    .lock()
                    .await
                    .send(&ServerControlPacket::KeepAlive)
                    .await;
                if let Err(e) = res {
                    log_id(LogId::ClientDisconnected);
                    info!("Client disconnected. Cause: {}", e);
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
                    playspace_sync_sender.send(packet).ok();
                }
                Ok(ClientControlPacket::RequestIDR) => unsafe {
                    #[cfg(windows)]
                    crate::RequestIDR()
                },
                Ok(_) => (),
                Err(e) => {
                    log_id(LogId::ClientDisconnected);
                    info!("Client disconnected. Cause: {}", e);
                    break;
                }
            }
        }

        Ok(())
    };

    // Run many tasks concurrently. Threading is managed by the runtime, for best performance.
    tokio::select! {
        res = spawn_cancelable(stream_socket.receive_loop()) => {
            log_id(LogId::ClientDisconnected);
            if let Err(e) = res {
                info!("Client disconnected. Cause: {}", e);
            }

            Ok(())
        },
        res = spawn_cancelable(game_audio_loop) => res,
        res = spawn_cancelable(microphone_loop) => res,
        res = spawn_cancelable(legacy_send_loop) => res,
        res = spawn_cancelable(legacy_receive_loop) => res,

        // leave these loops on the current task
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
                show_err(connection_pipeline().await);
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL),
        );
    }
}
