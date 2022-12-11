use alvr_client_core::ClientEvent;
use alvr_common::{once_cell::sync::Lazy, prelude::*};
use futures::SinkExt;
use headers::HeaderMapExt;
use hyper::{
    body::Buf,
    header::{self, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL},
    service, Body, Request, Response, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json as json;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_tungstenite::{tungstenite::protocol, WebSocketStream};

static NAL_SENDER: Lazy<broadcast::Sender<(Duration, Vec<u8>)>> =
    Lazy::new(|| broadcast::channel(256).0);

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
                            }
                            Err(RecvError::Lagged(_)) => {
                                warn!("Some nals have been lost because the buffer is full");
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

async fn http_api(request: Request<Body>) -> StrResult<Response<Body>> {
    let mut response = match request.uri().path() {
        "/api/path-string-to-id" => reply_json(&alvr_common::hash_string(
            &from_request_body::<String>(request).await.unwrap(),
        ))?,
        // note: logs are collected by alvr_client_core and sent to the server
        "/api/log-error" => {
            error!("{}", from_request_body::<String>(request).await.unwrap());
            reply(StatusCode::OK)?
        }
        "/api/log-warn" => {
            warn!("{}", from_request_body::<String>(request).await.unwrap());
            reply(StatusCode::OK)?
        }
        "/api/log-info" => {
            info!("{}", from_request_body::<String>(request).await.unwrap());
            reply(StatusCode::OK)?
        }
        "/api/log-debug" => {
            debug!("{}", from_request_body::<String>(request).await.unwrap());
            reply(StatusCode::OK)?
        }
        // request body: { recommended_view_resolution: [int, int], supported_refresh_rates: [float, float, ...] }
        "/api/initialize" => {
            let data = from_request_body::<json::Value>(request).await.unwrap();
            alvr_client_core::initialize(
                json::from_value(data["recommended_view_resolution"].clone()).unwrap(),
                json::from_value(data["supported_refresh_rates"].clone()).unwrap(),
                true,
            );

            reply(StatusCode::OK)?
        }
        "/api/destroy" => {
            alvr_client_core::destroy();
            reply(StatusCode::OK)?
        }
        "/api/resume" => {
            alvr_client_core::resume();
            reply(StatusCode::OK)?
        }
        // response: ClientEvent or null. The FrameReady variant only returns the timestamp, get the
        // nal stream through /api/nal-stream
        "/api/poll-event" => match alvr_client_core::poll_event() {
            Some(ClientEvent::FrameReady { timestamp, nal }) => {
                NAL_SENDER.send((timestamp, nal)).ok();

                reply_json(&json::json!({ "FrameReady": timestamp }))?
            }
            Some(event) => reply_json(&event)?,
            None => reply_json(&None::<()>)?,
        },
        // packet: [ timestamp (8 bytes secs, 4 bytes nanos) | payload length (8 bytes) | payload ]
        "/api/nal-stream" => {
            websocket(request, NAL_SENDER.clone(), |e| {
                protocol::Message::Binary(bincode::serialize(&e).unwrap())
            })
            .await?
        }
        // request body: { fov: [Fov, Fov], ipd_m: float }
        "/api/send-views-config" => {
            let data = from_request_body::<json::Value>(request).await.unwrap();
            alvr_client_core::send_views_config(
                json::from_value(data["fov"].clone()).unwrap(),
                data["ipd_m"].as_f64().unwrap() as _,
            );

            reply(StatusCode::OK)?
        }
        // request body: { device_id: int, gauge_value: float, is_plugged: bool }
        "/api/send-battery" => {
            let data = from_request_body::<json::Value>(request).await.unwrap();
            alvr_client_core::send_battery(
                data["device_id"].as_u64().unwrap(),
                data["gauge_value"].as_f64().unwrap() as _,
                data["is_plugged"].as_bool().unwrap(),
            );

            reply(StatusCode::OK)?
        }
        // request body: [float float]
        "/api/send-playspace" => {
            alvr_client_core::send_playspace(from_request_body(request).await.unwrap());

            reply(StatusCode::OK)?
        }
        // request body: { path_id: int, value: ButtonValue }
        "/api/send-button" => {
            let data = from_request_body::<json::Value>(request).await.unwrap();
            alvr_client_core::send_button(
                data["path_id"].as_u64().unwrap(),
                json::from_value(data["value"].clone()).unwrap(),
            );

            reply(StatusCode::OK)?
        }
        // request body: Tracking
        "/api/send-tracking" => {
            alvr_client_core::send_tracking(from_request_body(request).await.unwrap());

            reply(StatusCode::OK)?
        }
        // response: Duration
        "/api/get-prediction-offset" => reply_json(&alvr_client_core::get_prediction_offset())?,
        // request body: { target_timestamp: Duration, vsync_queue: Duration }
        "/api/report-submit" => {
            let data = from_request_body::<json::Value>(request).await.unwrap();
            alvr_client_core::report_submit(
                json::from_value(data["target_timestamp"].clone()).unwrap(),
                json::from_value(data["vsync_queue"].clone()).unwrap(),
            );

            reply(StatusCode::OK)?
        }
        "/api/request-idr" => {
            alvr_client_core::request_idr();

            reply(StatusCode::OK)?
        }
        // request body: Duration
        "/api/report-frame-decoded" => {
            alvr_client_core::report_frame_decoded(from_request_body(request).await.unwrap());

            reply(StatusCode::OK)?
        }
        // request body: Duration
        "/api/report-compositor-start" => {
            alvr_client_core::report_compositor_start(from_request_body(request).await.unwrap());

            reply(StatusCode::OK)?
        }
        _ => reply(StatusCode::BAD_REQUEST)?,
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

async fn web_server() -> StrResult {
    let service = service::make_service_fn(|_| async move {
        StrResult::Ok(service::service_fn(move |request| async move {
            let res = http_api(request).await;
            if let Err(e) = &res {
                alvr_common::show_e(e);
            }

            res
        }))
    });

    hyper::Server::bind(&SocketAddr::new("0.0.0.0".parse().unwrap(), 8083))
        .serve(service)
        .await
        .map_err(err!())
}

#[tokio::main]
async fn main() {
    alvr_client_core::init_logging();

    alvr_common::show_err(web_server().await);
}
