use crate::*;
use alvr_common::{data::*, logging::*, sockets::*, *};
use settings_schema::Switch;
use std::time::{Duration, Instant};
use tokio::time;

const STATISTICS_SEND_INTERVAL: Duration = Duration::from_secs(1);
const INPUT_SEND_INTERVAL: Duration = Duration::from_millis(8);

async fn try_connect(
    headset_info: &HeadsetInfoPacket,
    private_identity: &PrivateIdentity,
    on_stream_stop_notifier: broadcast::Sender<()>,
    java_vm: &JavaVM,
    activity_ref: &GlobalRef,
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
            activity_ref,
            "onServerFound",
            "(ZLjava/lang/String;I)V",
            &[
                false.into(),
                trace_err!(trace_err!(java_vm.attach_current_thread())?.new_string(""))?.into(),
                0_i32.into(),
            ],
        ))?;

        return trace_str!("Found unupported server");
    };

    // todo: go through session representation. this requires settings -> session representation
    // conversion code
    let settings = trace_err!(serde_json::from_value::<Settings>(client_config.settings))?;

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        activity_ref,
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

    let input_sender = stream_socket
        .request_stream::<InputPacket>(StreamId::Input, settings.headset.tracking_stream_mode)
        .await?;

    let maybe_microphone_sender = if settings.audio.microphone {
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

    let mut video_receiver = stream_socket
        .subscribe_to_stream::<VideoPacket>(StreamId::Video())
        .await?;
    let mut on_stream_stop_receiver = on_stream_stop_notifier.subscribe();
    tokio::spawn(async move {
        loop {
            let packet = tokio::select! {
                Ok(packet) = video_receiver.recv() => packet,
                _ = on_stream_stop_receiver.recv() => break,
                else => break,
            };

            // todo
        }
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

                // todo
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

            // todo
        }
    });

    // todo: send guardian here

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
        eyeWidth: client_config.eye_resolution.0 as _,
        eyeHeight: client_config.eye_resolution.1 as _,
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
        enableMicrophone: settings.audio.microphone,
        refreshRate: client_config.fps as _,
    });
    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        activity_ref,
        "onStreamStart",
        "()V",
        &[],
    ))?;

    let last_statistics_send_time = Instant::now() - STATISTICS_SEND_INTERVAL;
    loop {
        let input_loop_deadline = time::Instant::now() + INPUT_SEND_INTERVAL;

        // todo: send input

        // todo: maybe send microphone

        if Instant::now() - last_statistics_send_time > STATISTICS_SEND_INTERVAL {
            let stats = STATISTICS.lock().get();

            if let Err(e) = control_socket
                .send(ClientControlPacket::Statistics(stats))
                .await
            {
                trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
                    activity_ref,
                    "onStreamStop",
                    "(Z)V",
                    &[false.into()],
                ))?;

                return trace_str!("{}", e);
            }
        }

        tokio::select! {
            maybe_packet = control_socket.recv() => {
                match trace_err!(maybe_packet)? {
                    ServerControlPacket::Restarting => {
                        control_socket.send(ClientControlPacket::Disconnect).await.ok();

                        trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
                            activity_ref,
                            "onStreamStop",
                            "(Z)V",
                            &[true.into()],
                        ))?;
                    }
                    ServerControlPacket::Shutdown => {
                        control_socket.send(ClientControlPacket::Disconnect).await.ok();

                        trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
                            activity_ref,
                            "onStreamStop",
                            "(Z)V",
                            &[false.into()],
                        ))?;
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
    java_vm: JavaVM,
    activity_ref: GlobalRef,
) {
    let mut on_stream_stop_receiver = on_stream_stop_notifier.subscribe();

    // this loop has no exit, but the execution can be halted by the caller with tokio::select!{}
    loop {
        let try_connect_future = show_err_async(try_connect(
            &headset_info,
            &private_identity,
            on_stream_stop_notifier.clone(),
            &java_vm,
            &activity_ref,
        ));

        tokio::select! {
            _ = try_connect_future => (),
            _ = on_stream_stop_receiver.recv() => (),
        }
    }
}
