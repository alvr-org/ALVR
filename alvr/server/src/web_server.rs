use crate::{graphics_info, ClientListAction, FILESYSTEM_LAYOUT, SESSION_MANAGER};
use alvr_common::{logging, prelude::*, ALVR_VERSION};
use bytes::Buf;
use futures::SinkExt;
use headers::HeaderMapExt;
use hyper::{
    header::{self, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, CONTENT_TYPE},
    service, Body, Request, Response, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json as json;
use std::{fs, io::Write, net::SocketAddr, path::PathBuf};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_tungstenite::{tungstenite::protocol, WebSocketStream};
use tokio_util::codec::{BytesCodec, FramedRead};

pub const WS_BROADCAST_CAPACITY: usize = 256;

fn reply(code: StatusCode) -> StrResult<Response<Body>> {
    trace_err!(Response::builder().status(code).body(Body::empty()))
}

fn reply_json<T: Serialize>(obj: &T) -> StrResult<Response<Body>> {
    trace_err!(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(trace_err!(json::to_string(obj))?.into()))
}

async fn from_request_body<T: DeserializeOwned>(request: Request<Body>) -> StrResult<T> {
    trace_err!(json::from_reader(
        trace_err!(hyper::body::aggregate(request).await)?.reader()
    ))
}

async fn text_websocket(
    request: Request<Body>,
    sender: broadcast::Sender<String>,
) -> StrResult<Response<Body>> {
    if let Some(key) = request.headers().typed_get::<headers::SecWebsocketKey>() {
        tokio::spawn(async move {
            match hyper::upgrade::on(request).await {
                Ok(upgraded) => {
                    let mut log_receiver = sender.subscribe();

                    let mut ws =
                        WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server, None)
                            .await;

                    loop {
                        match log_receiver.recv().await {
                            Ok(line) => {
                                if let Err(e) = ws.send(protocol::Message::text(line)).await {
                                    info!("Failed to send log with websocket: {}", e);
                                    break;
                                }
                            }
                            Err(RecvError::Lagged(_)) => {
                                warn!("Some log lines have been lost because the buffer is full");
                            }
                            Err(RecvError::Closed) => break,
                        }
                    }

                    ws.close(None).await.ok();
                }
                Err(e) => error!("{}", e),
            }
        });

        let mut response = trace_err!(Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .body(Body::empty()))?;

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
    events_sender: broadcast::Sender<String>,
) -> StrResult<Response<Body>> {
    let mut response = match request.uri().path() {
        "/api/settings-schema" => reply_json(&alvr_session::settings_schema(
            alvr_session::session_settings_default(),
        ))?,
        "/api/session/load" => reply_json(SESSION_MANAGER.lock().get())?,
        "/api/session/store-settings" => {
            if let Ok(session_settings) = from_request_body::<json::Value>(request).await {
                let res = SESSION_MANAGER
                    .lock()
                    .get_mut()
                    .merge_from_json(&json::json!({ "session_settings": session_settings }));
                if let Err(e) = res {
                    warn!("{}", e);
                    // HTTP Code: WARNING
                    reply(trace_err!(StatusCode::from_u16(199))?)?
                } else {
                    reply(StatusCode::OK)?
                }
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/session/store" => {
            if let Ok(data) = from_request_body::<json::Value>(request).await {
                if let Some(value) = data.get("session") {
                    let res = SESSION_MANAGER.lock().get_mut().merge_from_json(value);
                    if let Err(e) = res {
                        warn!("{}", e);
                        // HTTP Code: WARNING
                        reply(trace_err!(StatusCode::from_u16(199))?)?
                    } else {
                        reply(StatusCode::OK)?
                    }
                } else {
                    reply(StatusCode::BAD_REQUEST)?
                }
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/log" => text_websocket(request, log_sender).await?,
        "/api/events" => text_websocket(request, events_sender).await?,
        "/api/driver/register" => {
            if alvr_commands::driver_registration(
                &[FILESYSTEM_LAYOUT.openvr_driver_root_dir.clone()],
                true,
            )
            .is_ok()
            {
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::INTERNAL_SERVER_ERROR)?
            }
        }
        "/api/driver/unregister" => {
            if let Ok(path) = from_request_body::<PathBuf>(request).await {
                if alvr_commands::driver_registration(&[path], false).is_ok() {
                    reply(StatusCode::OK)?
                } else {
                    reply(StatusCode::INTERNAL_SERVER_ERROR)?
                }
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/driver/list" => {
            reply_json(&alvr_commands::get_registered_drivers().unwrap_or_default())?
        }
        uri @ "/firewall-rules/add" | uri @ "/firewall-rules/remove" => {
            let add = uri.ends_with("add");
            let maybe_err = alvr_commands::firewall_rules(add).err();
            if let Some(e) = &maybe_err {
                error!("Setting firewall rules failed: code {}", e);
            }
            reply_json(&maybe_err.unwrap_or(0))?
        }
        "/api/audio-devices" => reply_json(&alvr_audio::get_devices_list()?)?,
        "/api/graphics-devices" => reply_json(&graphics_info::get_gpu_names())?,
        "/restart-steamvr" => {
            crate::notify_restart_driver();
            reply(StatusCode::OK)?
        }
        "/api/client/add" => {
            if let Ok((display_name, hostname, ip)) =
                from_request_body::<(_, String, _)>(request).await
            {
                crate::update_client_list(
                    hostname.clone(),
                    ClientListAction::AddIfMissing { display_name },
                )
                .await;
                crate::update_client_list(hostname, ClientListAction::TrustAndMaybeAddIp(Some(ip)))
                    .await;

                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/client/trust" => {
            if let Ok((hostname, maybe_ip)) = from_request_body(request).await {
                crate::update_client_list(hostname, ClientListAction::TrustAndMaybeAddIp(maybe_ip))
                    .await;
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/client/remove" => {
            if let Ok((hostname, maybe_ip)) = from_request_body(request).await {
                crate::update_client_list(hostname, ClientListAction::RemoveIpOrEntry(maybe_ip))
                    .await;
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/version" => Response::new(ALVR_VERSION.to_string().into()),
        "/api/open" => {
            if let Ok(url) = from_request_body::<String>(request).await {
                webbrowser::open(&url).ok();
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/api/update" => {
            if let Ok(url) = from_request_body::<String>(request).await {
                let redirection_response = trace_err!(reqwest::get(&url).await)?;
                let mut resource_response =
                    trace_err!(reqwest::get(redirection_response.url().clone()).await)?;

                let mut file = trace_err!(fs::File::create(alvr_filesystem::installer_path()))?;

                let mut downloaded_bytes_count = 0;
                loop {
                    match resource_response.chunk().await {
                        Ok(Some(chunk)) => {
                            downloaded_bytes_count += chunk.len();
                            trace_err!(file.write_all(&chunk))?;
                            log_event(Event::UpdateDownloadedBytesCount(downloaded_bytes_count));
                        }
                        Ok(None) => break,
                        Err(e) => {
                            log_event(Event::UpdateDownloadError);
                            error!("Download update failed: {}", e);
                            return reply(StatusCode::BAD_GATEWAY);
                        }
                    }
                }

                crate::notify_application_update();
            }
            reply(StatusCode::BAD_REQUEST)?
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
                    "{}{}",
                    FILESYSTEM_LAYOUT.dashboard_dir().to_string_lossy(),
                    path_branch
                ))
                .await;

                if let Ok(file) = maybe_file {
                    let mut builder = Response::builder();
                    if other_uri.ends_with(".wasm") {
                        builder = builder.header(CONTENT_TYPE, "application/wasm");
                    }
                    trace_err!(
                        builder.body(Body::wrap_stream(FramedRead::new(file, BytesCodec::new())))
                    )?
                } else {
                    reply(StatusCode::NOT_FOUND)?
                }
            }
        }
    };

    response.headers_mut().insert(
        CACHE_CONTROL,
        trace_err!(HeaderValue::from_str("no-cache, no-store, must-revalidate"))?,
    );
    response
        .headers_mut()
        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));

    Ok(response)
}

pub async fn web_server(
    log_sender: broadcast::Sender<String>,
    events_sender: broadcast::Sender<String>,
) -> StrResult {
    let web_server_port = SESSION_MANAGER
        .lock()
        .get()
        .to_settings()
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
                        logging::show_e(e);
                    }

                    res
                }
            }))
        }
    });

    trace_err!(
        hyper::Server::bind(&SocketAddr::new(
            "0.0.0.0".parse().unwrap(),
            web_server_port
        ))
        .serve(service)
        .await
    )
}
