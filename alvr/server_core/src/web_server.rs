use crate::{
    ConnectionContext, FILESYSTEM_LAYOUT, SESSION_MANAGER, ServerCoreEvent,
    logging_backend::EVENTS_SENDER,
};
use alvr_common::{ConnectionState, LogEntry, anyhow::Result, error, info, log};
use alvr_events::{ButtonEvent, EventType};
use alvr_packets::{ButtonEntry, ClientConnectionsAction, FirewallRulesAction, PathValuePairList};
use alvr_session::SessionConfig;
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
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, sync::broadcast::error::RecvError};
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
        .nest(
            "/api",
            Router::new()
                .route("/events", routing::get(events_websocket))
                .route("/log", routing::post(set_log))
                .nest(
                    "/session",
                    Router::new()
                        .route("/", routing::get(get_session).post(update_session))
                        .route("/values", routing::post(set_session_values))
                        .route(
                            "/client-connections",
                            routing::post(update_client_connections),
                        ),
                )
                .route("/buttons", routing::post(set_buttons))
                .route("/insert-idr", routing::post(insert_idr))
                .route("/capture-frame", routing::post(capture_frame))
                .nest(
                    "/recording",
                    Router::new()
                        .route("/start", routing::post(start_recording))
                        .route("/stop", routing::post(stop_recording)),
                )
                .nest(
                    "/firewall-rules",
                    Router::new()
                        .route("/add", routing::post(add_firewall_rules))
                        .route("/remove", routing::post(remove_firewall_rules)),
                )
                .nest(
                    "/drivers",
                    Router::new()
                        .route("/", routing::get(get_driver_list))
                        .route("/register-alvr", routing::post(register_alvr_driver))
                        .route("/unregister", routing::post(unregister_driver)),
                )
                .nest(
                    "/steamvr",
                    Router::new()
                        .route("/restart", routing::post(restart_steamvr))
                        .route("/shutdown", routing::post(shutdown_steamvr)),
                )
                .route(
                    "/version",
                    routing::get(async || alvr_common::ALVR_VERSION.to_string()),
                )
                .route("/ping", routing::get(async || ())),
        )
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        ))
        .layer(middleware::from_fn(ensure_preflight))
        .with_state(connection_context);

    axum::serve(
        TcpListener::bind(SocketAddr::new([0, 0, 0, 0].into(), web_server_port))
            .await
            .unwrap(),
        router,
    )
    .await?;

    Ok(())
}

async fn events_websocket(ws: WebSocketUpgrade) -> Response {
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

async fn set_log(Json(entry): Json<LogEntry>) {
    let level = entry.severity.into_log_level();
    log::log!(level, "{}", entry.content);
}

async fn get_session() {
    alvr_events::send_event(EventType::Session(Box::new(
        crate::SESSION_MANAGER.read().session().clone(),
    )));
}

async fn update_session(Json(config): Json<SessionConfig>) {
    *SESSION_MANAGER.write().session_mut() = config;
}

async fn set_session_values(Json(descs): Json<PathValuePairList>) {
    SESSION_MANAGER.write().set_session_values(descs).ok();
}

async fn update_client_connections(
    State(ctx): State<Arc<ConnectionContext>>,
    Json((hostname, mut action)): Json<(String, ClientConnectionsAction)>,
) {
    let mut session_manager = SESSION_MANAGER.write();
    if matches!(action, ClientConnectionsAction::RemoveEntry)
        && let Some(entry) = session_manager.client_list().get(&hostname)
        && entry.connection_state != ConnectionState::Disconnected
    {
        ctx.clients_to_be_removed.lock().insert(hostname.clone());

        action = ClientConnectionsAction::SetConnectionState(ConnectionState::Disconnecting);
    }

    session_manager.update_client_connections(hostname, action);
}

async fn insert_idr(State(ctx): State<Arc<ConnectionContext>>) {
    ctx.events_sender.send(ServerCoreEvent::RequestIDR).ok();
}

async fn capture_frame(State(ctx): State<Arc<ConnectionContext>>) {
    ctx.events_sender.send(ServerCoreEvent::CaptureFrame).ok();
}

async fn start_recording(State(ctx): State<Arc<ConnectionContext>>) {
    crate::create_recording_file(&ctx, crate::SESSION_MANAGER.read().settings())
}

async fn stop_recording(State(ctx): State<Arc<ConnectionContext>>) {
    *ctx.video_recording_file.lock() = None;
}

async fn add_firewall_rules() {
    if let Err(e) =
        alvr_server_io::firewall_rules(FirewallRulesAction::Add, FILESYSTEM_LAYOUT.get().unwrap())
    {
        error!("Failed to add firewall rules! code: {e}");
    } else {
        info!("Successfully added firewall rules!");
    }
}

async fn remove_firewall_rules() {
    if let Err(e) = alvr_server_io::firewall_rules(
        FirewallRulesAction::Remove,
        FILESYSTEM_LAYOUT.get().unwrap(),
    ) {
        error!("Failed to remove firewall rules! code: {e}");
    } else {
        info!("Successfully removed firewall rules!");
    }
}

async fn get_driver_list() {
    if let Ok(list) = alvr_server_io::get_registered_drivers() {
        alvr_events::send_event(EventType::DriversList(list));
    }
}

async fn register_alvr_driver() {
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

async fn unregister_driver(Json(path): Json<PathBuf>) {
    alvr_server_io::driver_registration(&[path], false).ok();

    if let Ok(list) = alvr_server_io::get_registered_drivers() {
        alvr_events::send_event(EventType::DriversList(list));
    }
}

async fn restart_steamvr(State(ctx): State<Arc<ConnectionContext>>) {
    ctx.events_sender.send(ServerCoreEvent::RestartPending).ok();
}

async fn shutdown_steamvr(State(ctx): State<Arc<ConnectionContext>>) {
    ctx.events_sender
        .send(ServerCoreEvent::ShutdownPending)
        .ok();
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
