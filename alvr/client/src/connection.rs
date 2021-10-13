use crate::{
    connection_utils::{self, ConnectionError},
    MAYBE_LEGACY_SENDER,
};
use alvr_common::{prelude::*, ALVR_NAME, ALVR_VERSION};
use alvr_session::{CodecType, SessionDesc, TrackingSpace};
use alvr_sockets::{
    spawn_cancelable, ClientConfigPacket, ClientControlPacket, ClientHandshakePacket,
    HeadsetInfoPacket, PeerType, PlayspaceSyncPacket, PrivateIdentity, ProtoControlSocket,
    ServerControlPacket, ServerHandshakePacket, StreamSocketBuilder, LEGACY,
};
use futures::future::BoxFuture;
use jni::{
    objects::{GlobalRef, JClass},
    JavaVM,
};
use nalgebra::{Point2, Point3, Quaternion, UnitQuaternion};
use serde_json as json;
use settings_schema::Switch;
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

#[cfg(target_os = "android")]
use crate::audio;

const INITIAL_MESSAGE: &str = "Searching for server...\n(open ALVR on your PC)";
const NETWORK_UNREACHABLE_MESSAGE: &str = "Cannot connect to the internet";
const CLIENT_UNTRUSTED_MESSAGE: &str = "On the PC, click \"Trust\"\nnext to the client entry";
const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
    "Server and client have\n",
    "incompatible types.\n",
    "Please update either the app\n",
    "on the PC or on the headset"
);
const STREAM_STARTING_MESSAGE: &str = "The stream will begin soon\nPlease wait...";
const SERVER_RESTART_MESSAGE: &str = "The server is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The server has disconnected.";

const CONTROL_CONNECT_RETRY_PAUSE: Duration = Duration::from_millis(500);
const RETRY_CONNECT_MIN_INTERVAL: Duration = Duration::from_secs(1);
const PLAYSPACE_SYNC_INTERVAL: Duration = Duration::from_millis(500);
const NETWORK_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
const CLEANUP_PAUSE: Duration = Duration::from_millis(500);

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard {
    is_connected: Arc<AtomicBool>,
}

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        self.is_connected.store(false, Ordering::Relaxed);
    }
}

