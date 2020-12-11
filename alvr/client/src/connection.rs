use alvr_common::{data::*, logging::*, sockets::ControlSocket, *};
use jni::{objects::GlobalRef, JavaVM};
use serde_json as json;
use std::sync::Arc;
use tokio::sync::broadcast;

async fn try_connect(
    headset_info: &HeadsetInfoPacket,
    device_name: String,
    private_identity: &PrivateIdentity,
    on_stream_stop_notifier: broadcast::Sender<()>,
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

    loop {
        match trace_err!(control_socket.recv().await)? {
            ServerControlPacket::Restarting => {
                // stopStream(&*java_vm, &*activity_ref, &mut control_socket, true).await?;
            }
            ServerControlPacket::Shutdown => {
                // stopStream(&*java_vm, &*activity_ref, &mut control_socket, false).await?;
            }
            ServerControlPacket::Reserved(_) => (),
            ServerControlPacket::ReservedBuffer(_) => (),
        }
    }
}

pub async fn connection_loop(
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
