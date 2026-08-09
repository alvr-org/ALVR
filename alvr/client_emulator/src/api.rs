//! HTTP control API.
//!
//! Runs on its own thread with a blocking server, and communicates with the UI thread through
//! shared state plus a request channel for anything needing the GPU. Rendering must happen on the
//! thread owning the wgpu device, so capture requests are queued and answered by the UI thread.

use alvr_common::{
    glam::Vec3,
    info,
    parking_lot::{Condvar, Mutex},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

pub const DEFAULT_PORT: u16 = 8080;

/// How long a capture request waits for the UI thread before giving up.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize)]
pub struct StateResponse {
    pub connected: bool,
    pub streaming: bool,
    pub hud_message: String,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub environment_file: String,
    pub environment_loaded: bool,
    pub view_resolution: [u32; 2],
    pub refresh_rate: f32,
    pub codec: Option<String>,
}

/// Body of `POST /api/move`. Every field is optional so a caller can change only what it cares
/// about; omitted fields keep their current value.
#[derive(Deserialize)]
pub struct MoveRequest {
    pub position: Option<[f32; 3]>,
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub roll: Option<f32>,
}

/// A pending pose change, applied by the UI thread on the next frame.
pub struct PendingMove {
    pub position: Option<Vec3>,
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub roll: Option<f32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaptureRequestKind {
    Color,
    Depth,
}

/// A capture request handed to the UI thread, with a slot for the encoded PNG to come back in.
pub struct CaptureRequest {
    pub kind: CaptureRequestKind,
    pub result: Arc<CaptureSlot>,
}

/// One-shot rendezvous for a capture result.
#[derive(Default)]
pub struct CaptureSlot {
    /// `None` while pending. `Some(Err)` if the render failed.
    value: Mutex<Option<Result<Vec<u8>, String>>>,
    ready: Condvar,
}

impl CaptureSlot {
    pub fn fulfill(&self, value: Result<Vec<u8>, String>) {
        *self.value.lock() = Some(value);
        self.ready.notify_all();
    }

    fn wait(&self, timeout: Duration) -> Result<Vec<u8>, String> {
        let deadline = Instant::now() + timeout;
        let mut guard = self.value.lock();

        while guard.is_none() {
            if self.ready.wait_until(&mut guard, deadline).timed_out() && guard.is_none() {
                return Err("Timed out waiting for the renderer".into());
            }
        }

        guard.take().unwrap_or_else(|| Err("No result".into()))
    }
}

/// State shared between the HTTP thread and the UI thread.
pub struct SharedState {
    /// Latest snapshot published by the UI thread, serialised straight out to `/api/state`.
    pub state: Mutex<StateResponse>,
    /// Pose changes queued by `/api/move`.
    pub moves: Mutex<VecDeque<PendingMove>>,
    /// Capture requests queued by the view endpoints.
    pub captures: Mutex<VecDeque<CaptureRequest>>,
}

impl SharedState {
    pub fn new(initial: StateResponse) -> Self {
        Self {
            state: Mutex::new(initial),
            moves: Mutex::new(VecDeque::new()),
            captures: Mutex::new(VecDeque::new()),
        }
    }
}

/// Starts the HTTP server on a background thread.
///
/// Binds to localhost only: this is a debugging interface with no authentication, and it should not
/// be reachable from the network.
pub fn spawn(shared: Arc<SharedState>, port: u16) -> alvr_common::anyhow::Result<()> {
    let server = tiny_http::Server::http(("127.0.0.1", port))
        .map_err(|e| alvr_common::anyhow::anyhow!("Cannot start HTTP server on port {port}: {e}"))?;

    info!("Control API listening on http://127.0.0.1:{port}");

    std::thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let response = route(&shared, &mut request);

            if let Err(e) = respond(request, response) {
                info!("Failed to send HTTP response: {e}");
            }
        }
    });

    Ok(())
}

enum Reply {
    Json(String),
    Png(Vec<u8>),
    Error(u16, String),
}

fn route(shared: &SharedState, request: &mut tiny_http::Request) -> Reply {
    let method = request.method().clone();
    // Strip any query string; none of the endpoints take parameters.
    let url = request.url().split('?').next().unwrap_or("").to_owned();

    match (&method, url.as_str()) {
        (tiny_http::Method::Get, "/api/state") => match serde_json::to_string_pretty(&*shared.state.lock()) {
            Ok(json) => Reply::Json(json),
            Err(e) => Reply::Error(500, format!("Cannot serialise state: {e}")),
        },

        (tiny_http::Method::Get, "/api/view/color") => capture(shared, CaptureRequestKind::Color),
        (tiny_http::Method::Get, "/api/view/depth") => capture(shared, CaptureRequestKind::Depth),

        (tiny_http::Method::Post, "/api/move") => {
            let mut body = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut body) {
                return Reply::Error(400, format!("Cannot read request body: {e}"));
            }

            match serde_json::from_str::<MoveRequest>(&body) {
                Ok(parsed) => {
                    shared.moves.lock().push_back(PendingMove {
                        position: parsed.position.map(Vec3::from_array),
                        yaw: parsed.yaw,
                        pitch: parsed.pitch,
                        roll: parsed.roll,
                    });

                    Reply::Json("{\"ok\":true}".into())
                }
                Err(e) => Reply::Error(400, format!("Invalid JSON: {e}")),
            }
        }

        _ => Reply::Error(404, format!("No such endpoint: {method} {url}")),
    }
}

/// Queues a capture for the UI thread and blocks until it comes back.
fn capture(shared: &SharedState, kind: CaptureRequestKind) -> Reply {
    let slot = Arc::new(CaptureSlot::default());

    shared.captures.lock().push_back(CaptureRequest {
        kind,
        result: Arc::clone(&slot),
    });

    match slot.wait(CAPTURE_TIMEOUT) {
        Ok(png) => Reply::Png(png),
        Err(e) => Reply::Error(503, e),
    }
}

fn respond(request: tiny_http::Request, reply: Reply) -> std::io::Result<()> {
    match reply {
        Reply::Json(body) => {
            let header = "Content-Type: application/json".parse::<tiny_http::Header>().unwrap();
            request.respond(tiny_http::Response::from_string(body).with_header(header))
        }
        Reply::Png(bytes) => {
            let header = "Content-Type: image/png".parse::<tiny_http::Header>().unwrap();
            request.respond(tiny_http::Response::from_data(bytes).with_header(header))
        }
        Reply::Error(code, message) => {
            let body = serde_json::json!({ "error": message }).to_string();
            let header = "Content-Type: application/json".parse::<tiny_http::Header>().unwrap();
            request.respond(
                tiny_http::Response::from_string(body)
                    .with_status_code(code)
                    .with_header(header),
            )
        }
    }
}