fn set_loading_message(
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

    let handshake_packet = ClientHandshakePacket {
        alvr_name: ALVR_NAME.into(),
        version: ALVR_VERSION.clone(),
        device_name,
        hostname: hostname.clone(),
        reserved1: "".into(),
        reserved2: "".into(),
    };

    let (mut proto_socket, server_ip) = tokio::select! {
        res = connection_utils::announce_client_loop(handshake_packet) => {
            match res? {
                ConnectionError::ServerMessage(message) => {
                    info!("Server response: {:?}", message);
                    let message_str = match message {
                        ServerHandshakePacket::ClientUntrusted => CLIENT_UNTRUSTED_MESSAGE,
                        ServerHandshakePacket::IncompatibleVersions =>
                            INCOMPATIBLE_VERSIONS_MESSAGE,
                    };
                    set_loading_message(&*java_vm, &*activity_ref, hostname, message_str)?;
                    return Ok(());
                }
                ConnectionError::NetworkUnreachable => {
                    info!("Network unreachable");
                    set_loading_message(
                        &*java_vm,
                        &*activity_ref,
                        hostname,
                        NETWORK_UNREACHABLE_MESSAGE,
                    )?;

                    time::sleep(RETRY_CONNECT_MIN_INTERVAL).await;

                    set_loading_message(
                        &*java_vm,
                        &*activity_ref,
                        &private_identity.hostname,
                        INITIAL_MESSAGE,
                    )
                    .ok();

                    return Ok(());
                }
            }
        },
        pair = async {
            loop {
                if let Ok(pair) = ProtoControlSocket::connect_to(PeerType::Server).await {
                    break pair;
                }

                time::sleep(CONTROL_CONNECT_RETRY_PAUSE).await;
            }
        } => pair
    };

    trace_err!(proto_socket.send(&(headset_info, server_ip)).await)?;
    let config_packet = trace_err!(proto_socket.recv::<ClientConfigPacket>().await)?;

    let (control_sender, mut control_receiver) = proto_socket.split();
    let control_sender = Arc::new(Mutex::new(control_sender));

    match control_receiver.recv().await {
        Ok(ServerControlPacket::StartStream) => {
            info!("Stream starting");
            set_loading_message(&*java_vm, &*activity_ref, hostname, STREAM_STARTING_MESSAGE)?;
        }
        Ok(ServerControlPacket::Restarting) => {
            info!("Server restarting");
            set_loading_message(&*java_vm, &*activity_ref, hostname, SERVER_RESTART_MESSAGE)?;
            return Ok(());
        }
        Err(e) => {
            info!("Server disconnected. Cause: {}", e);
            set_loading_message(
                &*java_vm,
                &*activity_ref,
                hostname,
                SERVER_DISCONNECTED_MESSAGE,
            )?;
            return Ok(());
        }
        _ => {
            info!("Unexpected packet");
            set_loading_message(&*java_vm, &*activity_ref, hostname, "Unexpected packet")?;
            return Ok(());
        }
    }

    let settings = {
        let mut session_desc = SessionDesc::default();
        session_desc.merge_from_json(&trace_err!(json::from_str(&config_packet.session_desc))?)?;
        session_desc.to_settings()
    };

    let stream_socket_builder = StreamSocketBuilder::listen_for_server(
        settings.connection.stream_port,
        settings.connection.stream_protocol,
    )
    .await?;

    if let Err(e) = control_sender
        .lock()
        .await
        .send(&ClientControlPacket::StreamReady)
        .await
    {
        info!("Server disconnected. Cause: {}", e);
        set_loading_message(
            &*java_vm,
            &*activity_ref,
            hostname,
            SERVER_DISCONNECTED_MESSAGE,
        )?;
        return Ok(());
    }

    let mut stream_socket = tokio::select! {
        res = stream_socket_builder.accept_from_server(
            server_ip,
            settings.connection.stream_port,
        ) => res?,
        _ = time::sleep(Duration::from_secs(5)) => {
            return fmt_e!("Timeout while setting up streams");
        }
    };

    info!("Connected to server");

    let is_connected = Arc::new(AtomicBool::new(true));
    let _stream_guard = StreamCloseGuard {
        is_connected: Arc::clone(&is_connected),
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
            foveationCenterSizeX: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_size_x
            } else {
                3_f32 / 5_f32
            },
            foveationCenterSizeY: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_size_y
            } else {
                2_f32 / 5_f32
            },
            foveationCenterShiftX: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_shift_x
            } else {
                2_f32 / 5_f32
            },
            foveationCenterShiftY: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.center_shift_y
            } else {
                1_f32 / 10_f32
            },
            foveationEdgeRatioX: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.edge_ratio_x
            } else {
                2_f32
            },
            foveationEdgeRatioY: if let Switch::Enabled(foveation_vars) =
                &settings.video.foveated_rendering
            {
                foveation_vars.edge_ratio_y
            } else {
                2_f32
            },
            trackingSpaceType: matches!(settings.headset.tracking_space, TrackingSpace::Stage) as _,
            extraLatencyMode: settings.headset.extra_latency_mode,
        });
    }

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        &*activity_ref,
        "onServerConnected",
        "(FIZLjava/lang/String;)V",
        &[
            config_packet.fps.into(),
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

    // let (debug_sender, mut debug_receiver) = tmpsc::unbounded_channel();
    // let debug_loop = {
    //     let control_sender = Arc::clone(&control_sender);
    //     async move {
    //         while let Some(data) = debug_receiver.recv().await {
    //             control_sender
    //                 .lock()
    //                 .await
    //                 .send(&ClientControlPacket::Reserved(data))
    //                 .await
    //                 .ok();
    //         }

    //         Ok(())
    //     }
    // };

    let legacy_send_loop = {
        let mut socket_sender = stream_socket.request_stream::<_, LEGACY>().await?;
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

    let (legacy_receive_data_sender, legacy_receive_data_receiver) = smpsc::channel();
    let legacy_receive_loop = {
        let mut receiver = stream_socket.subscribe_to_stream::<(), LEGACY>().await?;
        async move {
            loop {
                let packet = receiver.recv().await?;
                legacy_receive_data_sender.send(packet.buffer).ok();
            }
        }
    };

    // The main stream loop must be run in a normal thread, because it needs to access the JNI env
    // many times per second. If using a future I'm forced to attach and detach the env continuously.
    // When the parent function exits or gets canceled, this loop will run to finish.
    let legacy_stream_socket_loop = task::spawn_blocking({
        let java_vm = Arc::clone(&java_vm);
        let activity_ref = Arc::clone(&activity_ref);
        let nal_class_ref = Arc::clone(&nal_class_ref);
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

                let mut idr_request_deadline = None;

                while let Ok(mut data) = legacy_receive_data_receiver.recv() {
                    // Send again IDR packet every 2s in case it is missed
                    // (due to dropped burst of packets at the start of the stream or otherwise).
                    if !crate::IDR_PARSED.load(Ordering::Relaxed) {
                        if let Some(deadline) = idr_request_deadline {
                            if deadline < Instant::now() {
                                crate::IDR_REQUEST_NOTIFIER.notify_waiters();
                                idr_request_deadline = None;
                            }
                        } else {
                            idr_request_deadline = Some(Instant::now() + Duration::from_secs(2));
                        }
                    }

                    crate::legacyReceive(data.as_mut_ptr(), data.len() as _);
                }

                crate::closeSocket(env_ptr);
            }

            Ok(())
        }
    });

    let tracking_interval = Duration::from_secs_f32(1_f32 / 360_f32);
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
        let control_sender = Arc::clone(&control_sender);
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
                            .map(|p| Point2::from_slice(&[p[0], p[2]]))
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
                        .await
                        .ok();
                }

                time::sleep(PLAYSPACE_SYNC_INTERVAL).await;
            }
        }
    };

    let game_audio_loop: BoxFuture<_> = if let Switch::Enabled(desc) = settings.audio.game_audio {
        #[cfg(target_os = "android")]
        {
            let game_audio_receiver = stream_socket.subscribe_to_stream().await?;
            Box::pin(audio::play_audio_loop(
                config_packet.game_audio_sample_rate,
                desc.config,
                game_audio_receiver,
            ))
        }
        #[cfg(not(target_os = "android"))]
        Box::pin(future::pending())
    } else {
        Box::pin(future::pending())
    };

    let microphone_loop: BoxFuture<_> = if let Switch::Enabled(config) = settings.audio.microphone {
        #[cfg(target_os = "android")]
        {
            let microphone_sender = stream_socket.request_stream().await?;
            Box::pin(audio::record_audio_loop(
                config.sample_rate,
                microphone_sender,
            ))
        }
        #[cfg(not(target_os = "android"))]
        Box::pin(future::pending())
    } else {
        Box::pin(future::pending())
    };

    let keepalive_sender_loop = {
        let control_sender = Arc::clone(&control_sender);
        let java_vm = Arc::clone(&java_vm);
        let activity_ref = Arc::clone(&activity_ref);
        async move {
            loop {
                let res = control_sender
                    .lock()
                    .await
                    .send(&ClientControlPacket::KeepAlive)
                    .await;
                if let Err(e) = res {
                    info!("Server disconnected. Cause: {}", e);
                    set_loading_message(
                        &*java_vm,
                        &*activity_ref,
                        hostname,
                        SERVER_DISCONNECTED_MESSAGE,
                    )?;
                    break Ok(());
                }

                time::sleep(NETWORK_KEEPALIVE_INTERVAL).await;
            }
        }
    };

    let control_loop = {
        let java_vm = Arc::clone(&java_vm);
        let activity_ref = Arc::clone(&activity_ref);
        async move {
            loop {
                tokio::select! {
                    _ = crate::IDR_REQUEST_NOTIFIER.notified() => {
                        control_sender.lock().await.send(&ClientControlPacket::RequestIdr).await?;
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
                                )?;
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
                                )?;
                                break Ok(());
                            }
                        }
                }
            }
        }
    };

    // Run many tasks concurrently. Threading is managed by the runtime, for best performance.
    tokio::select! {
        res = spawn_cancelable(stream_socket.receive_loop()) => {
            if let Err(e) = res {
                info!("Server disconnected. Cause: {}", e);
            }
            set_loading_message(
                &*java_vm,
                &*activity_ref,
                hostname,
                SERVER_DISCONNECTED_MESSAGE
            )?;

            Ok(())
        },
        res = spawn_cancelable(game_audio_loop) => res,
        res = spawn_cancelable(microphone_loop) => res,
        res = spawn_cancelable(tracking_loop) => res,
        res = spawn_cancelable(playspace_sync_loop) => res,
        res = spawn_cancelable(legacy_send_loop) => res,
        res = spawn_cancelable(legacy_receive_loop) => res,
        res = legacy_stream_socket_loop => trace_err!(res)?,

        // keep these loops on the current task
        res = keepalive_sender_loop => res,
        res = control_loop => res,
        // res = debug_loop => res,
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
    .ok();

    loop {
        tokio::join!(
            async {
                let maybe_error = connection_pipeline(
                    &headset_info,
                    device_name.to_owned(),
                    &private_identity,
                    Arc::clone(&java_vm),
                    Arc::clone(&activity_ref),
                    Arc::clone(&nal_class_ref),
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
                    .ok();
                }

                // let any running task or socket shutdown
                time::sleep(CLEANUP_PAUSE).await;
            },
            time::sleep(RETRY_CONNECT_MIN_INTERVAL),
        );
    }
}
