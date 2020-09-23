use crate::*;
use alvr_common::{data::*, logging::*, sockets::*, *};
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3};
use settings_schema::Switch;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time;

const STATISTICS_SEND_INTERVAL: Duration = Duration::from_secs(1);
const INPUT_SEND_INTERVAL: Duration = Duration::from_millis(8);

async fn stopStream(
    java_vm: &JavaVM,
    activity_ref: &GlobalRef,
    control_socket: &mut ControlSocket<ServerControlPacket, ClientControlPacket>,
    restart: bool,
) -> StrResult {
    control_socket
        .send(ClientControlPacket::Disconnect)
        .await
        .ok();

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        activity_ref,
        "onStreamStop",
        "(Z)V",
        &[restart.into()],
    ))?;

    Ok(())
}

async fn try_connect(
    headset_info: &HeadsetInfoPacket,
    private_identity: &PrivateIdentity,
    on_stream_stop_notifier: broadcast::Sender<()>,
    java_vm: Arc<JavaVM>,
    activity_ref: Arc<GlobalRef>,
) -> StrResult {
    let maybe_connection_res = trace_err!(
        ControlSocket::connect_to_server(
            &headset_info,
            private_identity.hostname.clone(),
            private_identity.certificate_pem.clone(),
        )
        .await
    )?;

    let (mut control_socket, client_config) = if let Some(pair) = maybe_connection_res {
        pair
    } else {
        // Note: env = java_vm.attach_current_thread() cannot be saved into a variable because it is
        // not Send (compile error). This makes sense since tokio could move the execution of this
        // task to another thread at any time, and env is valid only within a specific thread. For
        // the same reason, other jni objects cannot be made into variables and the arguments must
        // be created inline within the call_method() call
        trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
            &*activity_ref,
            "onServerFound",
            "(ZLjava/lang/String;I)V",
            &[
                false.into(),
                trace_err!(trace_err!(java_vm.attach_current_thread())?.new_string(""))?.into(),
                0_i32.into(),
            ],
        ))?;

        return trace_str!("Found unsupported server");
    };

    // todo: go through session representation. this requires settings -> session representation
    // conversion code
    let settings = trace_err!(serde_json::from_str::<Settings>(&client_config.settings))?;

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        &*activity_ref,
        "onServerFound",
        "(ZLjava/lang/String;I)V",
        &[
            true.into(),
            trace_err!(
                trace_err!(java_vm.attach_current_thread())?.new_string(client_config.web_gui_url)
            )?
            .into(),
            match settings.video.codec {
                CodecType::H264 => 0_i32,
                CodecType::Hevc => 1_i32,
            }
            .into()
        ],
    ))?;

    let mut stream_socket = StreamSocket::connect_to_server(
        control_socket.peer_ip(),
        settings.connection.stream_port,
        private_identity.certificate_pem.clone(),
        private_identity.key_pem.clone(),
        settings.connection.stream_socket_config,
    )
    .await?;

    let mut input_sender = stream_socket
        .request_stream::<InputPacket>(StreamId::Input, settings.headset.tracking_stream_mode)
        .await?;

    let mut maybe_microphone_sender = if settings.audio.microphone {
        Some(
            stream_socket
                .request_stream::<AudioPacket>(
                    StreamId::Audio,
                    settings.audio.microphone_stream_mode,
                )
                .await?,
        )
    } else {
        None
    };

    let video_receiver = stream_socket
        .subscribe_to_stream::<VideoPacket>(StreamId::Video())
        .await?;
    let video_loop = video::receive_and_process_frames_loop(
        java_vm.clone(),
        activity_ref.clone(),
        video_receiver,
        settings.video.codec,
    );
    let mut on_stream_stop_receiver = on_stream_stop_notifier.subscribe();
    tokio::spawn(async move {
        tokio::select! {
            _ = show_err_async(video_loop) => (),
            _ = on_stream_stop_receiver.recv() => (),
        };
    });

    if matches!(settings.audio.game_audio, Switch::Enabled(_)) {
        let mut audio_receiver = stream_socket
            .subscribe_to_stream::<AudioPacket>(StreamId::Audio)
            .await?;
        let mut on_stream_stop_receiver = on_stream_stop_notifier.subscribe();
        tokio::spawn(async move {
            loop {
                let packet = tokio::select! {
                    Ok(packet) = audio_receiver.recv() => packet,
                    _ = on_stream_stop_receiver.recv() => break,
                    else => break,
                };

                unsafe { enqueueAudio(packet.buffer.as_ptr() as _, packet.buffer.len() as _) }
            }
        });
    }

    let mut haptics_receiver = stream_socket
        .subscribe_to_stream::<HapticsPacket>(StreamId::Haptics)
        .await?;
    let mut on_stream_stop_receiver = on_stream_stop_notifier.subscribe();
    tokio::spawn(async move {
        loop {
            let packet = tokio::select! {
                Ok(packet) = haptics_receiver.recv() => packet,
                _ = on_stream_stop_receiver.recv() => break,
                else => break,
            };

            unsafe {
                onHapticsFeedback(
                    0,
                    packet.amplitude,
                    packet.duration.as_secs_f32(),
                    packet.frequency,
                    matches!(packet.device, TrackedDeviceType::RightController) as _,
                )
            }
        }
    });

    {
        // GuardianData contains raw pointers that are not ffi safe. Force it to be treated as safe.
        // This is needed by tokio.
        unsafe impl Send for GuardianData {}

        let data = unsafe { getGuardianInfo() };

        let points = unsafe { slice::from_raw_parts(data.points, data.totalPointCount as _) };
        let points = points
            .iter()
            .map(|vec3| Point3::new(vec3.x, vec3.y, vec3.z))
            .collect::<Vec<_>>();

        let packet = PlayspaceSyncPacket {
            position: Point3::new(
                data.standingPosPosition.x,
                data.standingPosPosition.y,
                data.standingPosPosition.z,
            ),
            rotation: UnitQuaternion::from_quaternion(Quaternion::new(
                data.standingPosRotation.w,
                data.standingPosRotation.x,
                data.standingPosRotation.y,
                data.standingPosRotation.z,
            )),
            space_rectangle: (data.playAreaSize.x, data.playAreaSize.y),
            points,
        };

        control_socket
            .send(ClientControlPacket::PlayspaceSync(packet))
            .await?;
    }

    {
        let mut foveation_enabled = false;
        let mut foveation_strength = 0_f32;
        let mut foveation_shape = 0_f32;
        let mut foveation_vertical_offset = 0_f32;
        if let Switch::Enabled(foveation_vars) = settings.video.foveated_rendering {
            foveation_enabled = true;
            foveation_strength = foveation_vars.strength;
            foveation_shape = foveation_vars.shape;
            foveation_vertical_offset = foveation_vars.vertical_offset;
        }

        // Store the parameters in a temporary variable so we don't need to pass them to java
        *ON_STREAM_START_PARAMS_TEMP.lock() = Some(OnStreamStartParams {
            eyeWidth: client_config.eye_resolution_width as _,
            eyeHeight: client_config.eye_resolution_height as _,
            leftEyeFov: EyeFov {
                left: client_config.left_eye_fov.left,
                right: client_config.left_eye_fov.right,
                top: client_config.left_eye_fov.top,
                bottom: client_config.left_eye_fov.bottom,
            },
            foveationEnabled: foveation_enabled,
            foveationStrength: foveation_strength,
            foveationShape: foveation_shape,
            foveationVerticalOffset: foveation_vertical_offset,
            enableGameAudio: matches!(settings.audio.game_audio, Switch::Enabled(_)),
            enableMicrophone: settings.audio.microphone,
            refreshRate: client_config.fps as _,
        });
        trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
            &*activity_ref,
            "onStreamStart",
            "()V",
            &[],
        ))?;
    }

    let last_statistics_send_time = Instant::now() - STATISTICS_SEND_INTERVAL;
    loop {
        let input_loop_deadline = time::Instant::now() + INPUT_SEND_INTERVAL;

        {
            unsafe impl Send for TrackingInfo {}

            let info = unsafe { getTrackingInfo() };

            STATISTICS.lock().report_tracking_frame(info.FrameIndex);

            let timestamp = Duration::from_secs_f64(info.predictedDisplayTime);

            let mut device_motions = HashMap::new();
            device_motions.insert(
                TrackedDeviceType::LeftController,
                MotionDesc {
                    timestamp,
                    pose: Pose {
                        position: Point3::new(
                            info.controller[0].position.x,
                            info.controller[0].position.y,
                            info.controller[0].position.z,
                        ),
                        orientation: UnitQuaternion::from_quaternion(Quaternion::new(
                            info.controller[0].orientation.w,
                            info.controller[0].orientation.x,
                            info.controller[0].orientation.y,
                            info.controller[0].orientation.z,
                        )),
                    },
                    linear_velocity: Vector3::new(
                        info.controller[0].linearVelocity.x,
                        info.controller[0].linearVelocity.y,
                        info.controller[0].linearVelocity.z,
                    ),
                    angular_velocity: Vector3::new(
                        info.controller[0].angularVelocity.x,
                        info.controller[0].angularVelocity.y,
                        info.controller[0].angularVelocity.z,
                    ),
                },
            );
            device_motions.insert(
                TrackedDeviceType::RightController,
                MotionDesc {
                    timestamp,
                    pose: Pose {
                        position: Point3::new(
                            info.controller[1].position.x,
                            info.controller[1].position.y,
                            info.controller[1].position.z,
                        ),
                        orientation: UnitQuaternion::from_quaternion(Quaternion::new(
                            info.controller[1].orientation.w,
                            info.controller[1].orientation.x,
                            info.controller[1].orientation.y,
                            info.controller[1].orientation.z,
                        )),
                    },
                    linear_velocity: Vector3::new(
                        info.controller[1].linearVelocity.x,
                        info.controller[1].linearVelocity.y,
                        info.controller[1].linearVelocity.z,
                    ),
                    angular_velocity: Vector3::new(
                        info.controller[1].angularVelocity.x,
                        info.controller[1].angularVelocity.y,
                        info.controller[1].angularVelocity.z,
                    ),
                },
            );

            let input_data = InputDeviceData::OculusTouchPair([
                OculusTouchInput {
                    thumbstick_coord: (
                        info.controller[0].trackpadPosition.x,
                        info.controller[0].trackpadPosition.y,
                    ),
                    trigger: info.controller[0].triggerValue,
                    grip: info.controller[0].gripValue,
                    battery_percentage: info.controller[0].batteryPercentRemaining,
                    digital_input: OculusTouchDigitalInput::empty(),
                },
                OculusTouchInput {
                    thumbstick_coord: (
                        info.controller[1].trackpadPosition.x,
                        info.controller[1].trackpadPosition.y,
                    ),
                    trigger: info.controller[1].triggerValue,
                    grip: info.controller[1].gripValue,
                    battery_percentage: info.controller[1].batteryPercentRemaining,
                    digital_input: OculusTouchDigitalInput::empty(),
                },
            ]);

            let bone_rotations_vec = info.controller[0]
                .boneRotations
                .iter()
                .map(|q| UnitQuaternion::from_quaternion(Quaternion::new(q.w, q.x, q.y, q.z)))
                .collect::<Vec<_>>();
            let mut bone_rotations_left = [UnitQuaternion::default(); 19];
            bone_rotations_left.copy_from_slice(&bone_rotations_vec);

            let bone_rotations_vec = info.controller[1]
                .boneRotations
                .iter()
                .map(|q| UnitQuaternion::from_quaternion(Quaternion::new(q.w, q.x, q.y, q.z)))
                .collect::<Vec<_>>();
            let mut bone_rotations_right = [UnitQuaternion::default(); 19];
            bone_rotations_right.copy_from_slice(&bone_rotations_vec);

            let bone_positions_vec = info.controller[0]
                .boneRotations
                .iter()
                .map(|v| Point3::new(v.x, v.y, v.z))
                .collect::<Vec<_>>();
            let mut bone_positions_left = [Point3::new(0_f32, 0_f32, 0_f32); 19];
            bone_positions_left.copy_from_slice(&bone_positions_vec);

            let bone_positions_vec = info.controller[1]
                .boneRotations
                .iter()
                .map(|v| Point3::new(v.x, v.y, v.z))
                .collect::<Vec<_>>();
            let mut bone_positions_right = [Point3::new(0_f32, 0_f32, 0_f32); 19];
            bone_positions_right.copy_from_slice(&bone_positions_vec);

            let input_packet = InputPacket {
                client_time: info.clientTime,
                frame_index: info.FrameIndex,
                head_motion: MotionDesc {
                    timestamp,
                    pose: Pose {
                        position: Point3::new(
                            info.HeadPose_Pose_Position.x,
                            info.HeadPose_Pose_Position.y,
                            info.HeadPose_Pose_Position.z,
                        ),
                        orientation: UnitQuaternion::from_quaternion(Quaternion::new(
                            info.HeadPose_Pose_Orientation.w,
                            info.HeadPose_Pose_Orientation.x,
                            info.HeadPose_Pose_Orientation.y,
                            info.HeadPose_Pose_Orientation.z,
                        )),
                    },
                    linear_velocity: Vector3::new(0_f32, 0_f32, 0_f32),
                    angular_velocity: Vector3::new(0_f32, 0_f32, 0_f32),
                },
                device_motions,
                input_data,
                input_data_timestamp: timestamp,
                buttons: [info.controller[0].buttons, info.controller[1].buttons],
                bone_rotations: [bone_rotations_left, bone_rotations_right],
                bone_positions_base: [bone_positions_left, bone_positions_right],
                bone_root_oritentation: [
                    UnitQuaternion::from_quaternion(Quaternion::new(
                        info.controller[0].boneRootOrientation.w,
                        info.controller[0].boneRootOrientation.x,
                        info.controller[0].boneRootOrientation.y,
                        info.controller[0].boneRootOrientation.z,
                    )),
                    UnitQuaternion::from_quaternion(Quaternion::new(
                        info.controller[1].boneRootOrientation.w,
                        info.controller[1].boneRootOrientation.x,
                        info.controller[1].boneRootOrientation.y,
                        info.controller[1].boneRootOrientation.z,
                    )),
                ],
                bone_root_position: [
                    Point3::new(
                        info.controller[0].boneRootPosition.x,
                        info.controller[0].boneRootPosition.y,
                        info.controller[0].boneRootPosition.z,
                    ),
                    Point3::new(
                        info.controller[1].boneRootPosition.x,
                        info.controller[1].boneRootPosition.y,
                        info.controller[1].boneRootPosition.z,
                    ),
                ],
                input_state_status: [
                    info.controller[0].inputStateStatus,
                    info.controller[1].inputStateStatus,
                ],
                finger_pinch_strength: [
                    info.controller[0].fingerPinchStrengths,
                    info.controller[1].fingerPinchStrengths,
                ],
                hand_finger_confidences: [
                    info.controller[0].handFingerConfidences,
                    info.controller[1].handFingerConfidences,
                ],
            };

            if show_err(input_sender.send(&input_packet).await).is_err() {
                stopStream(&*java_vm, &*activity_ref, &mut control_socket, false).await?;
            }
        }

        if let Some(sender) = &mut maybe_microphone_sender {
            unsafe impl Send for MicAudioFrame {}

            let frame = unsafe { getMicData() };

            let buffer = unsafe {
                slice::from_raw_parts(frame.buffer as *const u8, 2 * frame.size as usize)
            }
            .to_vec();

            let res = sender
                .send(&AudioPacket {
                    packet_index: 0,
                    presentation_time: Duration::from_secs(0),
                    buffer,
                })
                .await;
            if show_err(res).is_err() {
                stopStream(&*java_vm, &*activity_ref, &mut control_socket, false).await?;
            }
        }

        if Instant::now() - last_statistics_send_time > STATISTICS_SEND_INTERVAL {
            let stats = STATISTICS.lock().get_and_reset();

            if let Err(e) = control_socket
                .send(ClientControlPacket::Statistics(stats))
                .await
            {
                stopStream(&*java_vm, &*activity_ref, &mut control_socket, false).await?;

                return trace_str!("{}", e);
            }
        }

        tokio::select! {
            maybe_packet = control_socket.recv() => {
                match trace_err!(maybe_packet)? {
                    ServerControlPacket::Restarting => {
                        stopStream(&*java_vm, &*activity_ref, &mut control_socket, true).await?;
                    }
                    ServerControlPacket::Shutdown => {
                        stopStream(&*java_vm, &*activity_ref, &mut control_socket, false).await?;
                    }
                    ServerControlPacket::Reserved(_) => ()
                }
            }
            _ = time::delay_until(input_loop_deadline) => ()
        }
    }
}

pub async fn connection_loop(
    headset_info: HeadsetInfoPacket,
    private_identity: PrivateIdentity,
    on_stream_stop_notifier: broadcast::Sender<()>,
    java_vm: Arc<JavaVM>,
    activity_ref: Arc<GlobalRef>,
) {
    let mut on_stream_stop_receiver = on_stream_stop_notifier.subscribe();

    // this loop has no exit, but the execution can be halted by the caller with tokio::select!{}
    loop {
        let try_connect_future = show_err_async(try_connect(
            &headset_info,
            &private_identity,
            on_stream_stop_notifier.clone(),
            java_vm.clone(),
            activity_ref.clone(),
        ));

        tokio::select! {
            _ = try_connect_future => (),
            _ = on_stream_stop_receiver.recv() => (),
        }
    }
}
