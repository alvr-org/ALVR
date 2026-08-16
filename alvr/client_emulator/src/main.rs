// Hide the console window in release builds, matching alvr_dashboard. Debug builds keep it so log
// output stays visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! ALVR client emulator.
//!
//! A desktop application that connects to an ALVR server as if it were a headset, renders a glTF
//! environment from a first person camera, and exposes an HTTP API so the emulated headset can be
//! inspected and driven programmatically.
//!
//! The window shows either the decoded video stream from the server or a locally rendered glTF
//! scene, selectable from the toolbar. The scene is also what the capture endpoints render.

mod api;
mod camera;
mod client;
mod controller_ui;
mod controllers;
mod decoder;
mod render;
mod scene;
mod video;

use crate::{
    api::{
        CaptureRequestKind, ControllerCommand, ControllerSnapshot, ControllersResponse,
        ProfileSummary, SharedState, StateResponse,
    },
    camera::{Camera, CameraInput, Eye},
    client::{ClientStatus, EmulatedClient, TrackedController, TrackedPose, TrackedState},
    controller_ui::ControllerUi,
    controllers::{ControllerSettings, ControllerState, Hand},
    render::{CaptureKind, ControllerRenderer, SceneRenderer, capture_stereo},
    scene::Scene,
    video::{FrameLayout, VideoRenderer},
};
use alvr_common::{
    DeviceMotion, Pose, error,
    glam::{Mat4, Vec3},
    info,
    parking_lot::Mutex,
};
use alvr_packets::ButtonValue;
use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{self, Color32, Key, PointerButton, RichText, Sense, Ui, ViewportBuilder},
    egui_wgpu,
};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

/// Which eye (or both) the on-screen view shows. Captures are always stereo.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Left,
    Right,
    Stereo,
}

/// Resolution of one eye in the capture endpoints, used when no stream has been negotiated yet.
const FALLBACK_CAPTURE_RESOLUTION: (u32, u32) = (960, 916);

/// What the pointer asked for this frame. Collected inside the panel closure, which cannot borrow
/// the app mutably, and applied afterwards.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LookRequest {
    None,
    /// Toggle the latched look mode; a second click releases it, as does Escape.
    Latch,
    /// Look only while the button is held.
    Hold,
}

struct EmulatorApp {
    /// `Option` so `on_exit` can take the client out and leak it. See `on_exit` for why running the
    /// client core's destructor would hang the process.
    client: Option<EmulatedClient>,
    camera: Camera,
    scene: Option<Arc<Scene>>,
    /// Set when the environment failed to load, so the UI can explain why rather than show nothing.
    scene_error: Option<String>,
    environment_path: PathBuf,
    view_mode: ViewMode,
    /// Show the streamed video rather than the local scene, when a stream is available.
    prefer_video: bool,
    /// Decoded frame count and layout, mirrored out of the video renderer for the toolbar.
    video_frames: u64,
    video_layout: FrameLayout,
    /// Pixel aspect ratio of one eye of the video, mirrored out for the letterboxed display.
    video_eye_aspect: Option<f32>,
    /// Latched look mode: left-click the view to enter, Escape to leave.
    look_latched: bool,
    /// Momentary look mode: active only while the right button is held.
    look_held: bool,
    /// Mirrors the grab state actually sent to the window, so the command is only sent on change.
    cursor_grabbed: bool,
    last_frame: Instant,
    shared: Arc<SharedState>,
    controller_settings: ControllerSettings,
    /// Emulated controller state, mutated by both the UI and the API commands. Left is index 0.
    controllers: [ControllerState; 2],
    controller_ui: ControllerUi,
    /// Pending releases queued by the API's click endpoint.
    timed_releases: Vec<TimedRelease>,
    /// Which profile's model is uploaded to the GPU per hand, to reload only on change.
    loaded_models: [Option<usize>; 2],
}

