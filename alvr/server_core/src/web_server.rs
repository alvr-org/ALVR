use crate::{
    logging_backend::LOGGING_EVENTS_SENDER, ConnectionContext, ServerCoreEvent, FILESYSTEM_LAYOUT,
    SESSION_MANAGER,
};
use alvr_common::{
    anyhow::{self, Result},
    error, info, log, ConnectionState,
};
use alvr_events::{ButtonEvent, EventType};
use alvr_packets::{ButtonEntry, ClientListAction, ServerRequest};
use bytes::Buf;
use futures::SinkExt;
use headers::HeaderMapExt;
use hyper::{
    header::{self, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL},
    service, Body, Request, Response, StatusCode,
};
use serde::de::DeserializeOwned;
use serde_json as json;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_tungstenite::{tungstenite::protocol, WebSocketStream};

pub const WS_BROADCAST_CAPACITY: usize = 256;

fn reply(code: StatusCode) -> Result<Response<Body>> {
    Ok(Response::builder().status(code).body(Body::empty())?)
}

async fn from_request_body<T: DeserializeOwned>(request: Request<Body>) -> Result<T> {
    Ok(json::from_reader(
        hyper::body::aggregate(request).await?.reader(),
    )?)
}

async fn websocket<T: Clone + Send + 'static>(
    request: Request<Body>,
    sender: broadcast::Sender<T>,
    message_builder: impl Fn(T) -> protocol::Message + Send + Sync + 'static,
) -> Result<Response<Body>> {
    if let Some(key) = request.headers().typed_get::<headers::SecWebsocketKey>() {
        tokio::spawn(async move {
            match hyper::upgrade::on(request).await {
                Ok(upgraded) => {
                    let mut data_receiver = sender.subscribe();

                    let mut ws =
                        WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server, None)
                            .await;

                    loop {
                        match data_receiver.recv().await {
                            Ok(data) => {
                                if let Err(e) = ws.send(message_builder(data)).await {
                                    info!("Failed to send log with websocket: {e}");
                                    break;
                                }

                                ws.flush().await.ok();
                            }
                            Err(RecvError::Lagged(_)) => (),
                            Err(RecvError::Closed) => break,
                        }
                    }

                    ws.close(None).await.ok();
                }
                Err(e) => error!("{e}"),
            }
        });

        let mut response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .body(Body::empty())?;

        let h = response.headers_mut();
        h.typed_insert(headers::Upgrade::websocket());
        h.typed_insert(headers::SecWebsocketAccept::from(key));
        h.typed_insert(headers::Connection::upgrade());

        Ok(response)
    } else {
        reply(StatusCode::BAD_REQUEST)
    }
}

