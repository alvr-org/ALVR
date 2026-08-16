//! HTTP control API.
//!
//! Runs on its own thread with a blocking server, and communicates with the UI thread through
//! shared state plus a request channel for anything needing the GPU. Rendering must happen on the
//! thread owning the wgpu device, so capture requests are queued and answered by the UI thread.

use crate::controllers::Hand;
use alvr_common::{
    glam::{Quat, Vec3},
    info,
    parking_lot::{Condvar, Mutex},
};
use alvr_packets::ButtonValue;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

pub const DEFAULT_PORT: u16 = 8080;

/// How long a capture request waits for the UI thread before giving up.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a convenience click holds the input down when the request does not say.
const DEFAULT_CLICK_DURATION: Duration = Duration::from_millis(100);

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

/// Snapshot of both emulated controllers, published by the UI thread every frame and serialised
/// straight out to `GET /api/controllers`. Also what requests are validated against, so errors are
/// reported to the caller instead of being dropped on the UI thread.
#[derive(Serialize, Clone, Default)]
pub struct ControllersResponse {
    /// The profiles available for emulation, in selection order.
    pub profiles: Vec<ProfileSummary>,
    pub left: ControllerSnapshot,
    pub right: ControllerSnapshot,
}

impl ControllersResponse {
    fn hand(&self, hand: Hand) -> &ControllerSnapshot {
        match hand {
            Hand::Left => &self.left,
            Hand::Right => &self.right,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ProfileSummary {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Clone)]
pub struct ControllerSnapshot {
    pub enabled: bool,
    pub profile: String,
    pub visible: bool,
    /// Head-relative position: X right, Y up, -Z forward.
    pub position: [f32; 3],
    /// Head-relative orientation quaternion, XYZW.
    pub orientation: [f32; 4],
    /// Inputs currently held, keyed by input path suffix such as `trigger/value`.
    pub inputs: BTreeMap<String, serde_json::Value>,
    /// Inputs the selected profile supports for this hand.
    pub supported_inputs: Vec<String>,
}

impl Default for ControllerSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: String::new(),
            visible: false,
            position: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            inputs: BTreeMap::new(),
            supported_inputs: Vec::new(),
        }
    }
}

