use crate::*;
use alvr_common::{data::*, logging::*, sockets::*, *};
use nalgebra::{Point3, UnitQuaternion, Vector3};
use settings_schema::Switch;
use std::{collections::HashMap, net::IpAddr, sync::Arc};
use tokio::{sync::broadcast, time};

const CONTROL_SOCKET_RECEIVE_TIMEOUT: Duration = Duration::from_millis(1500);

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
}

fn point3_to_tracking_vector3(pt: &Point3<f32>) -> TrackingVector3 {
    TrackingVector3 {
        x: pt.x,
        y: pt.y,
        z: pt.z,
    }
}
fn vector3_to_tracking_vector3(vec: &Vector3<f32>) -> TrackingVector3 {
    TrackingVector3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}
fn unit_quat_to_tracking_quat(quat: &UnitQuaternion<f32>) -> TrackingQuat {
    TrackingQuat {
        x: quat[0],
        y: quat[1],
        z: quat[2],
        w: quat.w,
    }
}

async fn setup_streams(
    settings: Settings,
    client_identity: PublicIdentity,
    control_socket: &ControlSocket<ClientControlPacket, ServerControlPacket>,
) -> StrResult {
    let mut stream_socket = StreamSocket::connect_to_client(
        control_socket.peer_ip(),
        settings.connection.stream_port,
        client_identity,
        settings.connection.stream_socket_config,
    )
    .await?;

    *MAYBE_VIDEO_SENDER.lock() = Some(trace_err!(
        stream_socket
            .request_stream(StreamId::Video(), settings.video.stream_mode)
            .await
    )?);

    if let Switch::Enabled(audio_desc) = settings.audio.game_audio.clone() {
        *MAYBE_AUDIO_SENDER.lock() = Some(trace_err!(
            stream_socket
                .request_stream(StreamId::Audio, audio_desc.stream_mode)
                .await
        )?);
    }

    if let Switch::Enabled(controllers_desc) = settings.headset.controllers.clone() {
        *MAYBE_HAPTICS_SENDER.lock() = Some(trace_err!(
            stream_socket
                .request_stream(StreamId::Haptics, controllers_desc.haptics_stream_mode)
                .await
        )?);
    }

    let mut input_receiver = trace_err!(
        stream_socket
            .subscribe_to_stream::<InputPacket>(StreamId::Input)
            .await
    )?;

    tokio::spawn(async move {
        loop {
            let maybe_packet = input_receiver.recv().await;
            match maybe_packet {
                Ok(packet) => {
                    let maybe_left_motion = packet
                        .device_motions
                        .get(&TrackedDeviceType::LeftController);
                    let maybe_right_motion = packet
                        .device_motions
                        .get(&TrackedDeviceType::RightController);
                    let motions = if let (Some(left_motion), Some(right_motion)) =
                        (maybe_left_motion, maybe_right_motion)
                    {
                        [left_motion, right_motion]
                    } else {
                        continue;
                    };

                    let controllers = motions
                        .iter()
                        .enumerate()
                        .map(|(idx, motion)| {
                            let mut controller_mut = TrackingInfo_Controller {
                                orientation: unit_quat_to_tracking_quat(&motion.pose.orientation),
                                position: point3_to_tracking_vector3(&motion.pose.position),
                                angularVelocity: vector3_to_tracking_vector3(
                                    &motion.angular_velocity,
                                ),
                                linearVelocity: vector3_to_tracking_vector3(
                                    &motion.linear_velocity,
                                ),
                                ..<_>::default()
                            };

                            if let InputDeviceData::OculusTouchPair(ctrl_pair) = &packet.input_data
                            {
                                let ctrl = &ctrl_pair[idx];
                                controller_mut.trackpadPosition =
                                    TrackingInfo_Controller__bindgen_ty_1 {
                                        x: ctrl.thumbstick_coord.0,
                                        y: ctrl.thumbstick_coord.1,
                                    };
                                controller_mut.triggerValue = ctrl.trigger;
                                controller_mut.gripValue = ctrl.grip;
                                controller_mut.batteryPercentRemaining = ctrl.battery_percentage;
                            }

                            controller_mut.buttons = packet.buttons[idx];

                            let bone_rotations_vec = packet.bone_rotations[idx]
                                .iter()
                                .map(unit_quat_to_tracking_quat)
                                .collect::<Vec<_>>();
                            controller_mut
                                .boneRotations
                                .copy_from_slice(&bone_rotations_vec);

                            let bone_positions_base_vec = packet.bone_positions_base[idx]
                                .iter()
                                .map(point3_to_tracking_vector3)
                                .collect::<Vec<_>>();
                            controller_mut
                                .bonePositionsBase
                                .copy_from_slice(&bone_positions_base_vec);

                            controller_mut.boneRootOrientation =
                                unit_quat_to_tracking_quat(&packet.bone_root_oritentation[idx]);
                            controller_mut.boneRootPosition =
                                point3_to_tracking_vector3(&packet.bone_root_position[idx]);
                            controller_mut.inputStateStatus = packet.input_state_status[idx];
                            controller_mut.fingerPinchStrengths = packet.finger_pinch_strength[idx];
                            controller_mut.handFingerConfidences =
                                packet.hand_finger_confidences[idx];

                            controller_mut
                        })
                        .collect::<Vec<_>>();

                    let tracking_info = TrackingInfo {
                        clientTime: packet.client_time,
                        FrameIndex: packet.frame_index,
                        predictedDisplayTime: packet.input_data_timestamp.as_secs_f64(),
                        HeadPose_Pose_Orientation: unit_quat_to_tracking_quat(
                            &packet.head_motion.pose.orientation,
                        ),
                        HeadPose_Pose_Position: point3_to_tracking_vector3(
                            &packet.head_motion.pose.position,
                        ),
                        controller: [controllers[0], controllers[1]],
                    };

                    unsafe { UpdatePose(tracking_info) };
                }
                Err(e) => debug!("Error while listening for input packet: {}", e),
            }
        }
    });

    if settings.audio.microphone {
        let mut microphone_receiver = trace_err!(
            stream_socket
                .subscribe_to_stream::<AudioPacket>(StreamId::Input)
                .await
        )?;

        tokio::spawn(async move {
            loop {
                let maybe_packet = microphone_receiver.recv().await;
                match maybe_packet {
                    Ok(mut packet) => unsafe {
                        PlayMicAudio(packet.buffer.as_mut_ptr(), packet.buffer.len() as _)
                    },
                    Err(e) => debug!("Error while listening for microphone packet: {}", e),
                }
            }
        });
    }

    let mut game_audio_enabled = false;
    let mut game_audio_device = std::ptr::null_mut();
    if let Switch::Enabled(config) = settings.audio.game_audio {
        game_audio_enabled = true;
        // Note: game_audio_device memory will not be cleaned up but it's not a problem
        game_audio_device = trace_err!(std::ffi::CString::new(config.device))?.into_raw();
    }

    let mut pose_time_offset = 0_f32;
    let mut position_offset_left = [0_f32; 3];
    let mut rotation_offset_left = [0_f32; 3];
    let mut haptics_intensity = 0_f32;
    if let Switch::Enabled(config) = settings.headset.controllers {
        pose_time_offset = config.pose_time_offset;
        position_offset_left = config.position_offset_left;
        rotation_offset_left = config.rotation_offset_left;
        haptics_intensity = config.haptics_intensity;
    }

    unsafe {
        // Initialize components on C++ side
        InitalizeStreaming(StreamSettings {
            gameAudio: game_audio_enabled,
            gameAudioDevice: game_audio_device,
            microphone: settings.audio.microphone,
            keyframeResendIntervalMs: settings.video.keyframe_resend_interval_ms,
            codec: matches!(settings.video.codec, CodecType::Hevc) as i32,
            encodeBitrateMbs: settings.video.encode_bitrate_mbs,
            trackingFrameOffset: settings.headset.tracking_frame_offset,
            poseTimeOffset: pose_time_offset,
            positionOffsetLeft: position_offset_left,
            rotationOffsetLeft: rotation_offset_left,
            hapticsIntensity: haptics_intensity,
        });
    }

    Ok(())
}