impl EmulatorApp {
    fn new(context: &CreationContext<'_>, shared: Arc<SharedState>) -> Self {
        let environment_path = scene::default_environment_path()
            .unwrap_or_else(|_| PathBuf::from(scene::ENVIRONMENT_FILE_NAME));

        let mut scene = None;
        let mut scene_error = None;

        match Scene::load(&environment_path) {
            Ok(loaded) => {
                let loaded = Arc::new(loaded);

                // Build the GPU resources on eframe's own device, so no second device or any
                // cross-device texture sharing is involved.
                if let Some(state) = context.wgpu_render_state.as_ref() {
                    // Worth logging: the backend decides what GPU interop is possible later, and
                    // wgpu picks it at runtime rather than at build time.
                    let info = state.adapter.get_info();
                    info!(
                        "Rendering on {:?} via {:?} ({})",
                        info.device_type, info.backend, info.name
                    );

                    let renderer = SceneRenderer::new(
                        &state.device,
                        &state.queue,
                        &loaded,
                        state.target_format,
                    );

                    // Stored in egui's type map so the paint callback can reach it.
                    state
                        .renderer
                        .write()
                        .callback_resources
                        .insert(Arc::new(renderer));
                } else {
                    scene_error = Some("wgpu render state unavailable".into());
                }

                scene = Some(loaded);
            }
            Err(e) => {
                error!("{e}");
                scene_error = Some(format!("{e}"));
            }
        }

        let mut camera = Camera::default();

        // Start in the middle of the scene at eye height rather than at the origin, which may be
        // outside the room entirely.
        if let Some(scene) = &scene {
            let (min, max) = scene.bounds();
            let centre = (min + max) / 2.0;
            camera.position = Vec3::new(centre.x, min.y + 1.6, centre.z);
        }

        let controller_settings = ControllerSettings::load_or_create(
            environment_path.parent().unwrap_or_else(|| Path::new(".")),
        );
        let controllers = [
            ControllerState::new(&controller_settings, Hand::Left),
            ControllerState::new(&controller_settings, Hand::Right),
        ];

        Self {
            client: Some(EmulatedClient::new()),
            camera,
            scene,
            scene_error,
            environment_path,
            view_mode: ViewMode::Stereo,
            prefer_video: true,
            video_frames: 0,
            video_layout: FrameLayout::Single,
            video_eye_aspect: None,
            look_latched: false,
            look_held: false,
            cursor_grabbed: false,
            last_frame: Instant::now(),
            shared,
            controller_settings,
            controllers,
            controller_ui: ControllerUi::new(),
            timed_releases: Vec::new(),
            loaded_models: [None, None],
        }
    }

    /// The client. Only `None` after `on_exit`, by which point nothing else runs.
    fn client(&mut self) -> &mut EmulatedClient {
        self.client.as_mut().expect("client used after shutdown")
    }

    /// Current connection status, or a default snapshot once the client has been shut down.
    fn status(&self) -> ClientStatus {
        self.client
            .as_ref()
            .map(|client| client.status().clone())
            .unwrap_or_default()
    }

    /// Whether the mouse is currently driving the camera, by either route.
    fn look_active(&self) -> bool {
        self.look_latched || self.look_held
    }

    /// Reads keyboard and mouse input and moves the camera.
    fn update_camera(&mut self, ui_context: &egui::Context, delta_seconds: f32) {
        let mut input = CameraInput::default();

        ui_context.input(|state| {
            let held = |key: Key| state.key_down(key);

            if held(Key::W) {
                input.forward += 1.0;
            }
            if held(Key::S) {
                input.forward -= 1.0;
            }
            if held(Key::D) {
                input.right += 1.0;
            }
            if held(Key::A) {
                input.right -= 1.0;
            }
            if held(Key::E) {
                input.roll += 1.0;
            }
            if held(Key::Q) {
                input.roll -= 1.0;
            }
            if held(Key::PageUp) {
                input.height += 1.0;
            }
            if held(Key::PageDown) {
                input.height -= 1.0;
            }

            input.fast = state.modifiers.shift;

            // The momentary mode ends the moment the button comes up, wherever the cursor is.
            if self.look_held && !state.pointer.button_down(PointerButton::Secondary) {
                self.look_held = false;
            }

            if self.look_active() {
                // `motion()` is raw mouse movement, unaffected by the cursor being locked in place
                // or hitting a screen edge. `pointer.delta()` reports cursor travel, which is zero
                // while the cursor is grabbed, so look control would not work at all.
                let motion = state.pointer.motion().unwrap_or_default();
                input.mouse_delta = (motion.x, motion.y);
            }

            // Escape releases the latched mode so the toolbar can be used again. It deliberately
            // does not touch the held mode, which the button release owns.
            if state.key_pressed(Key::Escape) {
                self.look_latched = false;
            }
        });

        self.camera.apply_input(&input, delta_seconds);
    }

    /// Applies pose changes queued by `POST /api/move`.
    fn apply_pending_moves(&mut self) {
        while let Some(pending) = self.shared.moves.lock().pop_front() {
            if let Some(position) = pending.position {
                self.camera.position = position;
            }
            if let Some(yaw) = pending.yaw {
                self.camera.yaw = yaw;
            }
            if let Some(pitch) = pending.pitch {
                self.camera.pitch = pitch;
            }
            if let Some(roll) = pending.roll {
                self.camera.roll = roll;
            }
        }
    }

