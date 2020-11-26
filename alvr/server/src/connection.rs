use crate::SESSION_MANAGER;
use alvr_common::{data::*, logging::*, *};
use settings_schema::Switch;
use std::time::{Duration, Instant, SystemTime};

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

const STREAM_RETRY_COOLDOWN: Duration = Duration::from_secs(5);

pub async fn client_discovery() {
    let mut stream_init_timestamp = Instant::now() - Duration::from_secs(10);

    let res = sockets::search_client(None, |address, client_handshake_packet| {
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
                    return None;
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

                return None;
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
                video_width = width;
                video_height = height;
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
            server_handshake_packet.web_gui_url = [0; 32];
            let url_string = format!("http://{}:{}/", host_address, 8082);
            let url_c_string = std::ffi::CString::new(url_string).unwrap();
            let url_bytes = url_c_string.as_bytes_with_nul();
            server_handshake_packet.web_gui_url[0..url_bytes.len()].copy_from_slice(url_bytes);

            if Instant::now() - stream_init_timestamp > STREAM_RETRY_COOLDOWN {
                unsafe { crate::InitializeStreaming() };
                stream_init_timestamp = Instant::now();
            }

            Some(server_handshake_packet)
        } else {
            None
        }
    })
    .await;

    if let Err(e) = res {
        show_err::<(), _>(trace_str!("Error while listening for client: {}", e)).ok();
    }
}
