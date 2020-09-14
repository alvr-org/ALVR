use alvr_common::{data::*, logging::show_err, sockets::*, *};
use tokio::sync::broadcast;

pub async fn connection_loop(
    headset_info: HeadsetInfoPacket,
    private_identity: PrivateIdentity,
    on_pause_notifier: broadcast::Sender<()>,
) {
    loop {
        show_err(|| -> StrResult {
            let control_socket = ControlSocket::connect_to_server(
                &headset_info,
                private_identity.hostname.clone(),
                private_identity.certificate_pem.clone(),
            );

            Ok(())
        }())
        .ok();
    }
}