async fn connect_to_any_client(
    clients_info: HashMap<IpAddr, PublicIdentity>,
    session_manager: Arc<AMutex<SessionManager>>,
) -> ControlSocket<ClientControlPacket, ServerControlPacket> {
    loop {
        let maybe_pending_connection = ControlSocket::begin_connecting_to_client(
            &clients_info.keys().cloned().collect::<Vec<_>>(),
        )
        .await;
        let PendingClientConnection {
            pending_socket,
            server_ip,
            headset_info,
        } = match maybe_pending_connection {
            Ok(pending_connection) => pending_connection,
            Err(e) => {
                warn!("{}", e);
                continue;
            }
        };

        let settings = session_manager.lock().await.get().to_settings();

        let (eye_width, eye_height) = match settings.video.render_resolution {
            FrameSize::Scale(scale) => {
                let (native_eye_width, native_eye_height) = headset_info.recommended_eye_resolution;
                (
                    native_eye_width as f32 * scale,
                    native_eye_height as f32 * scale,
                )
            }
            FrameSize::Absolute { width, height } => (width as f32 / 2_f32, height as f32 / 2_f32),
        };
        let eye_resolution = (align32(eye_width), align32(eye_height));

        let left_eye_fov = if let Some(left_eye_fov) = settings.video.left_eye_fov.clone() {
            left_eye_fov
        } else {
            headset_info.recommended_left_eye_fov.clone()
        };

        let fps = if let Some(fps) = settings.video.fps {
            let mut best_match = 0;
            let mut min_diff = f32::MAX;
            for rr in &headset_info.available_refresh_rates {
                let diff = (rr - fps as f32).abs();
                if diff < min_diff {
                    best_match = *rr as u32;
                    min_diff = diff;
                }
            }
            best_match
        } else {
            headset_info
                .available_refresh_rates
                .iter()
                .map(|&f| f as u32)
                .max()
                .unwrap()
        };

        let web_gui_url = format!(
            "http://{}:{}/",
            server_ip, settings.connection.web_server_port
        );

        let client_config = ClientConfigPacket {
            settings: serde_json::to_value(&settings).unwrap(),
            eye_resolution,
            left_eye_fov: left_eye_fov.clone(),
            fps,
            web_gui_url,
            reserved: serde_json::json!({}),
        };

        let control_socket =
            match ControlSocket::finish_connecting_to_client(pending_socket, client_config).await {
                Ok(control_socket) => control_socket,
                Err(e) => {
                    warn!("{}", e);
                    continue;
                }
            };

        let mut controllers_tracking_system_name = "".into();
        let mut controllers_manufacturer_name = "".into();
        let mut controllers_model_number = "".into();
        let mut render_model_name_left_controller = "".into();
        let mut render_model_name_right_controller = "".into();
        let mut controllers_serial_number = "".into();
        let mut controllers_type = "".into();
        let mut controllers_registered_device_type = "".into();
        let mut controllers_input_profile_path = "".into();
        let mut controllers_mode_idx = 0;
        let mut controllers_enabled = false;
        if let Switch::Enabled(config) = settings.headset.controllers.clone() {
            controllers_tracking_system_name = config.tracking_system_name;
            controllers_manufacturer_name = config.manufacturer_name;
            controllers_model_number = config.model_number;
            render_model_name_left_controller = config.render_model_name_left;
            render_model_name_right_controller = config.render_model_name_right;
            controllers_serial_number = config.serial_number;
            controllers_type = config.ctrl_type;
            controllers_registered_device_type = config.registered_device_type;
            controllers_input_profile_path = config.input_profile_path;
            controllers_mode_idx = config.mode_idx;
            controllers_enabled = true;
        }

        let mut enable_foveated_rendering = false;
        let mut foveation_strength = 0_f32;
        let mut foveation_shape = 1_f32;
        let mut foveation_vertical_offset = 0_f32;
        if let Switch::Enabled(config) = settings.video.foveated_rendering.clone() {
            enable_foveated_rendering = true;
            foveation_strength = config.strength;
            foveation_shape = config.shape;
            foveation_vertical_offset = config.vertical_offset
        }

        let mut enable_color_correction = false;
        let mut brightness = 0_f32;
        let mut contrast = 0_f32;
        let mut saturation = 0_f32;
        let mut gamma = 1_f32;
        let mut sharpening = 0_f32;
        if let Switch::Enabled(config) = settings.video.color_correction.clone() {
            enable_color_correction = true;
            brightness = config.brightness;
            contrast = config.contrast;
            saturation = config.saturation;
            gamma = config.gamma;
            sharpening = config.sharpening;
        }

        // check that OpenVR has been initialized correctly, otherwise restart SteamVR
        let new_openvr_config = OpenvrConfig {
            headset_serial_number: settings.headset.serial_number.clone(),
            headset_tracking_system_name: settings.headset.tracking_system_name.clone(),
            headset_model_number: settings.headset.model_number.clone(),
            headset_driver_version: settings.headset.driver_version.clone(),
            headset_manufacturer_name: settings.headset.manufacturer_name.clone(),
            headset_render_model_name: settings.headset.render_model_name.clone(),
            headset_registered_device_type: settings.headset.registered_device_type.clone(),
            eye_resolution,
            target_eye_resolution: eye_resolution,
            left_eye_fov,
            seconds_from_vsync_to_photons: settings.video.seconds_from_vsync_to_photons,
            ipd: settings.video.ipd,
            adapter_index: settings.video.adapter_index,
            fps,
            controllers_tracking_system_name,
            controllers_manufacturer_name,
            controllers_model_number,
            render_model_name_left_controller,
            render_model_name_right_controller,
            controllers_serial_number,
            controllers_type,
            controllers_registered_device_type,
            controllers_input_profile_path,
            controllers_mode_idx,
            controllers_enabled,
            enable_foveated_rendering,
            foveation_strength,
            foveation_shape,
            foveation_vertical_offset,
            enable_color_correction,
            brightness,
            contrast,
            saturation,
            gamma,
            sharpening,
        };

        if session_manager.lock().await.get().openvr_config != new_openvr_config {
            session_manager
                .lock()
                .await
                .get_mut(None, SessionUpdateType::Other)
                .openvr_config = new_openvr_config;

            restart_steamvr();
        }

        let identity = clients_info.get(&control_socket.peer_ip()).unwrap().clone();

        if let Err(e) = setup_streams(settings, identity, &control_socket).await {
            warn!("Setup streams failed: {}", e);
        } else {
            break control_socket;
        }
    }
}

