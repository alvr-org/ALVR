use crate::{ClientListAction, ALVR_DIR, SESSION_MANAGER};
use alvr_common::{commands::*, data::*, logging::*, *};
use bytes::Buf;
use futures::SinkExt;
use headers::{self, HeaderMapExt};
use hyper::{
    header::{self, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL},
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json as json;
use std::{fs, io::Write, net::SocketAddr, path::PathBuf};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_tungstenite::{tungstenite::protocol, WebSocketStream};
use tokio_util::codec::{BytesCodec, FramedRead};

pub const WS_BROADCAST_CAPACITY: usize = 256;
const DASHBOARD_DIR_NAME_STR: &str = "dashboard";

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
        "/settings-schema" => reply_json(&settings_schema(session_settings_default()))?,
        "/session" => {
            if matches!(request.method(), &Method::GET) {
                reply_json(SESSION_MANAGER.lock().get())?
            } else if let Ok(data) = from_request_body::<json::Value>(request).await {
                // POST
                if let (Some(update_type), Some(update_author_id), Some(value)) = (
                    data.get("updateType"),
                    data.get("webClientId"),
                    data.get("session"),
                ) {
                    if let (Ok(update_type), Ok(update_author_id)) = (
                        json::from_value(update_type.clone()),
                        json::from_value::<String>(update_author_id.clone()),
                    ) {
                        let res = SESSION_MANAGER
                            .lock()
                            .get_mut(Some(update_author_id), update_type)
                            .merge_from_json(value);
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
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/log" => text_websocket(request, log_sender).await?,
        "/events" => text_websocket(request, events_sender).await?,
        "/driver/register" => {
            if driver_registration(&[ALVR_DIR.clone()], true).is_ok() {
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::INTERNAL_SERVER_ERROR)?
            }
        }
        "/driver/unregister" => {
            if let Ok(path) = from_request_body::<PathBuf>(request).await {
                if driver_registration(&[path], false).is_ok() {
                    reply(StatusCode::OK)?
                } else {
                    reply(StatusCode::INTERNAL_SERVER_ERROR)?
                }
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/driver/list" => reply_json(&get_registered_drivers().unwrap_or_default())?,
        uri @ "/firewall-rules/add" | uri @ "/firewall-rules/remove" => {
            let add = uri.ends_with("add");
            let maybe_err = firewall_rules(add).err();
            if let Some(e) = &maybe_err {
                error!("Setting firewall rules failed: code {}", e);
            }
            reply_json(&maybe_err.unwrap_or(0))?
        }
        "/graphics-devices" => reply_json(&graphics::get_gpu_names())?,
        "/restart-steamvr" => {
            crate::notify_restart_driver();
            reply(StatusCode::OK)?
        }
        "/client/add" => {
            if let Ok((device_name, hostname, ip)) =
                from_request_body::<(_, String, _)>(request).await
            {
                crate::update_client_list(
                    hostname.clone(),
                    ClientListAction::AddIfMissing {
                        device_name,
                        ip,
                        certificate_pem: None,
                    },
                )
                .await;
                crate::update_client_list(hostname, ClientListAction::TrustAndMaybeAddIp(Some(ip)))
                    .await;

                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/client/trust" => {
            if let Ok((hostname, maybe_ip)) = from_request_body(request).await {
                crate::update_client_list(hostname, ClientListAction::TrustAndMaybeAddIp(maybe_ip))
                    .await;
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/client/remove" => {
            if let Ok((hostname, maybe_ip)) = from_request_body(request).await {
                crate::update_client_list(hostname, ClientListAction::RemoveIpOrEntry(maybe_ip))
                    .await;
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/version" => Response::new(ALVR_VERSION.to_string().into()),
        "/open" => {
            if let Ok(url) = from_request_body::<String>(request).await {
                webbrowser::open(&url).ok();
                reply(StatusCode::OK)?
            } else {
                reply(StatusCode::BAD_REQUEST)?
            }
        }
        "/update" => {
            if let Ok(url) = from_request_body::<String>(request).await {
                let redirection_response = trace_err!(reqwest::get(&url).await)?;
                let mut resource_response =
                    trace_err!(reqwest::get(redirection_response.url().clone()).await)?;

                let mut file = trace_err!(fs::File::create(commands::installer_path()))?;

                let mut downloaded_bytes_count = 0;
                loop {
                    match resource_response.chunk().await {
                        Ok(Some(chunk)) => {
                            downloaded_bytes_count += chunk.len();
                            trace_err!(file.write_all(&chunk))?;
                            log_id(LogId::UpdateDownloadedBytesCount(downloaded_bytes_count));
                        }
                        Ok(None) => break,
                        Err(e) => {
                            log_id(LogId::UpdateDownloadError);
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
                    ALVR_DIR.join(DASHBOARD_DIR_NAME_STR).to_string_lossy(),
                    path_branch
                ))
                .await;

                if let Ok(file) = maybe_file {
                    Response::new(Body::wrap_stream(FramedRead::new(file, BytesCodec::new())))
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

    let service = make_service_fn(|_| {
        let log_sender = log_sender.clone();
        let events_sender = events_sender.clone();
        async move {
            StrResult::Ok(service_fn(move |request| {
                let log_sender = log_sender.clone();
                let events_sender = events_sender.clone();
                async move {
                    let res = http_api(request, log_sender, events_sender).await;
                    if let Err(e) = &res {
                        show_e(e);
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
