use crate::{
    restart_steamvr, update_client_list, ClientListAction, CLIENTS_UPDATED_NOTIFIER,
    SESSION_MANAGER,
};
use alvr_common::{data::*, logging::*, sockets::*, *};
use std::{collections::HashMap, net::IpAddr};

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

async fn client_discovery() -> StrResult {
    let res = search_client_loop(|client_ip, handshake_packet| async move {
        update_client_list(
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

async fn connect_to_any_client(
    clients_info: HashMap<IpAddr, PublicIdentity>,
) -> (
    ControlSocketSender<ServerControlPacket>,
    ControlSocketReceiver<ClientControlPacket>,
) {
    loop {
        let maybe_pending_connection =
            sockets::begin_connecting_to_client(&clients_info.keys().cloned().collect::<Vec<_>>())
                .await;
        let PendingClientConnection {
            pending_socket,
            client_ip,
            server_ip,
            headset_info,
        } = match maybe_pending_connection {
            Ok(pending_connection) => pending_connection,
            Err(e) => {
                warn!("{}", e);
                continue;
            }
        };

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

        let web_gui_url = format!(
            "http://{}:{}/",
            server_ip, settings.connection.web_server_port
        );

        let client_config = ClientConfigPacket {
            session_desc: serde_json::to_string(SESSION_MANAGER.lock().get()).unwrap(),
            eye_resolution_width: video_eye_width,
            eye_resolution_height: video_eye_height,
            fps,
            web_gui_url,
            reserved: "".into(),
        };

        let (mut sender, receiver) =
            match sockets::finish_connecting_to_client(pending_socket, client_config).await {
                Ok(control_socket) => control_socket,
                Err(e) => {
                    warn!("{}", e);
                    continue;
                }
            };

        let session_settings = SESSION_MANAGER.lock().get().session_settings.clone();

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
            enable_game_audio: session_settings.audio.game_audio.enabled,
            game_audio_device: session_settings.audio.game_audio.content.device.clone(),
            mute_host_audio_output: session_settings.audio.game_audio.content.mute_when_streaming,
            enable_microphone: session_settings.audio.microphone.enabled,
            microphone_device: session_settings.audio.microphone.content.device.clone(),
            seconds_from_vsync_to_photons: settings.video.seconds_from_vsync_to_photons,
            ipd: settings.video.ipd,
            client_buffer_size: settings.connection.client_recv_buffer_size,
            force_3dof: settings.headset.force_3dof,
            aggressive_keyframe_resend: settings.connection.aggressive_keyframe_resend,
            adapter_index: settings.video.adapter_index,
            codec: matches!(settings.video.codec, CodecType::HEVC) as _,
            refresh_rate: fps as _,
            encode_bitrate_mbs: settings.video.encode_bitrate_mbs,
            throttling_bitrate_bits: settings.connection.throttling_bitrate_bits,
            listen_port: settings.connection.listen_port,
            client_address: client_ip.to_string(),
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
            controller_pose_offset: session_settings
                .headset
                .controllers
                .content
                .pose_time_offset,
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

            sender.send(&ServerControlPacket::Restarting).await.ok();

            restart_steamvr();

            // waiting for execution canceling
            std::future::pending::<()>().await;
        }

        break (sender, receiver);
    }
}

async fn pairing_loop() -> (
    ControlSocketSender<ServerControlPacket>,
    ControlSocketReceiver<ClientControlPacket>,
) {
    loop {
        let clients_info = SESSION_MANAGER
            .lock()
            .get()
            .client_connections
            .iter()
            .filter(|(_, client)| client.trusted)
            .fold(HashMap::new(), |mut clients_info, (hostname, client)| {
                let id = PublicIdentity {
                    hostname: hostname.clone(),
                    certificate_pem: client.certificate_pem.clone(),
                };
                clients_info.extend(client.manual_ips.iter().map(|&ip| (ip, id.clone())));
                clients_info.insert(client.last_local_ip, id);
                clients_info
            });

        tokio::select! {
            control_socket = connect_to_any_client(clients_info) => break control_socket,
            _ = CLIENTS_UPDATED_NOTIFIER.notified() => continue,
        }
    }
}

pub async fn connection_lifecycle_loop() -> StrResult {
    loop {
        let (mut control_sender, mut control_receiver) = tokio::select! {
            Err(e) = client_discovery() => break trace_str!("Client discovery failed: {}", e),
            control_socket = pairing_loop() => control_socket,
            else => unreachable!(),
        };

        info!(id: LogId::ClientConnected);

        unsafe { crate::InitializeStreaming() };

        loop {
            tokio::select! {
                _ = crate::RESTART_NOTIFIER.notified() => {
                    control_sender.send(&ServerControlPacket::Restarting).await.ok();
                    return Ok(());
                }
                maybe_packet = control_receiver.recv() => match maybe_packet {
                    Ok(ClientControlPacket::RequestIDR) => unsafe { crate::RequestIDR() },
                    Ok(ClientControlPacket::Reserved(_))
                    | Ok(ClientControlPacket::ReservedBuffer(_)) => (),
                    Err(e) => {
                        info!(id: LogId::ClientDisconnected, "Cause: {}", e);
                        break;
                    }
                }
            }
        }
    }
}