    /// Applies controller changes queued by the controller API endpoints.
    fn apply_controller_commands(&mut self) {
        while let Some(command) = self.shared.controller_commands.lock().pop_front() {
            match command {
                ControllerCommand::Configure {
                    hand,
                    enabled,
                    profile,
                    visible,
                } => {
                    if let Some(profile) = profile
                        && let Some(index) = self.controller_settings.find_profile(&profile)
                        && self.controllers[hand.index()].profile_index != index
                    {
                        let state = &mut self.controllers[hand.index()];
                        state.profile_index = index;
                        // Held inputs of the previous controller type would linger invisibly on
                        // the wire and stale in the panels.
                        state.inputs.clear();
                    }

                    let state = &mut self.controllers[hand.index()];
                    if let Some(enabled) = enabled {
                        state.enabled = enabled;
                    }
                    if let Some(visible) = visible {
                        state.model_visible = visible;
                    }
                }
                ControllerCommand::SetPose {
                    hand,
                    position,
                    orientation,
                } => {
                    let state = &mut self.controllers[hand.index()];
                    if let Some(position) = position {
                        state.position = position;
                    }
                    if let Some(orientation) = orientation {
                        state.orientation = orientation;
                    }
                }
                ControllerCommand::SetInputs { hand, inputs } => {
                    let state = &mut self.controllers[hand.index()];
                    for (suffix, value) in inputs {
                        state.set_input(suffix, value);
                    }
                }
                ControllerCommand::Click {
                    hand,
                    input,
                    duration,
                } => {
                    let pressed = match controllers::input_is_scalar(input) {
                        Some(true) => ButtonValue::Scalar(1.0),
                        _ => ButtonValue::Binary(true),
                    };

                    self.controllers[hand.index()].set_input(input, pressed);
                    self.timed_releases.push(TimedRelease {
                        hand: hand.index(),
                        suffix: input,
                        at: Instant::now() + duration,
                    });
                }
                ControllerCommand::Reset { hand } => {
                    self.controllers[hand.index()].reset(&self.controller_settings, hand);
                }
            }
        }
    }

    /// Releases inputs whose click duration has elapsed.
    fn apply_timed_releases(&mut self) {
        let now = Instant::now();
        let releases = std::mem::take(&mut self.timed_releases);

        for release in releases {
            if now >= release.at {
                // Resting values are removed by `set_input` whatever the input's type.
                self.controllers[release.hand].set_input(release.suffix, ButtonValue::Binary(false));
            } else {
                self.timed_releases.push(release);
            }
        }
    }

    /// Derives world poses and hands the head and controller state to the client, as one
    /// snapshot so the tracking thread never pairs a fresh head with stale controllers.
    ///
    /// Runs after the UI so inputs made this frame are sent this frame. Both hands' button
    /// entries merge into one set, matching how the server consumes them.
    fn update_controllers(&mut self) {
        let head_position = self.camera.position;
        let head_orientation = self.camera.orientation();

        let mut tracked = [TrackedController::default(); 2];
        let mut desired = HashMap::new();
        let mut profile_id = None;
        let mut input_ids = HashSet::new();

        for hand in Hand::BOTH {
            let index = hand.index();
            let state = &self.controllers[index];

            if !state.enabled {
                continue;
            }

            tracked[index] = TrackedController {
                enabled: true,
                motion: DeviceMotion {
                    pose: Pose {
                        orientation: (head_orientation * state.orientation).normalize(),
                        position: head_position + head_orientation * state.position,
                    },
                    // Zero, like the head: the server extrapolates poses by their velocity, and
                    // predicting the controllers while the head goes unpredicted makes them swim
                    // against the view whenever the camera moves. Unpredicted together, they stay
                    // locked together.
                    linear_velocity: Vec3::ZERO,
                    angular_velocity: Vec3::ZERO,
                },
            };

            if let Some(profile) = self.controller_settings.profiles.get(state.profile_index) {
                for (id, value) in state.effective_entries(profile, hand) {
                    desired.insert(id, value);
                }

                profile_id.get_or_insert(profile.id);
                input_ids.extend(profile.input_id_set());
            }
        }

        let client = self.client();

        client.set_tracking(TrackedState {
            head: TrackedPose {
                position: head_position,
                orientation: head_orientation,
            },
            controllers: tracked,
        });

        client.sync_buttons(&desired);

        if let Some(profile_id) = profile_id {
            client.sync_interaction_profile(profile_id, input_ids);
        }
    }

    /// World-space model matrices of the controllers whose 3D model should be drawn.
    fn controller_model_matrices(&self) -> [Option<Mat4>; 2] {
        let head_orientation = self.camera.orientation();

        std::array::from_fn(|index| {
            let state = &self.controllers[index];

            (state.enabled && state.model_visible && self.loaded_models[index].is_some()).then(
                || {
                    Mat4::from_rotation_translation(
                        (head_orientation * state.orientation).normalize(),
                        self.camera.position + head_orientation * state.position,
                    )
                },
            )
        })
    }

    /// Uploads controller models to the GPU when a visible controller's profile changed.
    ///
    /// Profiles name an optional glTF file; without one, or when loading fails, a procedural
    /// placeholder shows position and orientation instead.
    fn ensure_controller_models(&mut self, frame: &Frame) {
        let Some(state) = frame.wgpu_render_state() else {
            return;
        };

        // Created lazily, like the video renderer, and shared with the paint callback.
        let renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<Mutex<ControllerRenderer>>>()
            .cloned();

        let renderer = match renderer {
            Some(renderer) => renderer,
            None => {
                let created = Arc::new(Mutex::new(ControllerRenderer::new(
                    &state.device,
                    &state.queue,
                    state.target_format,
                )));

                state
                    .renderer
                    .write()
                    .callback_resources
                    .insert(Arc::clone(&created));

                created
            }
        };

        for hand in Hand::BOTH {
            let index = hand.index();
            let controller = &self.controllers[index];

            if !(controller.enabled && controller.model_visible)
                || self.loaded_models[index] == Some(controller.profile_index)
            {
                continue;
            }

            let Some(profile) = self.controller_settings.profiles.get(controller.profile_index)
            else {
                continue;
            };

            let model = profile.models[index]
                .as_ref()
                .and_then(|path| match Scene::load(path) {
                    Ok(scene) => Some(scene),
                    Err(e) => {
                        error!("Cannot load controller model {}: {e}", path.display());
                        None
                    }
                })
                .unwrap_or_else(Scene::placeholder_controller);

            renderer
                .lock()
                .set_model(&state.device, &state.queue, index, &model);

            self.loaded_models[index] = Some(controller.profile_index);
        }
    }

