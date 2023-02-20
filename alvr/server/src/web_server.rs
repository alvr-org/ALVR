use crate::{
    DECODER_CONFIG, DISCONNECT_CLIENT_NOTIFIER, FILESYSTEM_LAYOUT, SERVER_DATA_MANAGER,
    VIDEO_MIRROR_SENDER, VIDEO_RECORDING_FILE,
};
use alvr_common::{log, prelude::*, ALVR_VERSION};
use alvr_events::Event;
use alvr_sockets::{ClientListAction, DashboardRequest};
use bytes::Buf;
use futures::SinkExt;
use headers::HeaderMapExt;
use hyper::{
    header::{self, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, CONTENT_TYPE},
    service, Body, Request, Response, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json as json;
use std::{
    env::consts::OS,
    net::{IpAddr, SocketAddr},
};
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
    log_sender: broadcast::Sender<String>,
    events_sender: broadcast::Sender<Event>,
) -> StrResult<Response<Body>> {
    let mut response = match request.uri().path() {
        // New unified requests
        "/api/dashboard-request" => {
            if let Ok(request) = from_request_body::<DashboardRequest>(request).await {
                match request {
                    DashboardRequest::GetSession => {
                        alvr_events::send_event(alvr_events::EventType::Session(Box::new(
                            SERVER_DATA_MANAGER.read().session().clone(),
                        )));
                    }
                    DashboardRequest::UpdateSession(session) => {
                        *SERVER_DATA_MANAGER.write().session_mut() = *session
                    }
                    DashboardRequest::SetSingleValue { path, new_value } => {
                        SERVER_DATA_MANAGER
                            .write()
                            .set_single_value(path, new_value)
                            .ok();
                    }
                    DashboardRequest::ExecuteScript(code) => {
                        if let Err(e) = SERVER_DATA_MANAGER.write().execute_script(&code) {
                            error!("Error executing script: {e}");
                        }
                    }
                    DashboardRequest::UpdateClientList { hostname, action } => SERVER_DATA_MANAGER
                        .write()
                        .update_client_list(hostname, action),
                    DashboardRequest::GetAudioDevices => {
                        if let Ok(list) = SERVER_DATA_MANAGER.read().get_audio_devices_list() {
                            return reply_json(&list);
                        }
                    }
                    DashboardRequest::RestartSteamvr => crate::notify_restart_driver(),
                    DashboardRequest::Log(event) => {
                        let level = event.severity.into_log_level();
                        log::log!(level, "{}", event.content);
                    }
                    DashboardRequest::Ping => (),
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
                sender.send(config.clone()).ok();
            }

            let res = websocket(request, sender, protocol::Message::Binary).await?;

            unsafe { crate::RequestIDR() };

            res
        }
        // Legacy requests
        "/api/log" => websocket(request, log_sender, protocol::Message::Text).await?,
        "/api/session/load" => reply_json(SERVER_DATA_MANAGER.read().session())?,
        "/api/client/add" => {
            if let Ok((hostname, ip)) = from_request_body::<(String, _)>(request).await {
                let mut data_manager = SERVER_DATA_MANAGER.write();
                data_manager.update_client_list(
                    hostname,
                    ClientListAction::AddIfMissing {
                        trusted: true,
                        manual_ips: vec![ip],
                    },
                );

                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/client/trust" => {
            if let Ok((hostname, maybe_ip)) = from_request_body::<(String, _)>(request).await {
                let mut data_manager = SERVER_DATA_MANAGER.write();
                data_manager.update_client_list(hostname.clone(), ClientListAction::Trust);
                if let Some(ip) = maybe_ip {
                    data_manager
                        .update_client_list(hostname, ClientListAction::SetManualIps(vec![ip]));
                }
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/client/remove" => {
            if let Ok((hostname, maybe_ip)) =
                from_request_body::<(String, Option<IpAddr>)>(request).await
            {
                let mut data_manager = SERVER_DATA_MANAGER.write();
                if maybe_ip.is_some() {
                    data_manager
                        .update_client_list(hostname, ClientListAction::SetManualIps(vec![]));
                } else {
                    data_manager.update_client_list(hostname, ClientListAction::RemoveEntry);
                }
                DISCONNECT_CLIENT_NOTIFIER.notify_waiters();

                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/version" => Response::new(ALVR_VERSION.to_string().into()),
        "/api/server-os" => Response::new(OS.into()),
        "/api/capture-frame" => {
            unsafe { crate::CaptureFrame() };
            return reply(StatusCode::OK);
        }
        "/api/insert-idr" => {
            unsafe { crate::RequestIDR() };
            return reply(StatusCode::OK);
        }
        "/api/start-recording" => {
            crate::create_recording_file();
            return reply(StatusCode::OK);
        }
        "/api/stop-recording" => {
            *VIDEO_RECORDING_FILE.lock() = None;
            return reply(StatusCode::OK);
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

pub async fn web_server(
    log_sender: broadcast::Sender<String>,
    events_sender: broadcast::Sender<Event>,
) -> StrResult {
    let web_server_port = SERVER_DATA_MANAGER
        .read()
        .settings()
        .connection
        .web_server_port;

    let service = service::make_service_fn(|_| {
        let log_sender = log_sender.clone();
        let events_sender = events_sender.clone();
        async move {
            StrResult::Ok(service::service_fn(move |request| {
                let log_sender = log_sender.clone();
                let events_sender = events_sender.clone();
                async move {
                    let res = http_api(request, log_sender, events_sender).await;
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
