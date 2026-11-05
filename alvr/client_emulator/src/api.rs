//! HTTP control API.
//!
//! Runs on its own thread with a blocking server, and communicates with the UI thread through
//! shared state plus a request channel for anything needing the GPU. Rendering must happen on the
//! thread owning the wgpu device, so capture requests are queued and answered by the UI thread.

use crate::{client::FrameTiming, controllers::Hand, hands::HandPose};
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
    /// How evenly the streamed world has been advancing; see [`FrameTiming`].
    pub frame_timing: FrameTiming,
    /// What the window draws.
    pub source: ViewSource,
    /// The stream's passthrough mode, `None` when the video is opaque.
    pub passthrough: Option<String>,
    /// Frames of the alpha stream shown so far, for the alpha stream passthrough mode.
    pub alpha_frames: u64,
}

/// What the on-screen view draws.
///
/// The stream is the *virtual* source: it is the VR content the server renders. The local glTF
/// scene stands in for what the headset's cameras would see, so it is the *real* one.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ViewSource {
    /// What the headset shows: the video blended over the real one by the stream's passthrough
    /// mode. The same as [`ViewSource::Virtual`] when passthrough is off.
    Mixed,
    /// The video alone, opaque, whatever the passthrough mode.
    Virtual,
    /// The local scene alone.
    Real,
}

/// Body of `POST /api/source`.
#[derive(Deserialize)]
pub struct SourceRequest {
    pub source: ViewSource,
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

/// Body of `POST /api/drive`, and the state it sets.
///
/// A held input rather than a pose: the UI thread merges it into the same [`CameraInput`] the
/// keyboard and mouse produce, integrated against the real frame time, so it drives the camera
/// down exactly the path a person would. Posting individual poses instead makes the caller's own
/// scheduling part of the measurement, which for a jitter investigation is the whole problem —
/// an HTTP client cannot pace itself anywhere near a frame accurately enough to be a reference.
///
/// Omitted fields reset to zero, so `{}` stops the motion.
///
/// [`CameraInput`]: crate::camera::CameraInput
#[derive(Deserialize, Serialize, Clone, Copy, Default)]
#[serde(default)]
pub struct DriveRequest {
    /// Positive is the direction the camera faces, flattened to horizontal. Unit is one full
    /// press of the movement key, not metres per second.
    pub forward: f32,
    /// Positive is right.
    pub right: f32,
    /// Positive moves up.
    pub height: f32,
    /// Turn rate in degrees per second. Positive turns left, the way the yaw angle grows.
    pub yaw_rate: f32,
    /// Pitch rate in degrees per second. Positive looks up.
    pub pitch_rate: f32,
    /// Roll rate in degrees per second.
    pub roll_rate: f32,
    pub fast: bool,
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

/// Snapshot of both emulated hands, published by the UI thread every frame and serialised straight
/// out to `GET /api/hands`.
#[derive(Serialize, Clone, Default)]
pub struct HandsResponse {
    /// The articulations available for selection, in the order the panels show them.
    pub poses: Vec<NamedSummary>,
    /// The timed sequences of those poses.
    pub gestures: Vec<NamedSummary>,
    pub left: HandSnapshot,
    pub right: HandSnapshot,
}

#[derive(Serialize, Clone)]
pub struct NamedSummary {
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Clone)]
pub struct HandSnapshot {
    pub enabled: bool,
    pub visible: bool,
    /// Head-relative palm position: X right, Y up, -Z forward.
    pub position: [f32; 3],
    /// Head-relative palm orientation quaternion, XYZW.
    pub orientation: [f32; 4],
    /// The selected pose, or `null` when the articulation was set directly.
    pub pose: Option<String>,
    /// The gesture playing right now, if any.
    pub gesture: Option<String>,
    /// The articulation currently being sent, which mid-movement is between two poses.
    pub articulation: Articulation,
    /// Whether a pose change or a gesture is still running.
    pub moving: bool,
}

impl Default for HandSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            visible: false,
            position: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            pose: None,
            gesture: None,
            articulation: Articulation::default(),
            moving: false,
        }
    }
}