pub async fn connection_loop(
    session_manager: Arc<AMutex<SessionManager>>,
    update_client_listeners_notifier: broadcast::Sender<()>,
) -> StrResult {
    loop {
        let mut update_client_listeners_receiver = update_client_listeners_notifier.subscribe();

        let client_discovery = {
            let session_manager = session_manager.clone();
            let update_client_listeners_notifier = update_client_listeners_notifier.clone();
            async move {
                let res = search_client_loop(|client_ip, client_identity| {
                    update_client_list(
                        session_manager.clone(),
                        client_identity.hostname,
                        ClientListAction::AddIfMissing {
                            ip: client_ip,
                            certificate_pem: client_identity.certificate_pem,
                        },
                        update_client_listeners_notifier.clone(),
                    )
                })
                .await;

                Err::<(), _>(res.err().unwrap_or_else(|| "".into()))
            }
        };

        let clients_info = session_manager
            .lock()
            .await
            .get()
            .client_connections
            .iter()
            .fold(HashMap::new(), |mut clients_info, (hostname, client)| {
                let id = PublicIdentity {
                    hostname: hostname.clone(),
                    certificate_pem: client.certificate_pem.clone(),
                };
                clients_info.extend(client.manual_ips.iter().map(|&ip| (ip, id.clone())));
                clients_info.insert(client.last_ip, id);
                clients_info
            });
        let get_control_socket = connect_to_any_client(clients_info, session_manager.clone());

        let mut control_socket = tokio::select! {
            Err(e) = client_discovery => break trace_str!("Client discovery failed: {}", e),
            control_socket = get_control_socket => control_socket,
            _ = update_client_listeners_receiver.recv() => continue,
        };

        loop {
            tokio::select! {
                maybe_packet = control_socket.recv() => match maybe_packet {
                    Ok(ClientControlPacket::Statistics(client_statistics)) => {
                        STATISTICS.lock().update(client_statistics);
                    }
                    Ok(ClientControlPacket::PlayspaceSync(playspace_sync_packet)) => {
                        let position = point3_to_tracking_vector3(&playspace_sync_packet.position);
                        let rotation = unit_quat_to_tracking_quat(&playspace_sync_packet.rotation);
                        let space_rectangle = TrackingVector2 {
                            x: playspace_sync_packet.space_rectangle.0,
                            y: playspace_sync_packet.space_rectangle.1,
                        };
                        let mut points = playspace_sync_packet
                            .points
                            .iter()
                            .map(point3_to_tracking_vector3)
                            .collect::<Vec<_>>();

                        unsafe {
                            UpdateChaperone(
                                position,
                                rotation,
                                space_rectangle,
                                points.as_mut_ptr(),
                                points.len() as _
                            );
                        }
                    }
                    Ok(ClientControlPacket::RequestIdrFrame) => unsafe { HandlePacketLoss() },
                    Ok(ClientControlPacket::Disconnect) => {
                        info!(id: LogId::ClientDisconnected, "Client disconnected gracefully");
                        break;
                    }
                    Ok(ClientControlPacket::Reserved(_)) => (),
                    Err(e) => {
                        warn!(
                            id: LogId::ClientDisconnected,
                            "Error while listening for control packet: {}",
                            e
                        );
                        break;
                    }
                },
                _ = time::delay_for(CONTROL_SOCKET_RECEIVE_TIMEOUT) => {
                    warn!(id: LogId::ClientDisconnected, "CLient lost");
                    break;
                }
            };
        }
    }
}
