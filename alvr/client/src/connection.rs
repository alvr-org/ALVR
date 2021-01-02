use alvr_common::{data::*, logging::*, sockets::ConnectionResult, *};
use jni::{
    objects::{GlobalRef, JClass},
    JavaVM,
};
use nalgebra::{Point3, Quaternion, UnitQuaternion};
use serde_json as json;
use settings_schema::Switch;
use std::{
    ffi::CString,
    slice,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    sync::Mutex,
    time::{self, Instant},
};

const INITIAL_MESSAGE: &str = "Searching for server...\n(open ALVR on your PC)";
const CLIENT_UNTRUSTED_MESSAGE: &str = "On the PC, click \"Trust\"\nnext to the client entry";
const INCOMPATIBLE_VERSIONS_MESSAGE: &str = concat!(
    "Server and client have\n",
    "incompatible types.\n",
    "Please update either the app\n",
    "on the PC or on the headset"
);
const SERVER_RESTART_MESSAGE: &str = "The server is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The server has disconnected.";
const PLAYSPACE_SYNC_INTERVAL: Duration = Duration::from_millis(500);

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard {
    is_connected: Arc<AtomicBool>,
}

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        self.is_connected.store(false, Ordering::Relaxed)
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

async fn try_connect(
    headset_info: &HeadsetInfoPacket,
    device_name: String,
    private_identity: &PrivateIdentity,
    java_vm: Arc<JavaVM>,
    activity_ref: Arc<GlobalRef>,
    nal_class_ref: Arc<GlobalRef>,
) -> StrResult {
    let hostname = &private_identity.hostname;

    let connection_result = trace_err!(
        sockets::connect_to_server(
            &headset_info,
            device_name,
            hostname.clone(),
            private_identity.certificate_pem.clone(),
        )
        .await
    )?;
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
    };
    let control_sender = Arc::new(Mutex::new(control_sender));

    info!("Connected to server");

    let baseline_settings = {
        let mut session_desc = SessionDesc::default();
        session_desc.merge_from_json(&trace_err!(json::from_str(&config_packet.session_desc))?)?;
        session_desc.to_settings()
    };

    let is_connected = Arc::new(AtomicBool::new(true));
    let _stream_guard = StreamCloseGuard {
        is_connected: is_connected.clone(),
    };

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        &*activity_ref,
        "setDarkMode",
        "(Z)V",
        &[baseline_settings.extra.client_dark_mode.into()],
    ))?;

    trace_err!(trace_err!(java_vm.attach_current_thread())?.call_method(
        &*activity_ref,
        "onServerConnected",
        "(IIIZIZIFFFILjava/lang/String;)V",
        &[
            (config_packet.eye_resolution_width as i32 * 2).into(),
            (config_packet.eye_resolution_height as i32).into(),
            (matches!(baseline_settings.video.codec, CodecType::HEVC) as i32).into(),
            baseline_settings
                .video
                .client_request_realtime_decoder
                .into(),
            (config_packet.fps as i32).into(),
            matches!(baseline_settings.audio.microphone, Switch::Enabled(_)).into(),
            (matches!(
                baseline_settings.video.foveated_rendering,
                Switch::Enabled(_)
            ) as i32)
                .into(),
            (if let Switch::Enabled(foveation_vars) = &baseline_settings.video.foveated_rendering {
                foveation_vars.strength
            } else {
                0_f32
            })
            .into(),
            (if let Switch::Enabled(foveation_vars) = &baseline_settings.video.foveated_rendering {
                foveation_vars.shape
            } else {
                1_f32
            })
            .into(),
            (if let Switch::Enabled(foveation_vars) = &baseline_settings.video.foveated_rendering {
                foveation_vars.vertical_offset
            } else {
                0_f32
            })
            .into(),
            (matches!(
                baseline_settings.headset.tracking_space,
                TrackingSpace::Stage
            ) as i32)
                .into(),
            trace_err!(
                trace_err!(java_vm.attach_current_thread())?.new_string(config_packet.web_gui_url)
            )?
            .into()
        ],
    ))?;

    let tracking_clientside_prediction = match baseline_settings.headset.controllers {
        Switch::Enabled(ref controllers) => controllers.clientside_prediction,
        Switch::Disabled => false,
    };

    // setup stream loops

    // The main stream loop must be run in a normal thread, because it needs to access the JNI env
    // many times per second. If using a future I'm forced to attach and detach the env continuously.
    // When the parent function gets canceled, this loop will run to finish.
    let server_ip_cstring = CString::new(server_ip.to_string()).unwrap();
    let stream_socket_loop = tokio::task::spawn_blocking({
        let java_vm = java_vm.clone();
        let activity_ref = activity_ref.clone();
        let nal_class_ref = nal_class_ref.clone();
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
                    server_ip_cstring.as_ptr(),
                    matches!(baseline_settings.video.codec, CodecType::HEVC) as _,
                    baseline_settings.connection.client_recv_buffer_size as _,
                );

                while is_connected.load(Ordering::Relaxed) {
                    crate::runSocketLoopIter();
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
                        Ok(ServerControlPacket::Reserved(_))
                        | Ok(ServerControlPacket::ReservedBuffer(_)) => (),
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
        res = stream_socket_loop => trace_err!(res)?,
        res = tracking_loop => res,
        res = playspace_sync_loop => res,
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
        show_err(
            try_connect(
                &headset_info,
                device_name.to_owned(),
                &private_identity,
                java_vm.clone(),
                activity_ref.clone(),
                nal_class_ref.clone(),
            )
            .await,
        )
        .ok();
    }
}
