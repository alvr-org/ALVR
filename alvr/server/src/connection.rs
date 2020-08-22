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

async fn create_control_socket(
    clients_data: HashMap<IpAddr, Identity>,
    settings: Settings,
) -> (
    Identity,
    ControlSocket<ClientControlPacket, ServerControlPacket>,
) {
    loop {
        let maybe_control_socket = ControlSocket::connect_to_client(
            &clients_data.keys().cloned().collect::<Vec<_>>(),
            |server_config: ServerConfigPacket, server_ip| {
                let eye_width;
                let eye_height;
                match settings.video.render_resolution {
                    FrameSize::Scale(scale) => {
                        let (native_eye_width, native_eye_height) =
                            server_config.native_eye_resolution;
                        eye_width = native_eye_width as f32 * scale;
                        eye_height = native_eye_height as f32 * scale;
                    }
                    FrameSize::Absolute { width, height } => {
                        eye_width = width as f32 / 2_f32;
                        eye_height = height as f32 / 2_f32;
                    }
                }
                let eye_resolution = (align32(eye_width), align32(eye_height));

                let web_gui_url = format!(
                    "http://{}:{}/",
                    server_ip, settings.connection.web_server_port
                );

                ClientConfigPacket {
                    settings: settings.clone(),
                    eye_resolution,
                    web_gui_url,
                }
            },
        )
        .await;

        match maybe_control_socket {
            Ok(control_socket) => {
                let identity = clients_data.get(&control_socket.peer_ip()).unwrap().clone();
                break (identity, control_socket);
            }
            Err(e) => warn!("{}", e),
        }
    }
}

async fn setup_streams(
    settings: Settings,
    client_identity: Identity,
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

    if let Switch::Enabled(audio_desc) = settings.audio.game_audio {
        *MAYBE_AUDIO_SENDER.lock() = Some(trace_err!(
            stream_socket
                .request_stream(StreamId::Audio, audio_desc.stream_mode)
                .await
        )?);
    }

    if let Switch::Enabled(controllers_desc) = settings.headset.controllers {
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

    Ok(())
}

pub async fn connection_loop(
    session_manager: Arc<AMutex<SessionManager>>,
    update_client_listeners_notifier: broadcast::Sender<()>,
) -> StrResult {
    // Some settings cannot be applied right away because they were used to initialize some key
    // driver components. For these settings, send the cached values to the client.
    let settings_cache = session_manager.lock().await.get().to_settings();

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

                Err::<(), _>(res.err().unwrap())
            }
        };

        let clients_data = session_manager.lock().await.get().last_clients.iter().fold(
            HashMap::new(),
            |mut clients_data, (hostname, client)| {
                let id = Identity {
                    hostname: hostname.clone(),
                    certificate_pem: client.certificate_pem.clone(),
                };
                clients_data.extend(client.manual_ips.iter().map(|&ip| (ip, id.clone())));
                clients_data.insert(client.last_ip, id);
                clients_data
            },
        );
        let get_control_socket = create_control_socket(clients_data, settings_cache.clone());

        let (identity, mut control_socket) = tokio::select! {
            Err(e) = client_discovery => break trace_str!("Client discovery failed: {}", e),
            pair = get_control_socket => pair,
            _ = update_client_listeners_receiver.recv() => continue,
        };

        if let Err(e) = setup_streams(settings_cache.clone(), identity, &control_socket).await {
            warn!("Setup streams failed: {}", e);
            continue;
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
