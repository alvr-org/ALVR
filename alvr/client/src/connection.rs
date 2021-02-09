use crate::{audio, MAYBE_LEGACY_SENDER};
use alvr_common::{
    data::*,
    sockets::{ConnectionResult, AUDIO, LEGACY},
    *,
};
use futures::future::BoxFuture;
use jni::{
    objects::{GlobalRef, JClass},
    JavaVM,
};
use nalgebra::{Point3, Quaternion, UnitQuaternion};
use serde_json as json;
use settings_schema::Switch;
use sockets::StreamSocket;
use std::{
    future, slice,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as smpsc, Arc,
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc as tmpsc, Mutex},
    task,
    time::{self, Instant},
};

const INITIAL_MESSAGE: &str = "Searching for server...\n(open ALVR on your PC)";
const NETWORK_UNREACHABLE_MESSAGE: &str = "Cannot connect to the internet";
const CLIENT_UNTRUSTED_MESSAGE: &str = "On the PC, click \"Trust\"\nnext to the client entry";
const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
    "Server and client have\n",
    "incompatible types.\n",
    "Please update either the app\n",
    "on the PC or on the headset"
);
const SERVER_RESTART_MESSAGE: &str = "The server is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The server has disconnected.";
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_millis(500);
const PLAYSPACE_SYNC_INTERVAL: Duration = Duration::from_millis(500);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(5);

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard {
    is_connected: Arc<AtomicBool>,
}

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        self.is_connected.store(false, Ordering::Relaxed);
    }
}

async fn set_loading_message(
    java_vm: &JavaVM,
    activity_ref: &GlobalRef,
    hostname: &str,
    message: &str,
) -> StrResult {
    let message = format!(
        "ALVR v{}\nhostname: {}\n \n{}",
        ALVR_VERSION.to_string(),
        hostname,
        message
    );

    // Note: env = java_vm.attach_current_thread() cannot be saved into a variable because it is
    // not Send (compile error). This makes sense since tokio could move the execution of this
    // task to another thread at any time, and env is valid only within a specific thread. For
    // the same reason, other jni objects cannot be made into variables and the arguments must
    // be created inline within the call_method() call
    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        activity_ref,
        "setLoadingMessage",
        "(Ljava/lang/String;)V",
        &[trace_err!(trace_err!(java_vm.attach_current_thread())?.new_string(message))?.into()],
    ))?;

    Ok(())
}