/// A hand's articulation as the API expresses it: a curl per digit plus the two whole-hand
/// controls. Every field is optional on the way in, keeping its current value when omitted.
#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct Articulation {
    pub thumb: Option<f32>,
    pub index: Option<f32>,
    pub middle: Option<f32>,
    pub ring: Option<f32>,
    pub little: Option<f32>,
    pub spread: Option<f32>,
    pub thumb_opposition: Option<f32>,
}

impl Articulation {
    pub fn from_pose(pose: &HandPose) -> Self {
        Self {
            thumb: Some(pose.curl[0]),
            index: Some(pose.curl[1]),
            middle: Some(pose.curl[2]),
            ring: Some(pose.curl[3]),
            little: Some(pose.curl[4]),
            spread: Some(pose.spread),
            thumb_opposition: Some(pose.thumb_opposition),
        }
    }

    /// Applies the fields that were given on top of an existing articulation.
    pub fn apply(&self, base: HandPose) -> HandPose {
        HandPose {
            curl: [
                self.thumb.unwrap_or(base.curl[0]),
                self.index.unwrap_or(base.curl[1]),
                self.middle.unwrap_or(base.curl[2]),
                self.ring.unwrap_or(base.curl[3]),
                self.little.unwrap_or(base.curl[4]),
            ],
            spread: self.spread.unwrap_or(base.spread),
            thumb_opposition: self.thumb_opposition.unwrap_or(base.thumb_opposition),
        }
    }

    /// Whether the request left every field out, meaning it asks for no articulation change.
    pub fn is_default(&self) -> bool {
        [
            self.thumb,
            self.index,
            self.middle,
            self.ring,
            self.little,
            self.spread,
            self.thumb_opposition,
        ]
        .iter()
        .all(Option::is_none)
    }
}

/// A pending hand change, applied by the UI thread on the next frame.
pub enum HandCommand {
    Configure {
        hand: Hand,
        enabled: Option<bool>,
        visible: Option<bool>,
    },
    SetPose {
        hand: Hand,
        position: Option<Vec3>,
        orientation: Option<Quat>,
    },
    /// Move to a named pose from the settings, or to an articulation given outright. `transition`
    /// overrides the configured time, and zero applies it immediately.
    SetArticulation {
        hand: Hand,
        pose: Option<String>,
        articulation: Articulation,
        transition: Option<Duration>,
    },
    /// Play a timed gesture from the settings, from its beginning.
    PlayGesture {
        hand: Hand,
        gesture: String,
    },
    Reset {
        hand: Hand,
    },
}

/// Body of `POST /api/hands/{hand}`. Omitted fields keep their current value.
#[derive(Deserialize)]
struct HandConfigRequest {
    enabled: Option<bool>,
    visible: Option<bool>,
}

/// Body of `POST /api/hands/{hand}/pose`. Omitted fields keep their current value.
#[derive(Deserialize)]
struct HandPoseRequest {
    /// Head-relative palm position: X right, Y up, -Z forward.
    position: Option<[f32; 3]>,
    /// Head-relative palm orientation quaternion, XYZW. Normalised on apply.
    orientation: Option<[f32; 4]>,
}

/// Body of `POST /api/hands/{hand}/articulation`. Either names a pose or gives the articulation
/// directly; giving both applies the named pose with the given fields overridden.
#[derive(Deserialize)]
struct HandArticulationRequest {
    pose: Option<String>,
    #[serde(flatten)]
    articulation: Articulation,
    /// Transition time in seconds, overriding the configured one. Zero applies immediately.
    transition_seconds: Option<f32>,
}

/// Body of `POST /api/hands/{hand}/gesture`.
#[derive(Deserialize)]
struct HandGestureRequest {
    gesture: String,
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
    /// Latest hand snapshot published by the UI thread, for `/api/hands`.
    pub hands: Mutex<HandsResponse>,
    /// Pose changes queued by `/api/move`.
    pub moves: Mutex<VecDeque<PendingMove>>,
    /// Held camera input set by `/api/drive`, merged into every frame's input until changed.
    pub drive: Mutex<DriveRequest>,
    /// Controller changes queued by the controller endpoints.
    pub controller_commands: Mutex<VecDeque<ControllerCommand>>,
    /// Hand changes queued by the hand endpoints.
    pub hand_commands: Mutex<VecDeque<HandCommand>>,
    /// Capture requests queued by the view endpoints.
    pub captures: Mutex<VecDeque<CaptureRequest>>,
    /// View source change requested by `/api/source`, applied on the next frame.
    pub source: Mutex<Option<ViewSource>>,
}

