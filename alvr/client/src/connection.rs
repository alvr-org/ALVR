use alvr_common::{data::*, logging::*, *};
use jni::{
    objects::{GlobalRef, JClass},
    JavaVM,
};
use serde_json as json;
use settings_schema::Switch;
use std::{
    ffi::CString,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::{self, Instant};

const SERVER_RESTART_MESSAGE: &str = "The server is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The server has disconnected.";

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard {
    is_connected: Arc<AtomicBool>,
}

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        self.is_connected.store(false, Ordering::Relaxed)
    }
}

async fn setLoadingMessage(java_vm: &JavaVM, activity_ref: &GlobalRef, message: &str) -> StrResult {
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
    let (server_ip, mut control_sender, mut control_receiver, config_packet) = trace_err!(
        sockets::connect_to_server(
            &headset_info,
            device_name,
            private_identity.hostname.clone(),
            private_identity.certificate_pem.clone(),
        )
        .await
    )?;

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

    info!("Connected to server");

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
            unsafe { crate::onTrackingNative() };
            deadline += tracking_interval;
            time::sleep_until(deadline).await;
        }
    };

    let control_loop = async move {
        loop {
            tokio::select! {
                _ = crate::IDR_REQUEST_NOTIFIER.notified() => {
                    control_sender.send(&ClientControlPacket::RequestIDR).await.ok();
                }
                control_packet = control_receiver.recv() =>
                    match control_packet {
                        Ok(ServerControlPacket::Restarting) => {
                            info!("Server restarting");
                            setLoadingMessage(
                                &*java_vm,
                                &*activity_ref,
                                SERVER_RESTART_MESSAGE
                            )
                            .await?;
                            break Ok(());
                        }
                        Ok(ServerControlPacket::Reserved(_))
                        | Ok(ServerControlPacket::ReservedBuffer(_)) => (),
                        Err(e) => {
                            info!("Server disconnected. Cause: {}", e);
                            setLoadingMessage(
                                &*java_vm,
                                &*activity_ref,
                                SERVER_DISCONNECTED_MESSAGE
                            )
                            .await?;
                            break Ok(());
                        }
                    }
            }
        }
    };

    error!("starting loops");

    tokio::select! {
        res = stream_socket_loop => trace_err!(res)?,
        res = tracking_loop => res,
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
