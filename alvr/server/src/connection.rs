use crate::SESSION_MANAGER;
use alvr_common::{*, sockets::*, data::*, logging::*};
use settings_schema::Switch;
use std::time::SystemTime;

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

pub async fn client_discovery() {
    let res = search_client(None, |address, client_handshake_packet| {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        {
            let session_manager_ref = &mut SESSION_MANAGER.lock();
            let session_desc_ref =
                &mut session_manager_ref.get_mut(None, SessionUpdateType::ClientList);

            let maybe_known_client_ref =
                session_desc_ref
                    .last_clients
                    .iter_mut()
                    .find(|connection_desc| {
                        connection_desc.address == address.to_string()
                            && connection_desc.handshake_packet.device_name
                                == client_handshake_packet.device_name
                            && connection_desc.handshake_packet.version
                                == client_handshake_packet.version
                    });

            if let Some(known_client_ref) = maybe_known_client_ref {
                known_client_ref.last_update_ms_since_epoch = now_ms as _;

                if matches!(
                    known_client_ref.state,
                    ClientConnectionState::AvailableUntrusted
                ) {
                    return SearchResult::Wait;
                } else {
                    known_client_ref.state = ClientConnectionState::AvailableTrusted;
                }
            } else {
                session_desc_ref.last_clients.push(ClientConnectionDesc {
                    state: ClientConnectionState::AvailableUntrusted,
                    last_update_ms_since_epoch: now_ms as _,
                    address: address.to_string(),
                    handshake_packet: client_handshake_packet,
                });

                return SearchResult::Wait;
            }
        }

        let settings = SESSION_MANAGER.lock().get().to_settings();

        let video_width;
        let video_height;
        match settings.video.render_resolution {
            FrameSize::Scale(scale) => {
                video_width = align32(client_handshake_packet.render_width as f32 * scale);
                video_height = align32(client_handshake_packet.render_height as f32 * scale);
            }
            FrameSize::Absolute { width, height } => {
                video_width = align32(width as f32);
                video_height = align32(height as f32);
            }
        }

        let target_width;
        let target_height;
        match settings.video.render_resolution {
            FrameSize::Scale(scale) => {
                target_width = align32(client_handshake_packet.render_width as f32 * scale);
                target_height = align32(client_handshake_packet.render_height as f32 * scale);
            }
            FrameSize::Absolute { width, height } => {
                target_width = align32(width as f32);
                target_height = align32(height as f32);
            }
        }

        let foveation_mode;
        let foveation_strength;
        let foveation_shape;
        let foveation_vertical_offset;
        if let Switch::Enabled(foveation_data) = settings.video.foveated_rendering {
            foveation_mode = true as u8;
            foveation_strength = foveation_data.strength;
            foveation_shape = foveation_data.shape;
            foveation_vertical_offset = foveation_data.vertical_offset;
        } else {
            foveation_mode = false as u8;
            foveation_strength = 0.;
            foveation_shape = 0.;
            foveation_vertical_offset = 0.;
        }

        let mut server_handshake_packet = ServerHandshakePacket {
            packet_type: 2,
            codec: settings.video.codec as _,
            realtime_decoder: settings.video.client_request_realtime_decoder,
            video_width,
            video_height,
            buffer_size_bytes: settings.connection.client_recv_buffer_size as _,
            frame_queue_size: settings.connection.frame_queue_size as _,
            refresh_rate: settings.video.refresh_rate as _,
            stream_mic: matches!(settings.audio.microphone, Switch::Enabled(_)),
            foveation_mode,
            foveation_strength,
            foveation_shape,
            foveation_vertical_offset,
            tracking_space: settings.headset.tracking_space as _,
            web_gui_url: [0; 32],
        };

        let mut maybe_host_address = None;

        // todo: get the host address using another handshake round instead
        for adapter in ipconfig::get_adapters().expect("PC network adapters") {
            for host_address in adapter.ip_addresses() {
                let address_string = host_address.to_string();
                if address_string.starts_with("192.168.")
                    || address_string.starts_with("10.")
                    || address_string.starts_with("172.")
                {
                    maybe_host_address = Some(*host_address);
                }
            }
        }
        if let Some(host_address) = maybe_host_address {
            let session_manager_ref = &mut *SESSION_MANAGER.lock();
            let session_ref = &mut *session_manager_ref.get_mut(None, SessionUpdateType::Other);
            let session_settings = &session_ref.session_settings;

            let openvr_config = OpenvrConfig {
                headset_serial_number: settings.headset.serial_number,
                headset_tracking_system_name: settings.headset.tracking_system_name,
                headset_model_number: settings.headset.model_number,
                headset_driver_version: settings.headset.driver_version,
                headset_manufacturer_name: settings.headset.manufacturer_name,
                headset_render_model_name: settings.headset.render_model_name,
                headset_registered_device_type: settings.headset.registered_device_type,
                eye_resolution_width: video_width / 2,
                eye_resolution_height: video_height,
                target_eye_resolution_width: target_width / 2,
                target_eye_resolution_height: target_height,
                eye_fov: settings.video.eye_fov,
                enable_game_audio: session_settings.audio.game_audio.enabled,
                game_audio_device: session_settings.audio.game_audio.content.device.clone(),
                enable_microphone: session_settings.audio.microphone.enabled,
                microphone_device: session_settings.audio.microphone.content.device.clone(),
                seconds_from_vsync_to_photons: settings.video.seconds_from_vsync_to_photons,
                ipd: settings.video.ipd,
                client_buffer_size: settings.connection.client_recv_buffer_size,
                frame_queue_size: settings.connection.frame_queue_size,
                force_60hz: settings.video.force_60hz,
                force_3dof: settings.headset.force_3dof,
                aggressive_keyframe_resend: settings.connection.aggressive_keyframe_resend,
                adapter_index: settings.video.adapter_index,
                codec: matches!(settings.video.codec, CodecType::HEVC) as _,
                refresh_rate: settings.video.refresh_rate,
                encode_bitrate_mbs: settings.video.encode_bitrate_mbs,
                throttling_bitrate_bits: settings.connection.throttling_bitrate_bits,
                listen_host: settings.connection.listen_host,
                listen_port: settings.connection.listen_port,
                client_address: address.to_string(),
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

            if session_ref.openvr_config == openvr_config {
                server_handshake_packet.web_gui_url = [0; 32];
                let url_string = format!("http://{}:{}/", host_address, 8082);
                let url_c_string = std::ffi::CString::new(url_string).unwrap();
                let url_bytes = url_c_string.as_bytes_with_nul();
                server_handshake_packet.web_gui_url[0..url_bytes.len()].copy_from_slice(url_bytes);

                unsafe { crate::InitializeStreaming() };

                SearchResult::ClientReady(server_handshake_packet)
            } else {
                session_ref.openvr_config = openvr_config;

                crate::restart_steamvr();

                SearchResult::Exit
            }
        } else {
            SearchResult::Wait
        }
    })
    .await;

    if let Err(e) = res {
        show_err::<(), _>(trace_str!("Error while listening for client: {}", e)).ok();
    }
}