impl SharedState {
    pub fn new(initial: StateResponse) -> Self {
        Self {
            state: Mutex::new(initial),
            controllers: Mutex::new(ControllersResponse::default()),
            hands: Mutex::new(HandsResponse::default()),
            moves: Mutex::new(VecDeque::new()),
            drive: Mutex::new(DriveRequest::default()),
            controller_commands: Mutex::new(VecDeque::new()),
            hand_commands: Mutex::new(VecDeque::new()),
            captures: Mutex::new(VecDeque::new()),
            source: Mutex::new(None),
        }
    }
}

/// Starts the HTTP server on a background thread.
///
/// Binds to localhost only: this is a debugging interface with no authentication, and it should not
/// be reachable from the network.
pub fn spawn(shared: Arc<SharedState>, port: u16) -> alvr_common::anyhow::Result<()> {
    let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| {
        alvr_common::anyhow::anyhow!("Cannot start HTTP server on port {port}: {e}")
    })?;

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
        (tiny_http::Method::Get, "/api/state") => {
            match serde_json::to_string_pretty(&*shared.state.lock()) {
                Ok(json) => Reply::Json(json),
                Err(e) => Reply::Error(500, format!("Cannot serialise state: {e}")),
            }
        }

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

        (tiny_http::Method::Get, "/api/hands") => {
            match serde_json::to_string_pretty(&*shared.hands.lock()) {
                Ok(json) => Reply::Json(json),
                Err(e) => Reply::Error(500, format!("Cannot serialise hands: {e}")),
            }
        }

        (tiny_http::Method::Post, path) if path.starts_with("/api/hands/") => {
            let mut body = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut body) {
                return Reply::Error(400, format!("Cannot read request body: {e}"));
            }

            let rest = &path["/api/hands/".len()..];
            hand_route(shared, rest, &body)
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

        (tiny_http::Method::Post, "/api/source") => {
            let mut body = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut body) {
                return Reply::Error(400, format!("Cannot read request body: {e}"));
            }

            match serde_json::from_str::<SourceRequest>(&body) {
                Ok(parsed) => {
                    *shared.source.lock() = Some(parsed.source);

                    Reply::Json("{\"ok\":true}".into())
                }
                Err(e) => Reply::Error(400, format!("Invalid JSON: {e}")),
            }
        }

        (tiny_http::Method::Get, "/api/drive") => {
            match serde_json::to_string(&*shared.drive.lock()) {
                Ok(body) => Reply::Json(body),
                Err(e) => Reply::Error(500, format!("Cannot serialise drive state: {e}")),
            }
        }

        (tiny_http::Method::Post, "/api/drive") => {
            let mut body = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut body) {
                return Reply::Error(400, format!("Cannot read request body: {e}"));
            }

            match serde_json::from_str::<DriveRequest>(&body) {
                Ok(parsed) => {
                    *shared.drive.lock() = parsed;

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
        return Reply::Error(
            404,
            format!("No such controller: {side} (use left or right)"),
        );
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

/// Dispatches `POST /api/hands/{hand}[/...]` requests, mirroring the controller routes: validated
/// here against the last published snapshot, applied by the UI thread on its next frame.
fn hand_route(shared: &SharedState, rest: &str, body: &str) -> Reply {
    let (side, action) = match rest.split_once('/') {
        Some((side, action)) => (side, action),
        None => (rest, ""),
    };

    let Some(hand) = Hand::from_side(side) else {
        return Reply::Error(404, format!("No such hand: {side} (use left or right)"));
    };

    let body = if body.trim().is_empty() { "{}" } else { body };

    let command = match action {
        "" => parse_hand_configure(hand, body),
        "pose" => parse_hand_pose(hand, body),
        "articulation" => parse_hand_articulation(shared, hand, body),
        "gesture" => parse_hand_gesture(shared, hand, body),
        "reset" => Ok(HandCommand::Reset { hand }),
        _ => {
            return Reply::Error(404, format!("No such hand endpoint: {action}"));
        }
    };

    match command {
        Ok(command) => {
            shared.hand_commands.lock().push_back(command);
            Reply::Json("{\"ok\":true}".into())
        }
        Err(message) => Reply::Error(400, message),
    }
}

fn parse_hand_configure(hand: Hand, body: &str) -> Result<HandCommand, String> {
    let parsed: HandConfigRequest =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    Ok(HandCommand::Configure {
        hand,
        enabled: parsed.enabled,
        visible: parsed.visible,
    })
}

fn parse_hand_pose(hand: Hand, body: &str) -> Result<HandCommand, String> {
    let parsed: HandPoseRequest =
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

    Ok(HandCommand::SetPose {
        hand,
        position: parsed.position.map(Vec3::from_array),
        orientation,
    })
}

fn parse_hand_articulation(
    shared: &SharedState,
    hand: Hand,
    body: &str,
) -> Result<HandCommand, String> {
    let parsed: HandArticulationRequest =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    if parsed.pose.is_none() && parsed.articulation.is_default() {
        return Err(
            "Give a 'pose' name, or one or more of thumb, index, middle, ring, little, spread \
             and thumb_opposition"
                .into(),
        );
    }

    if let Some(pose) = &parsed.pose {
        known_name(shared, pose, false)?;
    }

    let transition = match parsed.transition_seconds {
        Some(seconds) if !(0.0..=10.0).contains(&seconds) => {
            return Err("Transition must be between 0 and 10 seconds".into());
        }
        Some(seconds) => Some(Duration::from_secs_f32(seconds)),
        None => None,
    };

    Ok(HandCommand::SetArticulation {
        hand,
        pose: parsed.pose,
        articulation: parsed.articulation,
        transition,
    })
}

fn parse_hand_gesture(shared: &SharedState, hand: Hand, body: &str) -> Result<HandCommand, String> {
    let parsed: HandGestureRequest =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    known_name(shared, &parsed.gesture, true)?;

    Ok(HandCommand::PlayGesture {
        hand,
        gesture: parsed.gesture,
    })
}

/// Checks a pose or gesture name against the last published snapshot, so the caller gets a proper
/// error listing what is available rather than the command being dropped on the UI thread.
fn known_name(shared: &SharedState, name: &str, gesture: bool) -> Result<(), String> {
    let snapshot = shared.hands.lock();
    let list = if gesture {
        &snapshot.gestures
    } else {
        &snapshot.poses
    };

    if list
        .iter()
        .any(|known| known.name.eq_ignore_ascii_case(name))
    {
        return Ok(());
    }

    let kind = if gesture { "gesture" } else { "pose" };
    let names = list
        .iter()
        .map(|known| known.name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    Err(format!("Unknown {kind} '{name}'. Available: {names}"))
}

fn parse_configure(
    shared: &SharedState,
    hand: Hand,
    body: &str,
) -> Result<ControllerCommand, String> {
    let parsed: ControllerConfigRequest =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {e}"))?;

    if let Some(profile) = &parsed.profile {
        let known =
            shared.controllers.lock().profiles.iter().any(|summary| {
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
            let header = "Content-Type: application/json"
                .parse::<tiny_http::Header>()
                .unwrap();
            request.respond(tiny_http::Response::from_string(body).with_header(header))
        }
        Reply::Png(bytes) => {
            let header = "Content-Type: image/png"
                .parse::<tiny_http::Header>()
                .unwrap();
            request.respond(tiny_http::Response::from_data(bytes).with_header(header))
        }
        Reply::Error(code, message) => {
            let body = serde_json::json!({ "error": message }).to_string();
            let header = "Content-Type: application/json"
                .parse::<tiny_http::Header>()
                .unwrap();
            request.respond(
                tiny_http::Response::from_string(body)
                    .with_status_code(code)
                    .with_header(header),
            )
        }
    }
}
