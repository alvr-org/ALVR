use crate::{
    ConnectionContext, FILESYSTEM_LAYOUT, SESSION_MANAGER, ServerCoreEvent,
    logging_backend::EVENTS_SENDER,
};
use alvr_common::{ConnectionState, anyhow::Result, error, info, log};
use alvr_events::{ButtonEvent, EventType};
use alvr_packets::{ButtonEntry, ClientListAction, ServerRequest};
use axum::{
    Json, Router,
    extract::{Request, State, WebSocketUpgrade, ws::Message},
    http::{
        HeaderValue, Method, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    middleware,
    response::Response,
    routing,
};
use serde_json as json;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast::error::RecvError;
use tower_http::{
    cors::{self, CorsLayer},
    set_header::SetResponseHeaderLayer,
};

const X_ALVR: &str = "X-ALVR";

// This is the actual core part of cors
// We require the X-ALVR header, but the browser forces a cors preflight
// if the site tries to send a request with it set since it's not-whitelisted
//
// The dashboard can just set the header and be allowed through without the preflight
// thus not getting blocked by allow_untrusted_http being disabled
async fn ensure_preflight(request: Request, next: middleware::Next) -> Response {
    if request.headers().contains_key(X_ALVR) || request.method() == Method::OPTIONS {
        next.run(request).await
    } else {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(format!("missing {X_ALVR} header").into())
            .unwrap()
    }
}

pub async fn web_server(connection_context: Arc<ConnectionContext>) -> Result<()> {
    let allow_untrusted_http;
    let web_server_port;

    {
        let session_manager = SESSION_MANAGER.read();
        allow_untrusted_http = session_manager.settings().connection.allow_untrusted_http;
        web_server_port = session_manager.settings().connection.web_server_port;
    }

    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE, X_ALVR.parse().unwrap()]);
    if allow_untrusted_http {
        cors = cors.allow_origin(cors::Any);
    }

    let router = Router::new()
        .route("/api/dashboard-request", routing::post(dashboard_request))
        .route("/api/events", routing::get(events))
        .route("/api/set-buttons", routing::post(set_buttons))
        .route(
            "/api/version",
            routing::get(async || alvr_common::ALVR_VERSION.to_string()),
        )
        .route("/api/ping", routing::get(async || ()))
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        ))
        .layer(middleware::from_fn(ensure_preflight))
        .with_state(connection_context);

    axum::serve(
        tokio::net::TcpListener::bind(SocketAddr::new([0, 0, 0, 0].into(), web_server_port))
            .await
            .unwrap(),
        router,
    )
    .await?;

    Ok(())
}

async fn dashboard_request(
    State(ctx): State<Arc<ConnectionContext>>,
    Json(request): Json<ServerRequest>,
) {
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
        ServerRequest::UpdateSession(session) => *SESSION_MANAGER.write().session_mut() = *session,
        ServerRequest::SetValues(descs) => {
            SESSION_MANAGER.write().set_values(descs).ok();
        }
        ServerRequest::UpdateClientList {
            hostname,
            mut action,
        } => {
            let mut session_manager = SESSION_MANAGER.write();
            if matches!(action, ClientListAction::RemoveEntry)
                && let Some(entry) = session_manager.client_list().get(&hostname)
                && entry.connection_state != ConnectionState::Disconnected
            {
                ctx.clients_to_be_removed.lock().insert(hostname.clone());

                action = ClientListAction::SetConnectionState(ConnectionState::Disconnecting);
            }

            session_manager.update_client_list(hostname, action);
        }
        ServerRequest::CaptureFrame => {
            ctx.events_sender.send(ServerCoreEvent::CaptureFrame).ok();
        }
        ServerRequest::InsertIdr => {
            ctx.events_sender.send(ServerCoreEvent::RequestIDR).ok();
        }
        ServerRequest::StartRecording => {
            crate::create_recording_file(&ctx, crate::SESSION_MANAGER.read().settings())
        }
        ServerRequest::StopRecording => *ctx.video_recording_file.lock() = None,
        ServerRequest::FirewallRules(action) => {
            if let Err(e) = alvr_server_io::firewall_rules(action, FILESYSTEM_LAYOUT.get().unwrap())
            {
                error!("Setting firewall rules failed! code: {e}");
            } else {
                info!("Setting firewall rules succeeded!");
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
            ctx.events_sender.send(ServerCoreEvent::RestartPending).ok();
        }
        ServerRequest::ShutdownSteamvr => {
            ctx.events_sender
                .send(ServerCoreEvent::ShutdownPending)
                .ok();
        }
    }
}

async fn events(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(async |mut ws| {
        let mut events_receiver = EVENTS_SENDER.subscribe();

        loop {
            match events_receiver.recv().await {
                Ok(event) => {
                    if let Err(e) = ws
                        .send(Message::Text(json::to_string(&event).unwrap().into()))
                        .await
                    {
                        info!("Failed to send event with websocket: {e}");
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => (),
                Err(RecvError::Closed) => break,
            }
        }
    })
}

async fn set_buttons(
    State(ctx): State<Arc<ConnectionContext>>,
    Json(button_events): Json<Vec<ButtonEvent>>,
) {
    let button_entries = button_events
        .iter()
        .map(|b| ButtonEntry {
            path_id: alvr_common::hash_string(&b.path),
            value: b.value,
        })
        .collect();

    ctx.events_sender
        .send(ServerCoreEvent::Buttons(button_entries))
        .ok();
}