    /// Publishes the current controller state for `GET /api/controllers`.
    fn publish_controllers(&self) {
        let snapshot = |hand: Hand| -> ControllerSnapshot {
            let state = &self.controllers[hand.index()];
            let profile = self.controller_settings.profiles.get(state.profile_index);

            ControllerSnapshot {
                enabled: state.enabled,
                profile: profile.map(|p| p.name.clone()).unwrap_or_default(),
                visible: state.model_visible,
                position: state.position.to_array(),
                orientation: state.orientation.to_array(),
                inputs: state
                    .inputs
                    .iter()
                    .map(|(suffix, value)| {
                        let value = match value {
                            ButtonValue::Binary(pressed) => serde_json::Value::from(*pressed),
                            ButtonValue::Scalar(scalar) => {
                                serde_json::Value::from(f64::from(*scalar))
                            }
                        };

                        ((*suffix).to_owned(), value)
                    })
                    .collect(),
                supported_inputs: profile
                    .map(|p| {
                        p.inputs[hand.index()]
                            .iter()
                            .map(|input| input.suffix.to_owned())
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        };

        *self.shared.controllers.lock() = ControllersResponse {
            profiles: self
                .controller_settings
                .profiles
                .iter()
                .map(|profile| ProfileSummary {
                    name: profile.name.clone(),
                    path: profile.path.clone(),
                })
                .collect(),
            left: snapshot(Hand::Left),
            right: snapshot(Hand::Right),
        };
    }

    /// Uploads the newest decoded video frame, if any, and reports which frame is now on screen.
    ///
    /// Runs on the UI thread because the wgpu queue lives here. Returns the timestamp so the caller
    /// can tell the server which frame was actually presented, and records the frame count and
    /// layout for the toolbar.
    fn upload_decoded_frame(&mut self, frame: &Frame) -> Option<Duration> {
        let decoded = self.client().take_latest_frame();

        let state = frame.wgpu_render_state()?;

        // Created lazily so the video pipeline is only built once a stream exists.
        let renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<Mutex<VideoRenderer>>>()
            .cloned();

        let renderer = match renderer {
            Some(renderer) => renderer,
            None => {
                let created = Arc::new(Mutex::new(VideoRenderer::new(
                    &state.device,
                    state.target_format,
                )));

                state
                    .renderer
                    .write()
                    .callback_resources
                    .insert(Arc::clone(&created));

                created
            }
        };

        let mut renderer = renderer.lock();

        if let Some(decoded) = &decoded {
            renderer.upload(&state.device, &state.queue, decoded);
        }

        self.video_frames = renderer.frames_shown();
        self.video_layout = renderer.layout();
        self.video_eye_aspect = renderer.eye_aspect_ratio();

        renderer.has_frame().then(|| renderer.current_timestamp())
    }

    /// Renders and answers any queued capture requests.
    ///
    /// Runs on the UI thread because that is the thread owning the wgpu device.
    fn service_captures(&mut self, frame: &Frame) {
        let pending = std::mem::take(&mut *self.shared.captures.lock());
        if pending.is_empty() {
            return;
        }

        let Some(state) = frame.wgpu_render_state() else {
            for request in pending {
                request
                    .result
                    .fulfill(Err("wgpu render state unavailable".into()));
            }
            return;
        };

        let renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<SceneRenderer>>()
            .cloned();

        let Some(renderer) = renderer else {
            for request in pending {
                request
                    .result
                    .fulfill(Err("Environment not loaded".into()));
            }
            return;
        };

        // Visible controller models are captured too, so the API sees what the window shows.
        let controller_renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<Mutex<ControllerRenderer>>>()
            .cloned();
        let controller_models = self.controller_model_matrices();

        // Capture at the negotiated stream resolution when streaming, so captures are deterministic
        // and independent of the window size.
        let status = self.status();
        let (eye_width, eye_height) = if status.view_resolution.x > 0 {
            (status.view_resolution.x, status.view_resolution.y)
        } else {
            FALLBACK_CAPTURE_RESOLUTION
        };

        for request in pending {
            let kind = match request.kind {
                CaptureRequestKind::Color => CaptureKind::Color,
                CaptureRequestKind::Depth => CaptureKind::Depth,
            };

            let controller_lock = controller_renderer.as_ref().map(|renderer| renderer.lock());
            let controllers = controller_lock
                .as_ref()
                .map(|renderer| (&**renderer, controller_models));

            let pixels = capture_stereo(
                &state.device,
                &state.queue,
                &renderer,
                controllers,
                &self.camera,
                eye_width,
                eye_height,
                kind,
            );

            request.result.fulfill(encode_png(
                &pixels,
                eye_width * 2,
                eye_height,
                request.kind,
            ));
        }
    }

    /// Publishes the current state for `GET /api/state`.
    fn publish_state(&self) {
        let status = self.status();

        *self.shared.state.lock() = StateResponse {
            // The client core reports a HUD message while not streaming; treat streaming as the
            // authoritative "connected" signal since this iteration has no other handshake state.
            connected: status.streaming,
            streaming: status.streaming,
            hud_message: status.hud_message.clone(),
            position: self.camera.position.to_array(),
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
            roll: self.camera.roll,
            environment_file: self.environment_path.display().to_string(),
            environment_loaded: self.scene.is_some(),
            view_resolution: [status.view_resolution.x, status.view_resolution.y],
            refresh_rate: status.refresh_rate,
            codec: status.codec.map(|codec| format!("{codec:?}")),
        };
    }

    fn draw_toolbar(&mut self, ui: &mut Ui) {
        egui::Panel::top(egui::Id::new("toolbar")).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("View:");
                ui.selectable_value(&mut self.view_mode, ViewMode::Left, "Left");
                ui.selectable_value(&mut self.view_mode, ViewMode::Right, "Right");
                ui.selectable_value(&mut self.view_mode, ViewMode::Stereo, "Stereo");

                ui.separator();

                // Switching to the scene while streaming is useful for telling a decode problem
                // apart from a rendering one.
                ui.label("Source:");
                ui.selectable_value(&mut self.prefer_video, true, "Video");
                ui.selectable_value(&mut self.prefer_video, false, "Scene");

                ui.separator();

                let status = self.status();
                if status.streaming {
                    ui.label(
                        RichText::new(format!(
                            "Streaming  {}x{} @ {:.0} Hz",
                            status.view_resolution.x, status.view_resolution.y, status.refresh_rate
                        ))
                        .color(Color32::LIGHT_GREEN),
                    );
                } else {
                    ui.label(
                        RichText::new("Not connected to ALVR server").color(Color32::LIGHT_RED),
                    );
                }

                ui.separator();

                if self.video_frames > 0 {
                    ui.label(format!(
                        "video {} frames, {:?}",
                        self.video_frames, self.video_layout
                    ));
                    ui.separator();
                }

                ui.label(format!(
                    "x {:.2}  y {:.2}  z {:.2}",
                    self.camera.position.x, self.camera.position.y, self.camera.position.z
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.look_latched {
                        ui.label(RichText::new("Esc to release mouse").weak());
                    } else if self.look_held {
                        ui.label(RichText::new("Release to stop looking").weak());
                    } else {
                        ui.label(
                            RichText::new("Click to look, or hold right button").weak(),
                        );
                    }
                });
            });

            controller_ui::toolbar_row(ui, &mut self.controllers, &self.controller_settings);
        });
    }
}