async fn connection_pipeline(
    headset_info: &HeadsetInfoPacket,
    device_name: String,
    private_identity: &PrivateIdentity,
    java_vm: Arc<JavaVM>,
    activity_ref: Arc<GlobalRef>,
    nal_class_ref: Arc<GlobalRef>,
) -> StrResult {
    let hostname = &private_identity.hostname;

    let connection_result = sockets::connect_to_server(
        &headset_info,
        device_name,
        hostname.clone(),
        private_identity.certificate_pem.clone(),
    )
    .await?;
    let (server_ip, control_sender, mut control_receiver, config_packet) = match connection_result {
        ConnectionResult::Connected {
            server_ip,
            control_sender,
            control_receiver,
            config_packet,
        } => (server_ip, control_sender, control_receiver, config_packet),
        ConnectionResult::ServerMessage(message) => {
            info!("Server response: {:?}", message);
            let message_str = match message {
                ServerHandshakePacket::ClientUntrusted => CLIENT_UNTRUSTED_MESSAGE,
                ServerHandshakePacket::IncompatibleVersions => INCOMPATIBLE_VERSIONS_MESSAGE,
            };
            set_loading_message(&*java_vm, &*activity_ref, hostname, message_str).await?;
            return Ok(());
        }
        ConnectionResult::NetworkUnreachable => {
            info!("Network unreachable");
            set_loading_message(
                &*java_vm,
                &*activity_ref,
                hostname,
                NETWORK_UNREACHABLE_MESSAGE,
            )
            .await?;

            time::sleep(RETRY_CONNECT_MIN_INTERVAL).await;

            set_loading_message(
                &*java_vm,
                &*activity_ref,
                &private_identity.hostname,
                INITIAL_MESSAGE,
            )
            .await
            .ok();

            return Ok(());
        }
    };
    let control_sender = Arc::new(Mutex::new(control_sender));

    match control_receiver.recv().await {
        Ok(ServerControlPacket::StartStream) => (),
        Ok(ServerControlPacket::Restarting) => {
            info!("Server restarting");
            set_loading_message(&*java_vm, &*activity_ref, hostname, SERVER_RESTART_MESSAGE)
                .await?;
            return Ok(());
        }
        Err(e) => {
            info!("Server disconnected. Cause: {}", e);
            set_loading_message(
                &*java_vm,
                &*activity_ref,
                hostname,
                SERVER_DISCONNECTED_MESSAGE,
            )
            .await?;
            return Ok(());
        }
        _ => {
            info!("Unexpected packet");
            set_loading_message(&*java_vm, &*activity_ref, hostname, "Unexpected packet").await?;
            return Ok(());
        }
    }

    let settings = {
        let mut session_desc = SessionDesc::default();
        session_desc.merge_from_json(&trace_err!(json::from_str(&config_packet.session_desc))?)?;
        session_desc.to_settings()
    };

    let mut stream_socket = tokio::select! {
        res = StreamSocket::connect_to_server(
            server_ip,
            settings.connection.stream_port,
            settings.connection.stream_config,
        ) => res?,
        _ = time::sleep(Duration::from_secs(2)) => {
            return fmt_e!("Timeout while setting up streams");
        }
    };

    info!("Connected to server");

    let is_connected = Arc::new(AtomicBool::new(true));
    let _stream_guard = StreamCloseGuard {
        is_connected: is_connected.clone(),
    };

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        &*activity_ref,
        "setDarkMode",
        "(Z)V",
        &[settings.extra.client_dark_mode.into()],
    ))?;

    unsafe {
        crate::setStreamConfig(crate::StreamConfig {
            eyeWidth: config_packet.eye_resolution_width,
            eyeHeight: config_packet.eye_resolution_height,
            refreshRate: config_packet.fps,
            enableFoveation: matches!(settings.video.foveated_rendering, Switch::Enabled(_)),
            foveationStrength: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.strength
            } else {
                0_f32
            },
            foveationShape: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.shape
            } else {
                1_f32
            },
            foveationVerticalOffset: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.vertical_offset
            } else {
                0_f32
            },
            trackingSpaceType: matches!(settings.headset.tracking_space, TrackingSpace::Stage) as _,
            extraLatencyMode: settings.headset.extra_latency_mode,
        });
    }

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        &*activity_ref,
        "onServerConnected",
        "(IZLjava/lang/String;)V",
        &[
            (matches!(settings.video.codec, CodecType::HEVC) as i32).into(),
            settings.video.client_request_realtime_decoder.into(),
            trace_err!(trace_err!(java_vm.attach_current_thread())?
                .new_string(config_packet.dashboard_url))?
            .into()
        ],
    ))?;

    let tracking_clientside_prediction = match &settings.headset.controllers {
        Switch::Enabled(controllers) => controllers.clientside_prediction,
        Switch::Disabled => false,
    };

    // setup stream loops

    let legacy_send_loop = {
        let socket_sender = stream_socket.request_stream(LEGACY).await?;
        async move {
            let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
            *MAYBE_LEGACY_SENDER.lock() = Some(data_sender);

            while let Some(data) = data_receiver.recv().await {
                let mut buffer = socket_sender.new_buffer(&(), data.len())?;
                buffer.get_mut().extend(data);
                socket_sender.send_buffer(buffer).await?;
            }

            Ok(())
        }
    };

    let (legacy_receive_data_sender, legacy_receive_data_receiver) = smpsc::channel();
    let legacy_receive_loop = {
        let mut receiver = stream_socket.subscribe_to_stream(LEGACY).await?;
        async move {
            loop {
                let ((), data) = receiver.recv_buffer().await?;
                legacy_receive_data_sender.send(data).ok();
            }
        }
    };

    // The main stream loop must be run in a normal thread, because it needs to access the JNI env
    // many times per second. If using a future I'm forced to attach and detach the env continuously.
    // When the parent function exits or gets canceled, this loop will run to finish.
    let legacy_stream_socket_loop = task::spawn_blocking({
        let java_vm = java_vm.clone();
        let activity_ref = activity_ref.clone();
        let nal_class_ref = nal_class_ref.clone();
        let codec = settings.video.codec;
        let enable_fec = settings.connection.enable_fec;
        move || -> StrResult {
            let env = trace_err!(java_vm.attach_current_thread())?;
            let env_ptr = env.get_native_interface() as _;
            let activity_obj = activity_ref.as_obj();
            let nal_class: JClass = nal_class_ref.as_obj().into();

            unsafe {
                crate::initializeSocket(
                    env_ptr,
                    *activity_obj as _,
                    **nal_class as _,
                    matches!(codec, CodecType::HEVC) as _,
                    enable_fec,
                );

                while let Ok(mut data) = legacy_receive_data_receiver.recv() {
                    crate::legacyReceive(data.as_mut_ptr(), data.len() as _)
                }

                crate::closeSocket(env_ptr);
            }

            Ok(())
        }
    });

    let tracking_interval = Duration::from_secs_f32(1_f32 / (config_packet.fps * 3_f32));
    let tracking_loop = async move {
        let mut deadline = Instant::now();
        loop {
            unsafe { crate::onTrackingNative(tracking_clientside_prediction) };
            deadline += tracking_interval;
            time::sleep_until(deadline).await;
        }
    };

    unsafe impl Send for crate::GuardianData {}
    let playspace_sync_loop = {
        let control_sender = control_sender.clone();
        async move {
            loop {
                let guardian_data = unsafe { crate::getGuardianData() };

                if guardian_data.shouldSync {
                    let perimeter_points = if guardian_data.perimeterPointsCount == 0 {
                        None
                    } else {
                        let perimeter_slice = unsafe {
                            slice::from_raw_parts(
                                guardian_data.perimeterPoints,
                                guardian_data.perimeterPointsCount as _,
                            )
                        };

                        let perimeter_points = perimeter_slice
                            .iter()
                            .map(|p| Point3::from_slice(p))
                            .collect::<Vec<_>>();

                        Some(perimeter_points)
                    };
                    let packet = PlayspaceSyncPacket {
                        position: Point3::from_slice(&guardian_data.position),
                        rotation: UnitQuaternion::from_quaternion(Quaternion::new(
                            guardian_data.rotation[3],
                            guardian_data.rotation[0],
                            guardian_data.rotation[1],
                            guardian_data.rotation[2],
                        )),
                        area_width: guardian_data.areaWidth,
                        area_height: guardian_data.areaHeight,
                        perimeter_points,
                    };

                    control_sender
                        .lock()
                        .await
                        .send(&ClientControlPacket::PlayspaceSync(packet))
                        .await?;
                }

                time::sleep(PLAYSPACE_SYNC_INTERVAL).await;
            }
        }
    };

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.game_audio {
        let game_audio_receiver = stream_socket.subscribe_to_stream(AUDIO).await?;
        Box::pin(audio::play_audio_loop(
            config_packet.game_audio_sample_rate,
            desc.buffer_range_multiplier,
            game_audio_receiver,
        ))
    } else {
        Box::pin(future::pending())
    };

    let microphone_loop: BoxFuture<_> = if matches!(settings.audio.microphone, Switch::Enabled(_)) {
        let microphone_sender = stream_socket.request_stream(AUDIO).await?;
        Box::pin(audio::record_audio_loop(
            config_packet.microphone_sample_rate,
            microphone_sender,
        ))
    } else {
        Box::pin(future::pending())
    };

    let keepalive_sender_loop = {
        let control_sender = control_sender.clone();
        async move {
            loop {
                control_sender
                    .lock()
                    .await
                    .send(&ClientControlPacket::KeepAlive)
                    .await
                    .ok();
                time::sleep(NETWORK_KEEPALIVE_INTERVAL).await;
            }
        }
    };

    let control_loop = async move {
        loop {
            tokio::select! {
                _ = crate::IDR_REQUEST_NOTIFIER.notified() => {
                    control_sender.lock().await.send(&ClientControlPacket::RequestIDR).await?;
                }
                control_packet = control_receiver.recv() =>
                    match control_packet {
                        Ok(ServerControlPacket::Restarting) => {
                            info!("Server restarting");
                            set_loading_message(
                                &*java_vm,
                                &*activity_ref,
                                hostname,
                                SERVER_RESTART_MESSAGE
                            )
                            .await?;
                            break Ok(());
                        }
                        Ok(_) => (),
                        Err(e) => {
                            info!("Server disconnected. Cause: {}", e);
                            set_loading_message(
                                &*java_vm,
                                &*activity_ref,
                                hostname,
                                SERVER_DISCONNECTED_MESSAGE
                            )
                            .await?;
                            break Ok(());
                        }
                    }
            }
        }
    };

    tokio::select! {
        res = stream_socket.receive_loop() => res,
        res = game_audio_loop => res,
        res = microphone_loop => res,
        res = tracking_loop => res,
        res = playspace_sync_loop => res,
        res = legacy_send_loop => res,
        res = legacy_receive_loop => res,
        res = legacy_stream_socket_loop => trace_err!(res)?,
        res = keepalive_sender_loop => res,
        res = control_loop => res,
    }
}

pub async fn connection_lifecycle_loop(
    headset_info: HeadsetInfoPacket,
    device_name: &str,
    private_identity: PrivateIdentity,
    java_vm: Arc<JavaVM>,
    activity_ref: Arc<GlobalRef>,
    nal_class_ref: Arc<GlobalRef>,
) {
    set_loading_message(
        &*java_vm,
        &*activity_ref,
        &private_identity.hostname,
        INITIAL_MESSAGE,
    )
    .await
    .ok();

    // this loop has no exit, but the execution can be halted by the caller with tokio::select!{}
    loop {
        tokio::join!(
            async {
                let maybe_error = connection_pipeline(
                    &headset_info,
                    device_name.to_owned(),
                    &private_identity,
                    java_vm.clone(),
                    activity_ref.clone(),
                    nal_class_ref.clone(),
                )
                .await;

                if let Err(e) = maybe_error {
                    let message =
                        format!("Connection error:\n{}\nCheck the PC for more details", e);
                    error!("{}", message);
                    set_loading_message(
                        &*java_vm,
                        &*activity_ref,
                        &private_identity.hostname,
                        &message,
                    )
                    .await
                    .ok();
                }
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL),
        );
    }
}