async fn http_api(
    connection_context: &ConnectionContext,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let mut response = match request.uri().path() {
        // New unified requests
        "/api/dashboard-request" => {
            if let Ok(request) = from_request_body::<ServerRequest>(request).await {
                match request {
                    ServerRequest::Log(event) => {
                        let level = event.severity.into_log_level();
                        log::log!(level, "{}", event.content);
                    }
                    ServerRequest::GetSession => {
                        alvr_events::send_event(EventType::Session(Box::new(
                            crate::SESSION_MANAGER.read().session().clone(),
                        )));
                    }
                    ServerRequest::UpdateSession(session) => {
                        *SESSION_MANAGER.write().session_mut() = *session
                    }
                    ServerRequest::SetValues(descs) => {
                        SESSION_MANAGER.write().set_values(descs).ok();
                    }
                    ServerRequest::UpdateClientList {
                        hostname,
                        mut action,
                    } => {
                        let mut session_manager = SESSION_MANAGER.write();
                        if matches!(action, ClientListAction::RemoveEntry) {
                            if let Some(entry) = session_manager.client_list().get(&hostname) {
                                if entry.connection_state != ConnectionState::Disconnected {
                                    connection_context
                                        .clients_to_be_removed
                                        .lock()
                                        .insert(hostname.clone());

                                    action = ClientListAction::SetConnectionState(
                                        ConnectionState::Disconnecting,
                                    )
                                };
                            }
                        }

                        session_manager.update_client_list(hostname, action);
                    }
                    ServerRequest::GetAudioDevices => {
                        if let Ok(list) = crate::SESSION_MANAGER.read().get_audio_devices_list() {
                            alvr_events::send_event(EventType::AudioDevices(list));
                        }
                    }
                    ServerRequest::CaptureFrame => {
                        connection_context
                            .events_sender
                            .send(ServerCoreEvent::CaptureFrame)
                            .ok();
                    }
                    ServerRequest::InsertIdr => {
                        connection_context
                            .events_sender
                            .send(ServerCoreEvent::RequestIDR)
                            .ok();
                    }
                    ServerRequest::StartRecording => crate::create_recording_file(
                        connection_context,
                        crate::SESSION_MANAGER.read().settings(),
                    ),
                    ServerRequest::StopRecording => {
                        *connection_context.video_recording_file.lock() = None
                    }
                    ServerRequest::FirewallRules(action) => {
                        if alvr_server_io::firewall_rules(action).is_ok() {
                            info!("Setting firewall rules succeeded!");
                        } else {
                            error!("Setting firewall rules failed!");
                        }
                    }
                    ServerRequest::RegisterAlvrDriver => {
                        alvr_server_io::driver_registration(
                            &[FILESYSTEM_LAYOUT
                                .get()
                                .unwrap()
                                .openvr_driver_root_dir
                                .clone()],
                            true,
                        )
                        .ok();

                        if let Ok(list) = alvr_server_io::get_registered_drivers() {
                            alvr_events::send_event(EventType::DriversList(list));
                        }
                    }
                    ServerRequest::UnregisterDriver(path) => {
                        alvr_server_io::driver_registration(&[path], false).ok();

                        if let Ok(list) = alvr_server_io::get_registered_drivers() {
                            alvr_events::send_event(EventType::DriversList(list));
                        }
                    }
                    ServerRequest::GetDriverList => {
                        if let Ok(list) = alvr_server_io::get_registered_drivers() {
                            alvr_events::send_event(EventType::DriversList(list));
                        }
                    }
                    ServerRequest::RestartSteamvr => {
                        connection_context
                            .events_sender
                            .send(ServerCoreEvent::RestartPending)
                            .ok();
                    }
                    ServerRequest::ShutdownSteamvr => {
                        connection_context
                            .events_sender
                            .send(ServerCoreEvent::ShutdownPending)
                            .ok();
                    }
                }

                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/events" => {
            websocket(request, LOGGING_EVENTS_SENDER.clone(), |e| {
                protocol::Message::Text(json::to_string(&e).unwrap())
            })
            .await?
        }
        "/api/video-mirror" => {
            let sender = {
                let mut sender_lock = connection_context.video_mirror_sender.lock();
                if let Some(sender) = &mut *sender_lock {
                    sender.clone()
                } else {
                    let (sender, _) = broadcast::channel(WS_BROADCAST_CAPACITY);
                    *sender_lock = Some(sender.clone());

                    sender
                }
            };

            if let Some(config) = &*connection_context.decoder_config.lock() {
                sender.send(config.config_buffer.clone()).ok();
            }

            let res = websocket(request, sender, protocol::Message::Binary).await?;

            connection_context
                .events_sender
                .send(ServerCoreEvent::RequestIDR)
                .ok();

            res
        }
        "/api/set-buttons" => {
            let button_entries = from_request_body::<Vec<ButtonEvent>>(request)
                .await?
                .iter()
                .map(|b| ButtonEntry {
                    path_id: alvr_common::hash_string(&b.path),
                    value: b.value,
                })
                .collect();

            connection_context
                .events_sender
                .send(ServerCoreEvent::Buttons(button_entries))
                .ok();

            reply(StatusCode::OK)?
        }
        "/api/average-video-latency-ms" => {
            let latency = if let Some(manager) = &*connection_context.statistics_manager.read() {
                manager.motion_to_photon_latency_average().as_millis()
            } else {
                0
            };

            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(latency.to_string().into())?
        }
        "/api/ping" => reply(StatusCode::OK)?,
        _ => reply(StatusCode::NOT_FOUND)?,
    };

    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_str("no-cache, no-store, must-revalidate")?,
    );
    response
        .headers_mut()
        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));

    Ok(response)
}

pub async fn web_server(connection_context: Arc<ConnectionContext>) -> Result<()> {
    let web_server_port = crate::SESSION_MANAGER
        .read()
        .settings()
        .connection
        .web_server_port;

    let service = service::make_service_fn(move |_| {
        let connection_context = Arc::clone(&connection_context);
        async move {
            Ok::<_, anyhow::Error>(service::service_fn(move |request| {
                let connection_context = Arc::clone(&connection_context);
                async move {
                    let res = http_api(&connection_context, request).await;
                    if let Err(e) = &res {
                        alvr_common::show_e(e);
                    }

                    res
                }
            }))
        }
    });

    Ok(hyper::Server::bind(&SocketAddr::new(
        "0.0.0.0".parse().unwrap(),
        web_server_port,
    ))
    .serve(service)
    .await?)
}