impl App for EmulatorApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut Frame) {
        let ui_context = ui.ctx().clone();

        let now = Instant::now();
        let delta_seconds = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        self.client().poll_events();
        let haptics = self.client().take_haptics();
        self.controller_ui.apply_haptics(haptics);

        self.apply_pending_moves();
        self.apply_controller_commands();
        self.apply_timed_releases();
        self.update_camera(&ui_context, delta_seconds);
        let camera = self.camera;

        self.ensure_controller_models(frame);
        let displayed_frame = self.upload_decoded_frame(frame);

        self.draw_toolbar(ui);

        // Falls back to the scene until the first frame arrives, so the window is never blank while
        // the stream is negotiating.
        let show_video = self.prefer_video && displayed_frame.is_some();
        // The video displays letterboxed at its own aspect ratio; the scene adapts to the window.
        let video_eye_aspect = show_video.then_some(self.video_eye_aspect).flatten();

        let scene_error = self.scene_error.clone();
        let view_mode = self.view_mode;
        let camera_snapshot = CameraSnapshot {
            position: self.camera.position,
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
            roll: self.camera.roll,
        };
        let controller_models = self.controller_model_matrices();
        let mut look_requested = LookRequest::None;

        let view_rect = egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                if let Some(message) = &scene_error {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Could not load environment\n\n{message}\n\nPlace {} next to the executable.",
                                scene::ENVIRONMENT_FILE_NAME
                            ))
                            .color(Color32::LIGHT_RED),
                        );
                    });
                    return None;
                }

                let available = ui.available_size();
                // Drag sensing as well as click, so the right button press is reported here rather
                // than being treated as a click that only lands on release.
                let response = ui.allocate_response(available, Sense::click_and_drag());

                if response.clicked_by(PointerButton::Primary) {
                    look_requested = LookRequest::Latch;
                } else if response.drag_started_by(PointerButton::Secondary) {
                    // Momentary: gated on the press landing in the view, so a right-click on the
                    // toolbar never starts a look.
                    look_requested = LookRequest::Hold;
                }

                ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                    response.rect,
                    ViewportCallback {
                        camera: camera_snapshot,
                        view_mode,
                        viewport: response.rect,
                        show_video,
                        video_eye_aspect,
                        controller_models,
                    },
                ));

                Some(response.rect)
            })
            .inner;

        match look_requested {
            // A toggle, so clicking again releases the mouse without reaching for Escape.
            LookRequest::Latch => self.look_latched = !self.look_latched,
            LookRequest::Hold => self.look_held = true,
            LookRequest::None => (),
        }

        // Controller overlays float above the view: the projected icons with their movement
        // panels, and the skeuomorphic input panels in the bottom corners. Interaction is
        // suppressed while the mouse drives the camera, so the release click cannot land on a
        // control the hidden cursor happens to be over.
        let interactive = !self.look_active();

        if let Some(view_rect) = view_rect {
            let views = view_sub_rects(view_mode, view_rect, video_eye_aspect);

            self.controller_ui.view_overlays(
                &ui_context,
                &views,
                &camera,
                &mut self.controllers,
                &self.controller_settings,
                interactive,
            );
        }

        self.controller_ui.controller_panels(
            &ui_context,
            &mut self.controllers,
            &self.controller_settings,
            interactive,
        );

        // After the UI so inputs and pose changes made this frame are sent this frame. The head
        // pose is published here too, in the same atomic snapshot as the controllers.
        self.update_controllers();

        // Grab and hide the cursor while looking around, and release it otherwise. Sent only on
        // change, since these are window-manager round trips.
        //
        // `Confined` rather than `Locked`: Windows does not implement pointer locking, and winit
        // silently reports it as unsupported, which would leave the cursor free to wander out of
        // the window mid-drag.
        let look_active = self.look_active();
        if look_active != self.cursor_grabbed {
            self.cursor_grabbed = look_active;

            ui_context.send_viewport_cmd(egui::ViewportCommand::CursorGrab(if look_active {
                egui::CursorGrab::Confined
            } else {
                egui::CursorGrab::None
            }));
            ui_context.send_viewport_cmd(egui::ViewportCommand::CursorVisible(!look_active));
        }

        self.service_captures(frame);
        self.publish_state();
        self.publish_controllers();

        // Paces itself at the stream rate; safe to call every repaint.
        self.client().report_frame(displayed_frame);

        // The scene is continuously interactive, so keep painting.
        ui_context.request_repaint();
    }

    /// Shuts the ALVR connection down on window close.
    ///
    /// The client is taken out of `self` and deliberately leaked when
    /// [`EmulatedClient::shutdown`] reports that dropping it would block. `alvr_client_core` never
    /// shuts down the mdns-sd service daemon it creates for discovery, so that daemon's thread parks
    /// forever and `ClientCoreContext::drop` — which joins the connection thread owning it — never
    /// returns. Leaking is the right trade here: the process is about to exit, so the OS reclaims
    /// everything anyway, whereas running the destructor hangs the window on close.
    ///
    /// eframe drops the app inside this same event-loop callback, so returning from here without
    /// removing the client would hit that destructor immediately.
    fn on_exit(&mut self) {
        if let Some(mut client) = self.client.take() {
            if client.shutdown() {
                drop(client);
            } else {
                std::mem::forget(client);
            }
        }
    }
}

