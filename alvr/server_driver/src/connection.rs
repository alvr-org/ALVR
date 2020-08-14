use crate::*;
use alvr_common::{data::*, logging::*, sockets::*, *};
use std::{collections::HashMap, net::IpAddr, sync::Arc};
use tokio::sync::broadcast;

fn align32(value: f32) -> u32 {
    ((value / 32.).floor() * 32.) as u32
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
    let stream_manager = StreamManager::connect_to_client(
        control_socket.peer_ip(),
        settings.connection.stream_port,
        client_identity,
        settings.connection.stream_socket_config,
    )
    .await?;

    // todo: create input/output streams, bind to C++ callbacks

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
                let res = search_client_loop(
                    |client_ip, client_identity| {
                        update_client_list(
                            session_manager.clone(),
                            client_identity.hostname,
                            ClientListAction::AddIfMissing {
                                ip: client_ip,
                                certificate_pem: client_identity.certificate_pem,
                            },
                            update_client_listeners_notifier.clone(),
                        )
                    }
                )
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

        control_socket.recv().await.ok();
    }
}
