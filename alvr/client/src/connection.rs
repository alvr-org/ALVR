use crate::{connectSocket, disconnectSocket, ConnectionMessage};
use alvr_common::{data::*, logging::*, sockets::ControlSocket, *};
use jni::{objects::GlobalRef, JavaVM};
use serde_json as json;
use settings_schema::Switch;
use std::{ffi::CString, sync::Arc};
use tokio::sync::broadcast;

const SERVER_RESTART_MESSAGE: &str = "The server is restarting\nPlease wait...";
const SERVER_DISCONNECTED_MESSAGE: &str = "The server has disconnected.";

// close stream on Drop (manual disconnection or execution canceling)
struct StreamCloseGuard(Arc<JavaVM>);
impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        if let Ok(env) = self.0.attach_current_thread() {
            unsafe { disconnectSocket(env.get_native_interface() as _) };
        }
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
) -> StrResult {
    let (mut control_socket, config_packet) = trace_err!(
        ControlSocket::connect_to_server(
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

    let ip_cstring = CString::new(control_socket.peer_ip().to_string()).unwrap();
    let web_gui_url_cstring = CString::new(config_packet.web_gui_url).unwrap();

    unsafe {
        connectSocket(
            trace_err!(java_vm.attach_current_thread())?.get_native_interface() as _,
            ConnectionMessage {
                ip: ip_cstring.as_ptr(),
                codec: matches!(baseline_settings.video.codec, CodecType::HEVC) as _,
                realtimeDecoder: baseline_settings.video.client_request_realtime_decoder,
                videoWidth: config_packet.eye_resolution_width * 2,
                videoHeight: config_packet.eye_resolution_height,
                bufferSize: baseline_settings.connection.client_recv_buffer_size as _,
                frameQueueSize: baseline_settings.connection.frame_queue_size as _,
                refreshRate: config_packet.fps as _,
                streamMic: matches!(baseline_settings.audio.microphone, Switch::Enabled(_)),
                foveationMode: matches!(
                    baseline_settings.video.foveated_rendering,
                    Switch::Enabled(_)
                ) as _,
                foveationStrength: if let Switch::Enabled(foveation_vars) =
                    &baseline_settings.video.foveated_rendering
                {
                    foveation_vars.strength
                } else {
                    0_f32
                },
                foveationShape: if let Switch::Enabled(foveation_vars) =
                    &baseline_settings.video.foveated_rendering
                {
                    foveation_vars.shape
                } else {
                    1_f32
                },
                foveationVerticalOffset: if let Switch::Enabled(foveation_vars) =
                    baseline_settings.video.foveated_rendering
                {
                    foveation_vars.vertical_offset
                } else {
                    0_f32
                },
                trackingSpace: matches!(
                    baseline_settings.headset.tracking_space,
                    TrackingSpace::Stage
                ) as _,
                webGuiUrl: web_gui_url_cstring.as_ptr(),
            },
        )
    };

    let _stream_guard = StreamCloseGuard(java_vm.clone());

    info!("Connected to server");

    loop {
        match control_socket.recv().await {
            Ok(ServerControlPacket::Restarting) => {
                info!("Server restarting");
                setLoadingMessage(&*java_vm, &*activity_ref, SERVER_RESTART_MESSAGE).await?;
                break Ok(());
            }
            Ok(ServerControlPacket::Reserved(_)) | Ok(ServerControlPacket::ReservedBuffer(_)) => (),
            Err(e) => {
                info!("Server disconnected. Cause: {}", e);
                setLoadingMessage(&*java_vm, &*activity_ref, SERVER_DISCONNECTED_MESSAGE).await?;
                break Ok(());
            }
        }
    }
}

pub async fn connection_lifecycle_loop(
    headset_info: HeadsetInfoPacket,
    device_name: &str,
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
            device_name.to_owned(),
            &private_identity,
            java_vm.clone(),
            activity_ref.clone(),
        ));

        tokio::select! {
            _ = try_connect_future => (),
            _ = on_stream_stop_receiver.recv() => (),
        }
    }
}