/// Camera values copied into the paint callback, which cannot borrow the app.
#[derive(Clone, Copy)]
struct CameraSnapshot {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    roll: f32,
}

impl CameraSnapshot {
    fn to_camera(self) -> Camera {
        Camera {
            position: self.position,
            yaw: self.yaw,
            pitch: self.pitch,
            roll: self.roll,
        }
    }
}

struct ViewportCallback {
    camera: CameraSnapshot,
    view_mode: ViewMode,
    /// The rect the scene is painted into, in points. Needed at prepare time to derive the aspect
    /// ratio, which the screen size alone would get wrong because the toolbar takes part of the
    /// window.
    viewport: egui::Rect,
    /// Draw the streamed video rather than the local scene.
    show_video: bool,
    /// Pixel aspect ratio of one eye of the video, when the video is shown. The video is displayed
    /// inner-fit at this ratio rather than stretched to the viewport.
    video_eye_aspect: Option<f32>,
    /// World-space model matrices of the controllers to draw. Drawn over the video as well: the
    /// VR application renders its own controllers, but the local models show what the emulator is
    /// actually sending, which is the point of the display toggle.
    controller_models: [Option<Mat4>; 2],
}

impl egui_wgpu::CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if self.show_video
            && let Some(renderer) = resources.get::<Arc<Mutex<VideoRenderer>>>()
        {
            renderer.lock().set_regions(queue);
        }

        let camera = self.camera.to_camera();

        // The callback rect, not the whole screen: the toolbar takes some of the window, and
        // using the screen height would skew the aspect ratio.
        let width = self.viewport.width().max(1.0);
        let height = self.viewport.height().max(1.0);

        // In stereo each eye gets half the width, so its aspect ratio is halved to match.
        let divisor = if self.view_mode == ViewMode::Stereo {
            2.0
        } else {
            1.0
        };
        let aspect_ratio = (width / divisor) / height;

        if !self.show_video
            && let Some(renderer) = resources.get::<Arc<SceneRenderer>>()
        {
            // Both slots are written even in single-eye mode; the draw call picks one.
            renderer.set_view(queue, &camera, Eye::Left, aspect_ratio);
            renderer.set_view(queue, &camera, Eye::Right, aspect_ratio);
        }

        if let Some(renderer) = resources.get::<Arc<Mutex<ControllerRenderer>>>() {
            let renderer = renderer.lock();

            // Over the video the models must use the projection the server rendered with — the
            // advertised FOV — not the window's shape, or they drift against the video content.
            let controller_aspect = if self.show_video {
                Camera::fov_aspect_ratio()
            } else {
                aspect_ratio
            };

            for eye in [Eye::Left, Eye::Right] {
                let view_proj =
                    Camera::projection_matrix(controller_aspect) * camera.view_matrix(eye);

                for (hand, model) in self.controller_models.iter().enumerate() {
                    if let Some(model) = model {
                        renderer.set_view(queue, hand, eye, view_proj * *model);
                    }
                }
            }
        }

        let _ = screen_descriptor;

        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let viewport = info.viewport_in_pixels();
        let left = viewport.left_px as f32;
        let top = viewport.top_px as f32;
        let width = viewport.width_px as f32;
        let height = viewport.height_px as f32;

        // The video and the scene are laid out identically, so the eye splitting is shared and only
        // the draw call differs.
        let video = self
            .show_video
            .then(|| resources.get::<Arc<Mutex<VideoRenderer>>>())
            .flatten();
        let scene = resources.get::<Arc<SceneRenderer>>();
        let controllers = self
            .controller_models
            .iter()
            .any(Option::is_some)
            .then(|| resources.get::<Arc<Mutex<ControllerRenderer>>>())
            .flatten();

        if video.is_none() && scene.is_none() {
            return;
        }

        let draw_eye = |pass: &mut wgpu::RenderPass<'static>, eye: Eye| {
            match &video {
                Some(video) => video.lock().draw(pass, eye),
                None => {
                    if let Some(scene) = scene {
                        scene.draw(pass, eye);
                    }
                }
            }

            // Over the video too: the application draws its own controllers, but the local models
            // show what the emulator is sending, which is what the display toggle is for.
            if let Some(renderer) = &controllers {
                let renderer = renderer.lock();

                for (hand, model) in self.controller_models.iter().enumerate() {
                    if model.is_some() {
                        renderer.draw(pass, hand, eye);
                    }
                }
            }
        };

        // The video keeps its own aspect ratio, displayed inner-fit within the eye's part of the
        // viewport; stretching it also mismatched the controller overlays. The scene has no fixed
        // aspect and fills the viewport.
        let mut draw_view = |pass: &mut wgpu::RenderPass<'static>,
                             eye: Eye,
                             left: f32,
                             top: f32,
                             width: f32,
                             height: f32| {
            let (left, top, width, height) = match self.video_eye_aspect {
                Some(aspect) => fit_viewport(left, top, width, height, aspect),
                None => (left, top, width, height),
            };

            pass.set_viewport(left, top, width, height, 0.0, 1.0);
            draw_eye(pass, eye);
        };

        match self.view_mode {
            ViewMode::Left => draw_view(pass, Eye::Left, left, top, width, height),
            ViewMode::Right => draw_view(pass, Eye::Right, left, top, width, height),
            ViewMode::Stereo => {
                // Side by side, each eye in its own half of the viewport.
                let half = width / 2.0;

                draw_view(pass, Eye::Left, left, top, half, height);
                draw_view(pass, Eye::Right, left + half, top, half, height);
            }
        }

        // egui draws the rest of the UI in this same pass, so the viewport must be restored or
        // everything painted afterwards lands in the last eye's half.
        pass.set_viewport(
            0.0,
            0.0,
            info.screen_size_px[0] as f32,
            info.screen_size_px[1] as f32,
            0.0,
            1.0,
        );
    }
}

