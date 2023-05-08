use crate::{
    DECODER_CONFIG, FILESYSTEM_LAYOUT, SERVER_DATA_MANAGER, VIDEO_MIRROR_SENDER,
    VIDEO_RECORDING_FILE,
};
use alvr_common::{log, prelude::*};
use alvr_events::Event;
use alvr_packets::{ServerRequest, ServerResponse};
use bytes::Buf;
use futures::SinkExt;
use headers::HeaderMapExt;
use hyper::{
    header::{self, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, CONTENT_TYPE},
    service, Body, Request, Response, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json as json;
use std::net::SocketAddr;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_tungstenite::{tungstenite::protocol, WebSocketStream};
use tokio_util::codec::{BytesCodec, FramedRead};

pub const WS_BROADCAST_CAPACITY: usize = 256;

fn reply(code: StatusCode) -> StrResult<Response<Body>> {
    Response::builder()
        .status(code)
        .body(Body::empty())
        .map_err(err!())
}

fn reply_json<T: Serialize>(obj: &T) -> StrResult<Response<Body>> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(json::to_string(obj).map_err(err!())?.into())
        .map_err(err!())
}

async fn from_request_body<T: DeserializeOwned>(request: Request<Body>) -> StrResult<T> {
    json::from_reader(
        hyper::body::aggregate(request)
            .await
            .map_err(err!())?
            .reader(),
    )
    .map_err(err!())
}

async fn websocket<T: Clone + Send + 'static>(
    request: Request<Body>,
    sender: broadcast::Sender<T>,
    message_builder: impl Fn(T) -> protocol::Message + Send + Sync + 'static,
) -> StrResult<Response<Body>> {
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
                            Err(RecvError::Lagged(_)) => {
                                warn!("Some log lines have been lost because the buffer is full");
                            }
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
            .body(Body::empty())
            .map_err(err!())?;

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
    request: Request<Body>,
    events_sender: broadcast::Sender<Event>,
) -> StrResult<Response<Body>> {
    let mut response = match request.uri().path() {
        // New unified requests
        "/api/dashboard-request" => {
            if let Ok(request) = from_request_body::<ServerRequest>(request).await {
                match request {
                    ServerRequest::Ping => (),
                    ServerRequest::Log(event) => {
                        let level = event.severity.into_log_level();
                        log::log!(level, "{}", event.content);
                    }
                    ServerRequest::GetSession => {
                        alvr_events::send_event(alvr_events::EventType::Session(Box::new(
                            SERVER_DATA_MANAGER.read().session().clone(),
                        )));
                    }
                    ServerRequest::UpdateSession(session) => {
                        *SERVER_DATA_MANAGER.write().session_mut() = *session
                    }
                    ServerRequest::SetValues(descs) => {
                        SERVER_DATA_MANAGER.write().set_values(descs).ok();
                    }
                    ServerRequest::UpdateClientList { hostname, action } => SERVER_DATA_MANAGER
                        .write()
                        .update_client_list(hostname, action),
                    ServerRequest::GetAudioDevices => {
                        if let Ok(list) = SERVER_DATA_MANAGER.read().get_audio_devices_list() {
                            return reply_json(&ServerResponse::AudioDevices(list));
                        }
                    }
                    ServerRequest::CaptureFrame => unsafe { crate::CaptureFrame() },
                    ServerRequest::InsertIdr => unsafe { crate::RequestIDR() },
                    ServerRequest::StartRecording => crate::create_recording_file(),
                    ServerRequest::StopRecording => *VIDEO_RECORDING_FILE.lock() = None,
                    ServerRequest::FirewallRules(action) => {
                        if alvr_server_io::firewall_rules(action).is_ok() {
                            info!("Setting firewall rules succeeded!");
                        } else {
                            error!("Setting firewall rules failed!");
                        }
                    }
                    ServerRequest::RegisterAlvrDriver => {
                        alvr_server_io::driver_registration(
                            &[FILESYSTEM_LAYOUT.openvr_driver_root_dir.clone()],
                            true,
                        )
                        .ok();
                    }
                    ServerRequest::UnregisterDriver(path) => {
                        alvr_server_io::driver_registration(&[path], false).ok();
                    }
                    ServerRequest::GetDriverList => {
                        if let Ok(list) = alvr_server_io::get_registered_drivers() {
                            return reply_json(&ServerResponse::DriversList(list));
                        }
                    }
                    ServerRequest::RestartSteamvr => crate::notify_restart_driver(),
                    ServerRequest::ShutdownSteamvr => crate::notify_shutdown_driver(),
                }

                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/events" => {
            websocket(request, events_sender, |e| {
                protocol::Message::Text(json::to_string(&e).unwrap())
            })
            .await?
        }
        "/api/video-mirror" => {
            let sender = {
                let mut sender_lock = VIDEO_MIRROR_SENDER.lock();
                if let Some(sender) = &mut *sender_lock {
                    sender.clone()
                } else {
                    let (sender, _) = broadcast::channel(WS_BROADCAST_CAPACITY);
                    *sender_lock = Some(sender.clone());

                    sender
                }
            };

            if let Some(config) = &*DECODER_CONFIG.lock() {
                sender.send(config.config_buffer.clone()).ok();
            }

            let res = websocket(request, sender, protocol::Message::Binary).await?;

            unsafe { crate::RequestIDR() };

            res
        }
        other_uri => {
            if other_uri.contains("..") {
                // Attempted tree traversal
                reply(StatusCode::FORBIDDEN)?
            } else {
                let path_branch = match other_uri {
                    "/" => "/index.html",
                    other_path => other_path,
                };

                let maybe_file = tokio::fs::File::open(format!(
                    "{}{path_branch}",
                    FILESYSTEM_LAYOUT.dashboard_dir().to_string_lossy(),
                ))
                .await;

                if let Ok(file) = maybe_file {
                    let mut builder = Response::builder();
                    if other_uri.ends_with(".wasm") {
                        builder = builder.header(CONTENT_TYPE, "application/wasm");
                    }

                    builder
                        .body(Body::wrap_stream(FramedRead::new(file, BytesCodec::new())))
                        .map_err(err!())?
                } else {
                    reply(StatusCode::NOT_FOUND)?
                }
            }
        }
    };

    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_str("no-cache, no-store, must-revalidate").map_err(err!())?,
    );
    response
        .headers_mut()
        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));

    Ok(response)
}

pub async fn web_server(events_sender: broadcast::Sender<Event>) -> StrResult {
    let web_server_port = SERVER_DATA_MANAGER
        .read()
        .settings()
        .connection
        .web_server_port;

    let service = service::make_service_fn(|_| {
        let events_sender = events_sender.clone();
        async move {
            StrResult::Ok(service::service_fn(move |request| {
                let events_sender = events_sender.clone();
                async move {
                    let res = http_api(request, events_sender).await;
                    if let Err(e) = &res {
                        alvr_common::show_e(e);
                    }

                    res
                }
            }))
        }
    });

    hyper::Server::bind(&SocketAddr::new(
        "0.0.0.0".parse().unwrap(),
        web_server_port,
    ))
    .serve(service)
    .await
    .map_err(err!())
}