/// A pending controller change, applied by the UI thread on the next frame. User interface input
/// and API input merge by mutating the same state there.
pub enum ControllerCommand {
    Configure {
        hand: Hand,
        enabled: Option<bool>,
        profile: Option<String>,
        visible: Option<bool>,
    },
    SetPose {
        hand: Hand,
        position: Option<Vec3>,
        orientation: Option<Quat>,
    },
    SetInputs {
        hand: Hand,
        inputs: Vec<(&'static str, ButtonValue)>,
    },
    /// Press an input now and release it after `duration`.
    Click {
        hand: Hand,
        input: &'static str,
        duration: Duration,
    },
    Reset {
        hand: Hand,
    },
}

/// Body of `POST /api/controllers/{hand}`. Omitted fields keep their current value.
#[derive(Deserialize)]
struct ControllerConfigRequest {
    enabled: Option<bool>,
    profile: Option<String>,
    visible: Option<bool>,
}

/// Body of `POST /api/controllers/{hand}/pose`. Omitted fields keep their current value.
#[derive(Deserialize)]
struct ControllerPoseRequest {
    /// Head-relative position: X right, Y up, -Z forward.
    position: Option<[f32; 3]>,
    /// Head-relative orientation quaternion, XYZW. Normalised on apply.
    orientation: Option<[f32; 4]>,
}

/// Body of `POST /api/controllers/{hand}/inputs/click`.
#[derive(Deserialize)]
struct ControllerClickRequest {
    input: String,
    /// Hold time in seconds. Defaults to a brief tap.
    duration: Option<f32>,
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
    /// Latest controller snapshot published by the UI thread, for `/api/controllers`.
    pub controllers: Mutex<ControllersResponse>,
    /// Pose changes queued by `/api/move`.
    pub moves: Mutex<VecDeque<PendingMove>>,
    /// Controller changes queued by the controller endpoints.
    pub controller_commands: Mutex<VecDeque<ControllerCommand>>,
    /// Capture requests queued by the view endpoints.
    pub captures: Mutex<VecDeque<CaptureRequest>>,
}

impl SharedState {
    pub fn new(initial: StateResponse) -> Self {
        Self {
            state: Mutex::new(initial),
            controllers: Mutex::new(ControllersResponse::default()),
            moves: Mutex::new(VecDeque::new()),
            controller_commands: Mutex::new(VecDeque::new()),
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

        (tiny_http::Method::Get, "/api/controllers") => {
            match serde_json::to_string_pretty(&*shared.controllers.lock()) {
                Ok(json) => Reply::Json(json),
                Err(e) => Reply::Error(500, format!("Cannot serialise controllers: {e}")),
            }
        }

        (tiny_http::Method::Post, path) if path.starts_with("/api/controllers/") => {
            let mut body = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut body) {
                return Reply::Error(400, format!("Cannot read request body: {e}"));
            }

            let rest = &path["/api/controllers/".len()..];
            controller_route(shared, rest, &body)
        }

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

/// Dispatches `POST /api/controllers/{hand}[/...]` requests.
///
/// Requests are validated here, against the snapshot the UI thread published last frame, so the
/// caller gets a proper error instead of the command being dropped silently. The commands
/// themselves are applied by the UI thread on its next frame.
fn controller_route(shared: &SharedState, rest: &str, body: &str) -> Reply {
    let (side, action) = match rest.split_once('/') {
        Some((side, action)) => (side, action),
        None => (rest, ""),
    };

    let Some(hand) = Hand::from_side(side) else {
        return Reply::Error(404, format!("No such controller: {side} (use left or right)"));
    };

    // An empty body means "change nothing", which keeps `curl -X POST` without a payload usable.
    let body = if body.trim().is_empty() { "{}" } else { body };

    let command = match action {
        "" => parse_configure(shared, hand, body),
        "pose" => parse_pose(hand, body),
        "inputs" => parse_inputs(shared, hand, body),
        "inputs/click" => parse_click(shared, hand, body),
        "reset" => Ok(ControllerCommand::Reset { hand }),
        _ => {
            return Reply::Error(404, format!("No such controller endpoint: {action}"));
        }
    };

    match command {
        Ok(command) => {
            shared.controller_commands.lock().push_back(command);
            Reply::Json("{\"ok\":true}".into())
        }
        Err(message) => Reply::Error(400, message),
    }
}

fn parse_configure(shared: &SharedState, hand: Hand, body: &str) -> Result<ControllerCommand, String> {
    let parsed: ControllerConfigRequest =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    if let Some(profile) = &parsed.profile {
        let known = shared.controllers.lock().profiles.iter().any(|summary| {
            summary.name.eq_ignore_ascii_case(profile) || summary.path == *profile
        });

        if !known {
            let names = shared
                .controllers
                .lock()
                .profiles
                .iter()
                .map(|summary| summary.name.clone())
                .collect::<Vec<_>>()
                .join(", ");

            return Err(format!("Unknown profile '{profile}'. Available: {names}"));
        }
    }

    Ok(ControllerCommand::Configure {
        hand,
        enabled: parsed.enabled,
        profile: parsed.profile,
        visible: parsed.visible,
    })
}

fn parse_pose(hand: Hand, body: &str) -> Result<ControllerCommand, String> {
    let parsed: ControllerPoseRequest =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    let orientation = match parsed.orientation {
        Some(values) => {
            let quat = Quat::from_array(values);
            if quat.length_squared() < f32::EPSILON {
                return Err("Orientation quaternion must not be zero".into());
            }
            Some(quat.normalize())
        }
        None => None,
    };

    Ok(ControllerCommand::SetPose {
        hand,
        position: parsed.position.map(Vec3::from_array),
        orientation,
    })
}

fn parse_inputs(shared: &SharedState, hand: Hand, body: &str) -> Result<ControllerCommand, String> {
    let parsed: HashMap<String, serde_json::Value> =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    let inputs = parsed
        .iter()
        .map(|(suffix, value)| {
            let suffix = validate_input(shared, hand, suffix)?;

            let value = match value {
                serde_json::Value::Bool(pressed) => ButtonValue::Binary(*pressed),
                serde_json::Value::Number(number) => {
                    ButtonValue::Scalar(number.as_f64().unwrap_or(0.0) as f32)
                }
                other => {
                    return Err(format!(
                        "Input '{suffix}' must be a boolean or a number, got: {other}"
                    ));
                }
            };

            Ok((suffix, value))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(ControllerCommand::SetInputs { hand, inputs })
}

fn parse_click(shared: &SharedState, hand: Hand, body: &str) -> Result<ControllerCommand, String> {
    let parsed: ControllerClickRequest =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    let input = validate_input(shared, hand, &parsed.input)?;

    let duration = match parsed.duration {
        Some(seconds) if !(0.0..=10.0).contains(&seconds) => {
            return Err("Click duration must be between 0 and 10 seconds".into());
        }
        Some(seconds) => Duration::from_secs_f32(seconds),
        None => DEFAULT_CLICK_DURATION,
    };

    Ok(ControllerCommand::Click {
        hand,
        input,
        duration,
    })
}

/// Checks an input path suffix against what the hand's current profile supports, and interns it.
fn validate_input(shared: &SharedState, hand: Hand, suffix: &str) -> Result<&'static str, String> {
    let snapshot = shared.controllers.lock();
    let supported = &snapshot.hand(hand).supported_inputs;

    if !supported.iter().any(|known| known == suffix) {
        return Err(format!(
            "Input '{suffix}' is not available on the current profile. Available: {}",
            supported.join(", ")
        ));
    }

    // The suffix passed profile validation, so it is one of the known canonical strings.
    crate::controllers::INPUT_SUFFIXES
        .iter()
        .copied()
        .find(|known| *known == suffix)
        .ok_or_else(|| format!("Input '{suffix}' is not an ALVR input"))
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