/// The inner-fit rectangle of the given aspect ratio, centred in a viewport.
fn fit_viewport(left: f32, top: f32, width: f32, height: f32, aspect: f32) -> (f32, f32, f32, f32) {
    let fitted_width = width.min(height * aspect);
    let fitted_height = fitted_width / aspect;

    (
        left + (width - fitted_width) / 2.0,
        top + (height - fitted_height) / 2.0,
        fitted_width,
        fitted_height,
    )
}

/// The sub-rectangles the view is split into, which eye each shows, and the aspect ratio of the
/// projection that maps world space onto each rectangle.
///
/// With the video shown (`video_eye_aspect` set), each rectangle is the letterboxed video area,
/// and the projection is the advertised FOV's — the one the server rendered the video with. The
/// scene fills the viewport and is projected at the rectangle's own aspect ratio.
fn view_sub_rects(
    view_mode: ViewMode,
    rect: egui::Rect,
    video_eye_aspect: Option<f32>,
) -> Vec<(Eye, egui::Rect, f32)> {
    let outer: Vec<(Eye, egui::Rect)> = match view_mode {
        ViewMode::Left => vec![(Eye::Left, rect)],
        ViewMode::Right => vec![(Eye::Right, rect)],
        ViewMode::Stereo => {
            let half = rect.width() / 2.0;
            vec![
                (Eye::Left, egui::Rect::from_min_size(rect.min, egui::vec2(half, rect.height()))),
                (
                    Eye::Right,
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + half, rect.min.y),
                        egui::vec2(half, rect.height()),
                    ),
                ),
            ]
        }
    };

    outer
        .into_iter()
        .map(|(eye, rect)| match video_eye_aspect {
            Some(aspect) => {
                let (left, top, width, height) =
                    fit_viewport(rect.left(), rect.top(), rect.width(), rect.height(), aspect);

                (
                    eye,
                    egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height)),
                    Camera::fov_aspect_ratio(),
                )
            }
            None => (eye, rect, rect.width() / rect.height().max(1.0)),
        })
        .collect()
}

/// A pending input release queued by the API's click endpoint.
struct TimedRelease {
    hand: usize,
    suffix: &'static str,
    at: Instant,
}

fn encode_png(
    pixels: &[u8],
    width: u32,
    height: u32,
    kind: CaptureRequestKind,
) -> Result<Vec<u8>, String> {
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);

    let color_type = match kind {
        CaptureRequestKind::Color => image::ExtendedColorType::Rgba8,
        CaptureRequestKind::Depth => image::ExtendedColorType::L8,
    };

    image::ImageEncoder::write_image(encoder, pixels, width, height, color_type)
        .map_err(|e| format!("Cannot encode PNG: {e}"))?;

    Ok(png)
}

/// Chooses which wgpu backends to consider, in preference order.
///
/// On Windows the DirectX backend is preferred over Vulkan. Both render the scene equally well, but
/// hardware video decode is what makes the difference: ffmpeg's working Windows decoder produces
/// D3D11 textures, and importing those into a D3D12 device is a shared-handle away, while importing
/// them into a Vulkan device needs external-memory interop. Vulkan Video would avoid the question
/// entirely, but it currently fails on the driver here.
///
/// Elsewhere the default order is kept, which prefers Vulkan on Linux and Metal on macOS.
fn preferred_wgpu_setup() -> egui_wgpu::WgpuSetup {
    // eframe fills in the display handle itself, so the simpler constructor is fine here.
    let mut create_new = egui_wgpu::WgpuSetupCreateNew::without_display_handle();

    // `Backends` is a filter, not a priority order, so preferring a backend means choosing the
    // adapter explicitly. WGPU_BACKEND still takes precedence, since it narrows what is enumerated.
    if cfg!(windows) {
        create_new.native_adapter_selector = Some(Arc::new(|adapters, _surface| {
            // Highest score wins: DirectX first for the video-decode reason above, then discrete
            // over integrated so a laptop does not pick the iGPU.
            let score = |adapter: &wgpu::Adapter| {
                let info = adapter.get_info();

                let backend = match info.backend {
                    wgpu::Backend::Dx12 => 2,
                    wgpu::Backend::Vulkan => 1,
                    _ => 0,
                };
                let device = match info.device_type {
                    wgpu::DeviceType::DiscreteGpu => 2,
                    wgpu::DeviceType::IntegratedGpu => 1,
                    _ => 0,
                };

                // Backend dominates, device type breaks ties.
                backend * 10 + device
            };

            adapters
                .iter()
                .max_by_key(|adapter| score(adapter))
                .cloned()
                .ok_or_else(|| "No suitable wgpu adapter found".to_owned())
        }));
    }

    egui_wgpu::WgpuSetup::CreateNew(create_new)
}

fn main() {
    env_logger::init();

    let port = std::env::var("ALVR_EMULATOR_API_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(api::DEFAULT_PORT);

    let shared = Arc::new(SharedState::new(StateResponse {
        connected: false,
        streaming: false,
        hud_message: String::new(),
        position: [0.0; 3],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        environment_file: String::new(),
        environment_loaded: false,
        view_resolution: [0, 0],
        refresh_rate: 0.0,
        codec: None,
    }));

    if let Err(e) = api::spawn(Arc::clone(&shared), port) {
        error!("{e}");
    }

    info!("Starting ALVR client emulator");

    let result = eframe::run_native(
        "ALVR Client Emulator",
        NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size((1280.0, 720.0)),
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: egui_wgpu::WgpuConfiguration {
                wgpu_setup: preferred_wgpu_setup(),
                ..Default::default()
            },
            // egui needs no depth buffer, so it creates none by default and its render pass has no
            // depth attachment. The scene pipeline requires one, and a pipeline whose depth format
            // does not match the pass fails validation. 32 bits maps to Depth32Float, matching
            // render.rs. Without this the paint callback panics on the first set_pipeline.
            depth_buffer: 32,
            ..Default::default()
        },
        Box::new(move |context| Ok(Box::new(EmulatorApp::new(context, shared)))),
    );

    if let Err(e) = result {
        error!("Emulator exited with error: {e}");
    }
}
