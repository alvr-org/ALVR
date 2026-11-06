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
mod hand_ui;
mod hands;
mod icons;
mod interaction;
mod overlay;
mod paths;
mod render;
mod scene;
mod server_settings;
mod skinned;
mod video;

use crate::{
    api::{
        Articulation, CaptureRequestKind, ControllerCommand, ControllerSnapshot,
        ControllersResponse, HandCommand, HandSnapshot, HandsResponse, NamedSummary,
        ProfileSummary, SharedState, StateResponse, ViewSource,
    },
    camera::{Camera, CameraInput, Eye},
    client::{
        ClientStatus, EmulatedClient, FrameTiming, TrackedController, TrackedHand, TrackedPose,
        TrackedState,
    },
    controller_ui::ControllerUi,
    controllers::{ControllerInteraction, ControllerSettings, ControllerState, Hand},
    hands::{HandSettings, HandState},
    icons::Icon,
    interaction::HandInteraction,
    overlay::{PoseOverlay, PoseTarget, Slot},
    render::{
        CaptureKind, ControllerRenderer, DeviceModels, GIZMO_SLOTS, GizmoDevice, GizmoRenderer,
        HandRenderer, HandTint, SceneRenderer, capture_stereo,
    },
    scene::Scene,
    skinned::SkinnedModel,
    video::{FrameLayout, VideoRenderer},
};
use alvr_common::{
    Pose, error,
    glam::{Mat4, Quat, Vec3},
    info,
    parking_lot::Mutex,
};
use alvr_packets::ButtonValue;
use alvr_session::PassthroughMode;
use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{self, Color32, Key, PointerButton, RichText, Sense, Ui, ViewportBuilder},
    egui_wgpu,
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
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

/// How much larger than egui's default the toolbar is drawn, text and controls alike.
const TOOLBAR_SCALE: f32 = 1.5;

/// Margin around the contents of the title bar, the toolbar and the status bar, in points.
const BAR_MARGIN: f32 = 8.0 * TOOLBAR_SCALE;

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
    client: EmulatedClient,
    camera: Camera,
    scene: Option<Arc<Scene>>,
    /// Set when the environment failed to load, so the UI can explain why rather than show nothing.
    scene_error: Option<String>,
    environment_path: PathBuf,
    view_mode: ViewMode,
    /// What to draw. The video sources fall back to the scene until a stream is available.
    source: ViewSource,
    /// Show the frame pacing overlay over the view.
    show_stats: bool,
    /// Decoded frame count and layout, mirrored out of the video renderer for the stats overlay.
    video_frames: u64,
    /// Alpha stream frames uploaded, for the stats overlay.
    alpha_frames: u64,
    /// How much older the alpha on screen is than the color frame it is shown with.
    alpha_lag: Option<Duration>,
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
    hand_settings: HandSettings,
    /// Emulated hand state, mutated by both the UI and the API commands. Left is index 0.
    hands: [HandState; 2],
    /// The icons and 6DoF panels of the controllers and the hands, which share one implementation.
    pose_overlay: PoseOverlay,
    /// Pending releases queued by the API's click endpoint.
    timed_releases: Vec<TimedRelease>,
    /// Which profile's model is uploaded to the GPU per hand, to reload only on change.
    loaded_models: [Option<usize>; 2],
    /// The loaded hand models, kept on the CPU because posing them needs their inverse bind
    /// matrices every frame. Left is index 0.
    hand_models: [Option<SkinnedModel>; 2],
    /// Whether loading a hand model has been attempted, so a missing file is reported once rather
    /// than every frame.
    hand_models_tried: [bool; 2],
    /// Which interaction pose to draw axes at on every enabled hand, if any. One selection for
    /// both hands: the poses are mirror images, so seeing them side by side is the comparison.
    hand_gizmo: Option<HandInteraction>,
    /// Which pose to draw axes at on every enabled controller, if any; independent of the hands.
    controller_gizmo: Option<ControllerInteraction>,
}

impl EmulatorApp {
    fn new(context: &CreationContext<'_>, shared: Arc<SharedState>) -> Self {
        icons::install_theme(&context.egui_ctx);

        let environment_path = scene::default_environment_path();

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

        // Settings live in the per-user folder; relative model paths in them name the models the
        // build bundles next to the executable.
        let settings_dir = paths::user_dir();
        let models_dir = paths::executable_dir();
        paths::adopt_settings_file(controllers::SETTINGS_FILE_NAME);
        paths::adopt_settings_file(hands::SETTINGS_FILE_NAME);

        let controller_settings = ControllerSettings::load_or_create(&settings_dir, &models_dir);
        let controllers = [
            ControllerState::new(&controller_settings, Hand::Left),
            ControllerState::new(&controller_settings, Hand::Right),
        ];

        let hand_settings = HandSettings::load_or_create(&settings_dir, &models_dir);
        let hands = [
            HandState::new(&hand_settings, Hand::Left),
            HandState::new(&hand_settings, Hand::Right),
        ];

        Self {
            client: EmulatedClient::new(),
            camera,
            scene,
            scene_error,
            environment_path,
            view_mode: ViewMode::Left,
            source: ViewSource::Mixed,
            show_stats: false,
            video_frames: 0,
            alpha_frames: 0,
            alpha_lag: None,
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
            hand_settings,
            hands,
            pose_overlay: PoseOverlay::new(),
            timed_releases: Vec::new(),
            loaded_models: [None, None],
            hand_models: [None, None],
            hand_gizmo: None,
            controller_gizmo: None,
            hand_models_tried: [false; 2],
        }
    }

    /// Current connection status.
    fn status(&self) -> ClientStatus {
        self.client.status().clone()
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
            if held(Key::R) || held(Key::PageUp) {
                input.height += 1.0;
            }
            if held(Key::F) || held(Key::PageDown) {
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

        // Held API input on top of the keyboard and mouse, so a scripted run drives the camera
        // down the same path a person would; see [`DriveRequest`].
        let drive = *self.shared.drive.lock();
        input.forward += drive.forward;
        input.right += drive.right;
        input.height += drive.height;
        input.roll_rate += drive.roll_rate.to_radians();
        input.yaw_rate += drive.yaw_rate.to_radians();
        input.pitch_rate += drive.pitch_rate.to_radians();
        input.fast |= drive.fast;

        self.camera.apply_input(&input, delta_seconds);
    }

    /// Applies pose changes queued by `POST /api/move`.
    /// Applies a view source change requested through the API.
    fn apply_pending_source(&mut self) {
        if let Some(source) = self.shared.source.lock().take() {
            self.source = source;
        }
    }

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

                        // The same exclusion the toolbar toggles apply: a side is a controller or
                        // a hand, never both.
                        if enabled {
                            self.hands[hand.index()].enabled = false;
                        }
                    }

                    let state = &mut self.controllers[hand.index()];
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

    /// Applies hand changes queued by the hand API endpoints.
    fn apply_hand_commands(&mut self) {
        while let Some(command) = self.shared.hand_commands.lock().pop_front() {
            match command {
                HandCommand::Configure {
                    hand,
                    enabled,
                    visible,
                } => {
                    if let Some(enabled) = enabled {
                        self.hands[hand.index()].enabled = enabled;

                        if enabled {
                            self.controllers[hand.index()].enabled = false;
                        }
                    }
                    if let Some(visible) = visible {
                        self.hands[hand.index()].model_visible = visible;
                    }
                }
                HandCommand::SetPose {
                    hand,
                    position,
                    orientation,
                } => {
                    let state = &mut self.hands[hand.index()];
                    if let Some(position) = position {
                        state.position = position;
                    }
                    if let Some(orientation) = orientation {
                        state.orientation = orientation;
                    }
                }
                HandCommand::SetArticulation {
                    hand,
                    pose,
                    articulation,
                    transition,
                } => {
                    let settings = &self.hand_settings;
                    let index = pose.as_deref().and_then(|name| settings.find_pose(name));
                    let state = &mut self.hands[hand.index()];

                    match index {
                        // A plain pose selection behaves exactly as clicking its button does,
                        // including keeping the pose named in the snapshot.
                        Some(index) if articulation.is_default() => {
                            state.select_pose(settings, index, transition);
                        }
                        // Loose articulation fields are applied on top of the named pose, so a
                        // caller can ask for "Pinch, but with the little finger out" without
                        // spelling out a whole articulation. Without a pose they modify what is
                        // held.
                        _ => {
                            let base = match index {
                                Some(index) => settings.pose_at(index),
                                None => state.pose,
                            };
                            let duration = transition
                                .or_else(|| {
                                    index.and_then(|index| settings.poses[index].transition)
                                })
                                .unwrap_or(settings.transition);

                            state.set_articulation(articulation.apply(base), duration);
                        }
                    }
                }
                HandCommand::PlayGesture { hand, gesture } => {
                    if let Some(index) = self.hand_settings.find_gesture(&gesture) {
                        self.hands[hand.index()].play_gesture(&self.hand_settings, index);
                    }
                }
                HandCommand::Reset { hand } => {
                    self.hands[hand.index()].reset(&self.hand_settings, hand);
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
                self.controllers[release.hand]
                    .set_input(release.suffix, ButtonValue::Binary(false));
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
        let mut tracked_hands = [TrackedHand::default(); 2];
        let mut desired = HashMap::new();
        let mut profile_id = None;
        let mut input_ids = HashSet::new();

        for hand in Hand::BOTH {
            let index = hand.index();
            let state = &self.hands[index];

            if !state.enabled {
                continue;
            }

            // Head-relative, like the controllers: the palm pose places the whole skeleton, and
            // the tracking thread composes it with the head pose going out in the same packet.
            let palm = Pose {
                orientation: state.orientation.normalize(),
                position: state.position,
            };
            let local = state.local_skeleton(&self.hand_settings, hand);

            tracked_hands[index] = TrackedHand {
                enabled: true,
                joints: std::array::from_fn(|joint| palm * local[joint]),
            };
        }

        for hand in Hand::BOTH {
            let index = hand.index();
            let state = &self.controllers[index];

            if !state.enabled {
                continue;
            }

            let profile = self.controller_settings.profiles.get(state.profile_index);

            // Head-relative: the tracking thread composes this with the head pose it sends in the
            // same packet, which is what keeps the controllers rigid against the view. Not the aim
            // the state holds, but the corrected grip, because that is what a real client sends.
            tracked[index] = TrackedController {
                enabled: true,
                pose: state.sent_pose(profile, hand),
            };

            if let Some(profile) = profile {
                for (id, value) in state.effective_entries(profile, hand) {
                    desired.insert(id, value);
                }

                profile_id.get_or_insert(profile.id);
                input_ids.extend(profile.input_id_set());
            }
        }

        let client = &mut self.client;

        client.set_tracking(TrackedState {
            head: TrackedPose {
                position: head_position,
                orientation: head_orientation,
            },
            controllers: tracked,
            hands: tracked_hands,
        });

        client.sync_buttons(&desired);

        if let Some(profile_id) = profile_id {
            client.sync_interaction_profile(profile_id, input_ids);
        }
    }

    /// World-space model matrices of the controllers whose 3D model should be drawn, composed
    /// with the given head pose (the live camera, or the displayed video frame's pose).
    ///
    /// At the grip, which is the frame controller models are authored around: OpenXR's and WebXR's
    /// own, and what `tools/convert_rendermodel.py` and `tools/fetch_controller_model.py` produce.
    fn controller_model_matrices(
        &self,
        head_position: Vec3,
        head_orientation: Quat,
    ) -> [Option<Mat4>; 2] {
        std::array::from_fn(|index| {
            let state = &self.controllers[index];

            (state.enabled && state.model_visible && self.loaded_models[index].is_some()).then(
                || {
                    let profile = self.controller_settings.profiles.get(state.profile_index);
                    let grip = state.grip_pose(profile, Hand::BOTH[index]);

                    Mat4::from_rotation_translation(
                        (head_orientation * grip.orientation).normalize(),
                        head_position + head_orientation * grip.position,
                    )
                },
            )
        })
    }

    /// Creates the gizmo renderer the first time a pose is selected for display.
    ///
    /// Lazy like the other renderers, and for the same reason: it costs a pipeline, and most runs
    /// never draw a gizmo at all.
    fn ensure_gizmo_renderer(&mut self, frame: &Frame) {
        if self.hand_gizmo.is_none() && self.controller_gizmo.is_none() {
            return;
        }

        let Some(state) = frame.wgpu_render_state() else {
            return;
        };

        if state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<GizmoRenderer>>()
            .is_some()
        {
            return;
        }

        let created = Arc::new(GizmoRenderer::new(
            &state.device,
            &state.queue,
            state.target_format,
        ));

        state
            .renderer
            .write()
            .callback_resources
            .insert(Arc::clone(&created));
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

            let Some(profile) = self
                .controller_settings
                .profiles
                .get(controller.profile_index)
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
                // The placeholder is built in the aim's frame, so it is moved into the grip frame it
                // is drawn in by the inverse offset: its nose then shows the aim.
                .unwrap_or_else(|| {
                    Scene::placeholder_controller(profile.grip_from_aim[index])
                        .transformed(profile.grip_from_aim[index].inverse())
                });

            renderer
                .lock()
                .set_model(&state.device, &state.queue, index, &model);

            self.loaded_models[index] = Some(controller.profile_index);
        }
    }

    /// World-space skinning matrices of the hands whose model should be drawn, composed with the
    /// given head pose (the live camera, or the displayed video frame's pose).
    ///
    /// The matrices are built from the very joints being sent on the wire, so the hand that is
    /// drawn is the hand the server is being told about, which is the whole point of the display
    /// toggle.
    fn hand_joint_matrices(
        &self,
        head_position: Vec3,
        head_orientation: Quat,
    ) -> [Option<Vec<Mat4>>; 2] {
        std::array::from_fn(|index| {
            let state = &self.hands[index];
            let model = self.hand_models[index].as_ref()?;

            if !(state.enabled && state.model_visible) {
                return None;
            }

            let hand = Hand::BOTH[index];
            let local = state.local_skeleton(&self.hand_settings, hand);
            let palm = Mat4::from_rotation_translation(
                (head_orientation * state.orientation).normalize(),
                head_position + head_orientation * state.position,
            );

            Some(
                model
                    .joint_matrices(&local, self.hand_settings.hand_length)
                    .into_iter()
                    .map(|joint| palm * joint)
                    .collect(),
            )
        })
    }

    /// World transforms of the debug pose gizmos, one slot per device; see [`GizmoRenderer::slot`].
    ///
    /// Placed against the same head pose the models are, so a gizmo sits on the hand it belongs to
    /// rather than drifting against it over the video. Drawn for any *enabled* device, whether or
    /// not its model is shown: turning the hand off to see the pose inside it is the point.
    fn gizmo_transforms(
        &self,
        head_position: Vec3,
        head_orientation: Quat,
    ) -> [Option<Mat4>; GIZMO_SLOTS] {
        let mut transforms = [None; GIZMO_SLOTS];

        if let Some(interaction) = self.hand_gizmo {
            for hand in Hand::BOTH {
                let index = hand.index();
                let state = &self.hands[index];

                if !state.enabled {
                    continue;
                }

                let joints = state.local_skeleton(&self.hand_settings, hand);
                let pose = interaction.transform(&joints, hand);

                let palm = Mat4::from_rotation_translation(
                    (head_orientation * state.orientation).normalize(),
                    head_position + head_orientation * state.position,
                );

                transforms[GizmoRenderer::slot(GizmoDevice::Hand, index)] =
                    Some(palm * Mat4::from_rotation_translation(pose.orientation, pose.position));
            }
        }

        if let Some(interaction) = self.controller_gizmo {
            for hand in Hand::BOTH {
                let index = hand.index();
                let state = &self.controllers[index];

                if !state.enabled {
                    continue;
                }

                let profile = self.controller_settings.profiles.get(state.profile_index);
                let pose = interaction.pose(state, profile, hand);

                transforms[GizmoRenderer::slot(GizmoDevice::Controller, index)] =
                    Some(Mat4::from_rotation_translation(
                        (head_orientation * pose.orientation).normalize(),
                        head_position + head_orientation * pose.position,
                    ));
            }
        }

        transforms
    }

    /// Loads the hand models and uploads them to the GPU, once, when a hand is first shown.
    ///
    /// Unlike the controllers there is nothing to reload on: the models come from the settings
    /// file and do not change while the emulator runs. A file that fails to load is reported once
    /// and the hand simply is not drawn; the joints still go on the wire, so a missing model
    /// costs the picture, not the emulation.
    fn ensure_hand_models(&mut self, frame: &Frame) {
        if !Hand::BOTH.iter().any(|hand| {
            let state = &self.hands[hand.index()];

            state.enabled && state.model_visible && !self.hand_models_tried[hand.index()]
        }) {
            return;
        }

        let Some(state) = frame.wgpu_render_state() else {
            return;
        };

        let renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<Mutex<HandRenderer>>>()
            .cloned();

        let renderer = match renderer {
            Some(renderer) => renderer,
            None => {
                let created = Arc::new(Mutex::new(HandRenderer::new(
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

            if !(self.hands[index].enabled && self.hands[index].model_visible)
                || self.hand_models_tried[index]
            {
                continue;
            }

            self.hand_models_tried[index] = true;

            let Some(path) = self.hand_settings.models[index].as_ref() else {
                info!("No hand model configured for the {} hand", hand.side());
                continue;
            };

            match SkinnedModel::load(
                path,
                hand,
                self.hand_settings.hand_length,
                &self.hand_settings.joint_names,
            ) {
                Ok(model) => {
                    renderer.lock().set_model(
                        &state.device,
                        &state.queue,
                        index,
                        &model,
                        HandTint {
                            skin: self.hand_settings.model_color,
                            glove: self.hand_settings.glove_color,
                        },
                    );

                    self.hand_models[index] = Some(model);
                }
                Err(e) => error!("Cannot load hand model {}: {e:#}", path.display()),
            }
        }
    }

    /// Publishes the current hand state for `GET /api/hands`.
    fn publish_hands(&self) {
        let snapshot = |hand: Hand| -> HandSnapshot {
            let state = &self.hands[hand.index()];

            HandSnapshot {
                enabled: state.enabled,
                visible: state.model_visible,
                position: state.position.to_array(),
                orientation: state.orientation.to_array(),
                pose: state
                    .pose_index
                    .and_then(|index| self.hand_settings.poses.get(index))
                    .map(|pose| pose.name.clone()),
                gesture: state
                    .playing_gesture()
                    .and_then(|index| self.hand_settings.gestures.get(index))
                    .map(|gesture| gesture.name.clone()),
                articulation: Articulation::from_pose(&state.pose),
                moving: state
                    .progress(&self.hand_settings, Instant::now())
                    .is_some(),
            }
        };

        let summarise = |name: &String, description: &String| NamedSummary {
            name: name.clone(),
            description: description.clone(),
        };

        *self.shared.hands.lock() = HandsResponse {
            poses: self
                .hand_settings
                .poses
                .iter()
                .map(|pose| summarise(&pose.name, &pose.description))
                .collect(),
            gestures: self
                .hand_settings
                .gestures
                .iter()
                .map(|gesture| summarise(&gesture.name, &gesture.description))
                .collect(),
            left: snapshot(Hand::Left),
            right: snapshot(Hand::Right),
        };
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
        let decoded = self.client.take_latest_frame();

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

        let mut renderer = renderer.lock();

        if let Some(decoded) = &decoded {
            renderer.upload(&state.device, &state.queue, decoded);
        }

        // Matched against the color frame on screen rather than only the one just uploaded: the
        // alpha for a frame can finish decoding a repaint later, and must still be shown with it.
        if self.client.status().streaming {
            if renderer.has_frame()
                && let Some(alpha) = self.client.take_alpha_frame(renderer.current_timestamp())
            {
                renderer.upload_alpha(&state.device, &state.queue, &alpha);
                self.alpha_frames += 1;
            }
        } else {
            renderer.clear_alpha();
            self.alpha_frames = 0;
        }

        renderer.set_passthrough(self.passthrough().as_ref());

        // Zero when every color frame finds its own alpha; a steady positive value means the alpha
        // is consistently late and the mask trails the image.
        self.alpha_lag = renderer
            .alpha_timestamp()
            .map(|alpha| renderer.current_timestamp().saturating_sub(alpha));

        self.video_frames = renderer.frames_shown();
        self.video_layout = renderer.layout();
        self.video_eye_aspect = renderer.eye_aspect_ratio();

        renderer.has_frame().then(|| renderer.current_timestamp())
    }

    /// The passthrough mode the video is drawn with: the stream's in the mixed view, none otherwise.
    fn passthrough(&self) -> Option<PassthroughMode> {
        match self.source {
            ViewSource::Mixed => self.client.status().passthrough.clone(),
            ViewSource::Virtual | ViewSource::Real => None,
        }
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
                request.result.fulfill(Err("Environment not loaded".into()));
            }
            return;
        };

        // Visible controller and hand models are captured too, so the API sees what the window
        // shows.
        let controller_renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<Mutex<ControllerRenderer>>>()
            .cloned();
        let hand_renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<Mutex<HandRenderer>>>()
            .cloned();
        let gizmo_renderer = state
            .renderer
            .read()
            .callback_resources
            .get::<Arc<GizmoRenderer>>()
            .cloned();
        // Captures render the scene live, so the live camera is the right head pose here.
        let controller_models =
            self.controller_model_matrices(self.camera.position, self.camera.orientation());
        let hand_poses = self.hand_joint_matrices(self.camera.position, self.camera.orientation());
        let gizmos = self.gizmo_transforms(self.camera.position, self.camera.orientation());

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
            let hand_lock = hand_renderer.as_ref().map(|renderer| renderer.lock());

            let devices = DeviceModels {
                controllers: controller_lock
                    .as_ref()
                    .map(|renderer| (&**renderer, controller_models)),
                hands: hand_lock.as_ref().map(|renderer| {
                    (
                        &**renderer,
                        std::array::from_fn(|hand| hand_poses[hand].as_deref()),
                    )
                }),
                gizmos: gizmo_renderer
                    .as_ref()
                    .map(|renderer| (&**renderer, gizmos)),
            };

            let pixels = capture_stereo(
                &state.device,
                &state.queue,
                &renderer,
                &devices,
                &self.camera,
                eye_width,
                eye_height,
                kind,
            );

            request
                .result
                .fulfill(encode_png(&pixels, eye_width * 2, eye_height, request.kind));
        }
    }

    /// Publishes the current state for `GET /api/state`.
    fn publish_state(&mut self) {
        let status = self.status();
        let timing = self.client.frame_timing();

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
            frame_timing: timing,
            source: self.source,
            passthrough: status
                .passthrough
                .as_ref()
                .map(|mode| passthrough_name(mode).into()),
            alpha_frames: self.alpha_frames,
        };
    }

    /// Label and value pairs for the stats overlay, in display order.
    ///
    /// The head motion is reported as a speed rather than the per-frame step it is measured as:
    /// degrees and metres per second are quantities you can compare against how fast you are
    /// actually turning or walking, whereas a per-frame step only means something once you have
    /// divided it by the frame interval yourself. The deviation carries over unchanged, and it is
    /// the number that matters — under steady motion the speed should be constant, so whatever it
    /// varies by is scene movement with nothing behind it.
    ///
    /// The camera position comes first and is there with or without a stream; everything after it
    /// describes the stream and is left out until one is up.
    fn stats_entries(&mut self) -> Vec<(String, String)> {
        let position = self.camera.position;
        let mut entries = vec![(
            "position".into(),
            format!("{:.2} {:.2} {:.2} m", position.x, position.y, position.z),
        )];

        let status = self.client.status();
        if !status.streaming {
            return entries;
        }

        let stream = format!(
            "{}x{} {:.0} Hz",
            status.view_resolution.x, status.view_resolution.y, status.refresh_rate
        );
        let timing = self.client.frame_timing();

        let frames_per_second = if timing.world_ms.mean > 0.0 {
            1000.0 / timing.world_ms.mean
        } else {
            0.0
        };

        entries.extend([
            ("stream".into(), stream),
            (
                "world".into(),
                format!(
                    "{:.1}±{:.1} ms",
                    timing.world_ms.mean, timing.world_ms.deviation
                ),
            ),
            (
                "screen".into(),
                format!(
                    "{:.1}±{:.1} ms",
                    timing.screen_ms.mean, timing.screen_ms.deviation
                ),
            ),
            (
                "speed".into(),
                format!(
                    "{:.1}±{:.1} °/s   {:.2}±{:.2} m/s",
                    timing.step_deg.mean * frames_per_second,
                    timing.step_deg.deviation * frames_per_second,
                    // Millimetres per frame times frames per second is millimetres per second.
                    timing.step_mm.mean * frames_per_second / 1000.0,
                    timing.step_mm.deviation * frames_per_second / 1000.0,
                ),
            ),
            (
                "frames".into(),
                format!("{} {:?}", self.video_frames, self.video_layout),
            ),
            (
                "passthrough".into(),
                match &self.client.status().passthrough {
                    None => "off".into(),
                    Some(PassthroughMode::AlphaStream(_)) => {
                        let lag = self.alpha_lag.map_or("-".into(), |lag| {
                            format!("{:.1} ms", lag.as_secs_f32() * 1000.0)
                        });

                        format!("alpha stream, {} frames, lag {lag}", self.alpha_frames)
                    }
                    Some(mode) => passthrough_name(mode).into(),
                },
            ),
        ]);

        entries
    }

    /// The window's title bar, drawn by the app in place of the system's so it matches the toolbar
    /// under it: the app name and the connection status on the left, and the window buttons on the
    /// right. Dragging it moves the window and double-clicking it maximises.
    fn draw_title_bar(&self, ui: &mut Ui) {
        let margin = BAR_MARGIN as i8;
        let frame = egui::Frame::new()
            .fill(ui.visuals().panel_fill)
            .inner_margin(egui::Margin {
                left: margin,
                right: margin / 2,
                top: margin,
                bottom: margin / 2,
            });

        egui::Panel::top(egui::Id::new("title bar"))
            .frame(frame)
            .show_separator_line(false)
            .resizable(false)
            .show(ui, |ui| {
                icons::toolbar_style(ui.style_mut(), TOOLBAR_SCALE);

                // Registered before the contents, so the buttons on top of it take the pointer
                // first and only the empty parts of the bar move the window.
                let bar = egui::Rect::from_min_size(
                    ui.max_rect().min,
                    egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                );
                let drag = ui.interact(
                    bar,
                    egui::Id::new("title bar drag"),
                    Sense::click_and_drag(),
                );
                let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));

                if drag.double_clicked() {
                    ui.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                } else if drag.drag_started_by(PointerButton::Primary) {
                    ui.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.horizontal(|ui| {
                    // Indented like the controls inside the groups below, so the title lines up
                    // with them rather than with the groups' rounded edges.
                    ui.add_space(icons::group_margin(ui));
                    ui.label(icons::normal_text(ui, "ALVR Client Emulator").strong());
                    let status = self.status();
                    connection_status(ui, &status);

                    // Only worth the space while a hand is emulated: controllers click on their
                    // own, whatever this setting says.
                    if status.hand_interaction == Some(false)
                        && self.hands.iter().any(|state| state.enabled)
                    {
                        hand_interaction_warning(ui);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icons::button(ui, Icon::Close, "Close").clicked() {
                            ui.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        let (icon, tooltip) = if maximized {
                            (Icon::Restore, "Restore")
                        } else {
                            (Icon::Maximize, "Maximise")
                        };
                        if icons::button(ui, icon, tooltip).clicked() {
                            ui.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }

                        if icons::button(ui, Icon::Minimize, "Minimise").clicked() {
                            ui.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    fn draw_toolbar(&mut self, ui: &mut Ui) {
        let margin = BAR_MARGIN as i8;
        let frame = egui::Frame::new()
            .fill(ui.visuals().panel_fill)
            .inner_margin(egui::Margin::symmetric(margin, margin / 2));

        let toolbar = egui::Panel::top(egui::Id::new("toolbar"))
            .frame(frame)
            .show_separator_line(false)
            .resizable(false)
            .show(ui, |ui| {
                icons::toolbar_style(ui.style_mut(), TOOLBAR_SCALE);

                // Every group in one row, wrapping group by group onto further lines when the
                // window is too narrow. Groups are told apart by space; options that exclude each
                // other share one segmented control, so the grouping is visible without a caption.
                ui.spacing_mut().item_spacing.y = 6.0 * icons::size_factor(ui);
                ui.horizontal_wrapped(|ui| {
                    icons::flex_item(ui, "view mode", |ui| {
                        icons::segmented(
                            ui,
                            "view mode",
                            &mut self.view_mode,
                            &[
                                (ViewMode::Left, Icon::LeftEye, "Left eye"),
                                (ViewMode::Right, Icon::RightEye, "Right eye"),
                                (ViewMode::Stereo, Icon::BothEyes, "Both eyes, side by side"),
                            ],
                        );
                    });

                    // The stream is the virtual content, the local scene stands in for what the
                    // headset's cameras would see. Switching to the latter while streaming is also
                    // how to tell a decode problem apart from a rendering one.
                    icons::flex_item(ui, "view source", |ui| {
                        icons::segmented(
                            ui,
                            "view source",
                            &mut self.source,
                            &[
                                (
                                    ViewSource::Mixed,
                                    Icon::Mixed,
                                    "Mixed: what the headset shows. The streamed video over the \
                                     local scene, blended by the stream's passthrough mode; plain \
                                     video when passthrough is off.",
                                ),
                                (
                                    ViewSource::Virtual,
                                    Icon::Virtual,
                                    "Virtual: the streamed video alone, opaque.",
                                ),
                                (
                                    ViewSource::Real,
                                    Icon::Real,
                                    "Real: the local scene alone, standing in for the \
                                     headset's cameras.",
                                ),
                            ],
                        );
                    });

                    // A group of one, so the toggle matches the segmented controls beside it. The
                    // numbers go in an overlay over the view, where there is room to label them;
                    // see [`draw_stats_overlay`].
                    icons::flex_item(ui, "stats", |ui| {
                        icons::group(ui, |ui| {
                            icons::toggle(
                                ui,
                                &mut self.show_stats,
                                Icon::Stats,
                                "Statistics: camera position, and the stream and its frame \
                                 pacing while streaming",
                            );
                        });
                    });

                    self.draw_input_groups(ui);
                });
            });

        // A hairline and a soft shadow along the bottom edge, so the toolbar reads as a bar resting
        // over the view rather than a block of controls stacked on top of it. Painted on a layer
        // above the view, which would otherwise cover anything drawn below the panel.
        let bar = toolbar.response.rect;
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("toolbar shadow"),
        ));
        let shadow_steps = 6;
        for step in 0..shadow_steps {
            let fade = 1.0 - step as f32 / shadow_steps as f32;
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(bar.left(), bar.bottom() + step as f32),
                    egui::pos2(bar.right(), bar.bottom() + step as f32 + 1.0),
                ),
                0.0,
                Color32::from_black_alpha((22.0 * fade * fade) as u8),
            );
        }
        painter.hline(
            bar.x_range(),
            bar.bottom(),
            egui::Stroke::new(1.0, icons::line_color(ui.visuals())),
        );
    }

    /// A thin bar along the bottom of the window with the hints on how to look around and drive
    /// the devices, in one line cut short rather than wrapped when the window is narrow. Styled
    /// like the dashboard's bottom bar showing an ordinary message, and indented to line up with
    /// the title.
    fn draw_status_bar(&self, ui: &mut Ui) {
        let hints = [Some(self.look_hint()), self.device_hint()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("    ·    ");

        let (fill, text_color) = icons::status_bar_colors(ui.visuals());

        // The title's indent: the bars' margin, and the group margin it adds to that.
        let indent = (BAR_MARGIN + icons::GROUP_MARGIN * TOOLBAR_SCALE) as i8;
        let frame = egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin::symmetric(indent, 5));

        let bar = egui::Panel::bottom(egui::Id::new("status bar"))
            .frame(frame)
            .show_separator_line(false)
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let text = RichText::new(hints).color(text_color).size(12.0);
                    ui.add(egui::Label::new(text).truncate());
                });
            })
            .response
            .rect;

        ui.painter().hline(
            bar.x_range(),
            bar.top(),
            egui::Stroke::new(1.0, icons::line_color(ui.visuals())),
        );

        overlay::set_bottom_inset(ui.ctx(), bar.height());
    }

    /// The controller section and the hand section, side by side, since a side can be emulated as
    /// one or the other and the two toggles belong next to each other.
    fn draw_input_groups(&mut self, ui: &mut Ui) {
        let mut hands_enabled = [self.hands[0].enabled, self.hands[1].enabled];
        icons::flex_item(ui, "controllers", |ui| {
            icons::group(ui, |ui| {
                controller_ui::toolbar_row(
                    ui,
                    &mut self.controllers,
                    &mut hands_enabled,
                    &self.controller_settings,
                    &mut self.controller_gizmo,
                )
            });
        });
        for (state, enabled) in self.hands.iter_mut().zip(hands_enabled) {
            state.enabled = enabled;
        }

        icons::flex_item(ui, "hands", |ui| {
            icons::group(ui, |ui| {
                hand_ui::toolbar_row(
                    ui,
                    &mut self.hands,
                    &mut self.controllers,
                    &self.hand_settings,
                    &mut self.hand_gizmo,
                )
            });
        });
    }

    /// How to look around, or how to stop.
    fn look_hint(&self) -> &'static str {
        if self.look_latched {
            "Esc to release mouse"
        } else if self.look_held {
            "Release to stop looking"
        } else {
            "Click to look, or hold right button"
        }
    }

    /// How to drive the enabled devices, if any are.
    fn device_hint(&self) -> Option<&'static str> {
        let controllers = self.controllers.iter().any(|state| state.enabled);
        let hands = self.hands.iter().any(|state| state.enabled);

        match (controllers, hands) {
            (true, true) => Some("Hover an icon to move a device; the corner panels drive it"),
            (true, false) => {
                Some("Hover a controller icon to move it; use the corner panels for buttons")
            }
            (false, true) => {
                Some("Hover a hand icon to move it; pick a gesture in the corner panels")
            }
            (false, false) => None,
        }
    }
}

/// Resizes the window from its edges and corners. Drawing our own title bar means doing without
/// the system frame, and with it the borders the system resizes from.
fn handle_window_edges(ctx: &egui::Context) {
    use egui::{CursorIcon, ResizeDirection};

    /// How far in from the window's edge a resize can start, in points.
    const EDGE: f32 = 5.0;

    if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
        return;
    }

    let Some(pointer) = ctx.input(|input| input.pointer.hover_pos()) else {
        return;
    };

    let window = ctx.content_rect();
    let west = pointer.x <= window.left() + EDGE;
    let east = pointer.x >= window.right() - EDGE;
    let north = pointer.y <= window.top() + EDGE;
    let south = pointer.y >= window.bottom() - EDGE;

    let (direction, cursor) = match (north, south, west, east) {
        (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNwSe),
        (true, _, _, true) => (ResizeDirection::NorthEast, CursorIcon::ResizeNeSw),
        (_, true, true, _) => (ResizeDirection::SouthWest, CursorIcon::ResizeNeSw),
        (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeNwSe),
        (true, ..) => (ResizeDirection::North, CursorIcon::ResizeVertical),
        (_, true, ..) => (ResizeDirection::South, CursorIcon::ResizeVertical),
        (.., true, _) => (ResizeDirection::West, CursorIcon::ResizeHorizontal),
        (.., true) => (ResizeDirection::East, CursorIcon::ResizeHorizontal),
        _ => return,
    };

    ctx.set_cursor_icon(cursor);

    if ctx.input(|input| input.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
    }
}

impl App for EmulatorApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut Frame) {
        let ui_context = ui.ctx().clone();

        let now = Instant::now();
        let delta_seconds = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        self.client.poll_events();
        let haptics = self.client.take_haptics();
        self.controller_ui.apply_haptics(haptics);

        self.apply_pending_moves();
        self.apply_pending_source();
        self.apply_controller_commands();
        self.apply_hand_commands();
        self.apply_timed_releases();

        // Gesture transitions run on wall time rather than on frames, so a slow frame does not
        // stretch the animation.
        for state in self.hands.iter_mut() {
            state.advance(&self.hand_settings, now);
        }

        self.update_camera(&ui_context, delta_seconds);
        let camera = self.camera;

        self.ensure_controller_models(frame);
        self.ensure_hand_models(frame);
        self.ensure_gizmo_renderer(frame);
        let displayed_frame = self.upload_decoded_frame(frame);
        // The world-space eye poses the displayed frame was rendered with. Also reports the
        // compositor start for the frame pacing statistics.
        let video_views = self.client.displayed_frame_views(displayed_frame);

        self.draw_title_bar(ui);
        self.draw_toolbar(ui);
        self.draw_status_bar(ui);

        // Falls back to the scene until the first frame arrives, so the window is never blank while
        // the stream is negotiating.
        let show_video = self.source != ViewSource::Real && displayed_frame.is_some();
        // With passthrough the video is see-through, and the scene is what shows through it.
        let scene_under_video = show_video && self.passthrough().is_some();
        // The video displays letterboxed at its own aspect ratio; the scene adapts to the window.
        let video_eye_aspect = show_video.then_some(self.video_eye_aspect).flatten();

        // What the overlays (icons, controller models) are aligned with. Over the video they
        // follow the pose the displayed frame was rendered with, so they move in lockstep with the
        // video content — anchoring them to the live camera made every wobble of video timing
        // visible as the in-video controllers jittering against them. Over the scene, which is
        // rendered live, they follow the live camera.
        let (overlay_position, overlay_orientation, overlay_eye_views) =
            match (show_video, video_views) {
                (true, Some(views)) => (
                    (views[0].pose.position + views[1].pose.position) / 2.0,
                    views[0].pose.orientation,
                    [eye_view_matrix(&views[0]), eye_view_matrix(&views[1])],
                ),
                _ => (
                    camera.position,
                    camera.orientation(),
                    [
                        camera.view_matrix(Eye::Left),
                        camera.view_matrix(Eye::Right),
                    ],
                ),
            };

        let scene_error = self.scene_error.clone();
        let view_mode = self.view_mode;
        let camera_snapshot = CameraSnapshot {
            position: self.camera.position,
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
            roll: self.camera.roll,
        };
        let controller_models =
            self.controller_model_matrices(overlay_position, overlay_orientation);
        let hand_poses = self.hand_joint_matrices(overlay_position, overlay_orientation);
        let gizmos = self.gizmo_transforms(overlay_position, overlay_orientation);
        let mut look_requested = LookRequest::None;

        let view_rect = egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                if let Some(message) = &scene_error {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Could not load environment\n\n{message}\n\nRebuild to restore the bundled test room, or place your own {} in {}.",
                                scene::ENVIRONMENT_FILE_NAME,
                                paths::user_dir().display()
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
                        scene_under_video,
                        video_eye_aspect,
                        controller_models,
                        hand_poses,
                        device_eye_views: overlay_eye_views,
                        gizmos,
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

        if let Some(view_rect) = view_rect
            && self.show_stats
        {
            let entries = self.stats_entries();
            draw_stats_overlay(&ui_context, view_rect, &entries);
        }

        if let Some(view_rect) = view_rect {
            let views = view_sub_rects(view_mode, view_rect, video_eye_aspect, overlay_eye_views);

            // The controllers and the hands are posed by one implementation, so they are collected
            // into one list of enabled devices. The borrows are split by hand and by kind, so no
            // two targets alias.
            let controller_sensitivity = self.controller_settings.rotation_sensitivity;
            let hand_sensitivity = self.hand_settings.rotation_sensitivity;

            let mut targets = Vec::with_capacity(4);

            for (index, state) in self.controllers.iter_mut().enumerate() {
                if state.enabled {
                    targets.push(PoseTarget {
                        slot: Slot::controller(Hand::BOTH[index]),
                        position: &mut state.position,
                        orientation: &mut state.orientation,
                        aim: Vec3::NEG_Z,
                        rotation_sensitivity: controller_sensitivity,
                    });
                }
            }

            for (index, state) in self.hands.iter_mut().enumerate() {
                if state.enabled {
                    targets.push(PoseTarget {
                        slot: Slot::hand(Hand::BOTH[index]),
                        position: &mut state.position,
                        orientation: &mut state.orientation,
                        aim: hands::aim_direction(Hand::BOTH[index]),
                        rotation_sensitivity: hand_sensitivity,
                    });
                }
            }

            self.pose_overlay.run(
                &ui_context,
                &views,
                (overlay_position, overlay_orientation),
                &mut targets,
                interactive,
            );
        }

        self.controller_ui.controller_panels(
            &ui_context,
            &mut self.controllers,
            &self.controller_settings,
            interactive,
        );

        hand_ui::hand_panels(
            &ui_context,
            &mut self.hands,
            &self.hand_settings,
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

        // Last, so its resize cursor wins over whatever is under the window's edge.
        if !look_active {
            handle_window_edges(&ui_context);
        }

        self.service_captures(frame);
        self.publish_state();
        self.publish_controllers();
        self.publish_hands();

        // Completes the pacing report for the frame shown this repaint.
        self.client.finish_frame();

        // The scene is continuously interactive, so keep painting.
        ui_context.request_repaint();
    }

    /// Shuts the ALVR connection down on window close.
    fn on_exit(&mut self) {
        self.client.shutdown();
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
    /// Draw the local scene behind the video, for the video to blend over. It is drawn from the
    /// displayed frame's eye poses and the stream's projection, so the two line up exactly as the
    /// headset's reprojected layer lines up with its passthrough.
    scene_under_video: bool,
    /// Pixel aspect ratio of one eye of the video, when the video is shown. The video is displayed
    /// inner-fit at this ratio rather than stretched to the viewport.
    video_eye_aspect: Option<f32>,
    /// World-space model matrices of the controllers to draw. Drawn over the video as well: the
    /// VR application renders its own controllers, but the local models show what the emulator is
    /// actually sending, which is the point of the display toggle.
    controller_models: [Option<Mat4>; 2],
    /// World-space skinning matrices of the hands to draw, for the same reason.
    hand_poses: [Option<Vec<Mat4>>; 2],
    /// Per-eye view matrices the device models are drawn with: the displayed video frame's eye
    /// poses in video mode, the live camera's in scene mode. Left is index 0.
    device_eye_views: [Mat4; 2],
    /// World transforms of the debug pose gizmos, empty unless one is selected in the toolbar.
    gizmos: [Option<Mat4>; GIZMO_SLOTS],
}

impl ViewportCallback {
    /// The device models to draw, resolved against the renderers in the callback resources.
    ///
    /// Both renderers are created lazily, so either may be missing; a device with nothing to draw
    /// simply contributes nothing.
    fn devices<'resources>(
        &'resources self,
        controllers: Option<&'resources ControllerRenderer>,
        hands: Option<&'resources HandRenderer>,
        gizmos: Option<&'resources GizmoRenderer>,
    ) -> DeviceModels<'resources> {
        DeviceModels {
            controllers: controllers.map(|renderer| (renderer, self.controller_models)),
            hands: hands.map(|renderer| {
                (
                    renderer,
                    std::array::from_fn(|hand| self.hand_poses[hand].as_deref()),
                )
            }),
            gizmos: gizmos.map(|renderer| (renderer, self.gizmos)),
        }
    }
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

        // Over the video the models must use the projection the server rendered with — the
        // advertised FOV — not the window's shape, or they drift against the video content.
        let device_aspect = if self.show_video {
            Camera::fov_aspect_ratio()
        } else {
            aspect_ratio
        };

        // Under the video the scene follows the same rule as the device models, for the same
        // reason: anything else drifts against the video content.
        if self.scene_under_video
            && let Some(renderer) = resources.get::<Arc<SceneRenderer>>()
        {
            let projection = Camera::projection_matrix(device_aspect);

            renderer.set_view_projection(queue, Eye::Left, projection * self.device_eye_views[0]);
            renderer.set_view_projection(queue, Eye::Right, projection * self.device_eye_views[1]);
        }

        let controllers = resources
            .get::<Arc<Mutex<ControllerRenderer>>>()
            .map(|r| r.lock());
        let hands = resources
            .get::<Arc<Mutex<HandRenderer>>>()
            .map(|r| r.lock());
        let gizmos = resources.get::<Arc<GizmoRenderer>>();

        self.devices(
            controllers.as_deref(),
            hands.as_deref(),
            gizmos.map(|renderer| &**renderer),
        )
        .upload(
            queue,
            self.device_eye_views,
            Camera::projection_matrix(device_aspect),
        );

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
        let controllers = resources
            .get::<Arc<Mutex<ControllerRenderer>>>()
            .map(|r| r.lock());
        let hands = resources
            .get::<Arc<Mutex<HandRenderer>>>()
            .map(|r| r.lock());
        let gizmos = resources.get::<Arc<GizmoRenderer>>();
        let devices = self.devices(
            controllers.as_deref(),
            hands.as_deref(),
            gizmos.map(|renderer| &**renderer),
        );

        if video.is_none() && scene.is_none() {
            return;
        }

        let draw_eye = |pass: &mut wgpu::RenderPass<'static>, eye: Eye| {
            match &video {
                Some(video) => {
                    if self.scene_under_video
                        && let Some(scene) = scene
                    {
                        scene.draw(pass, eye);
                    }

                    video.lock().draw(pass, eye);
                }
                None => {
                    if let Some(scene) = scene {
                        scene.draw(pass, eye);
                    }
                }
            }

            // Over the video too: the application draws its own controllers and hands, but the
            // local models show what the emulator is sending, which is what the display toggle is
            // for.
            devices.draw(pass, eye);
        };

        // The video keeps its own aspect ratio, displayed inner-fit within the eye's part of the
        // viewport; stretching it also mismatched the controller overlays. The scene has no fixed
        // aspect and fills the viewport.
        let draw_view = |pass: &mut wgpu::RenderPass<'static>,
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

/// A pill saying whether a stream is up, on a faint wash of its own colour, with the client core's
/// full status message, which carries the server address and any error, on hover. The stream's
/// resolution and refresh rate are in the stats overlay.
fn connection_status(ui: &mut Ui, status: &ClientStatus) {
    let (color, text) = if status.streaming {
        (icons::ok_color(ui.visuals()), "Streaming")
    } else {
        (icons::error_color(ui.visuals()), "Not connected")
    };

    let tooltip = if status.hud_message.is_empty() {
        "Connection to the ALVR server".to_owned()
    } else {
        status.hud_message.clone()
    };

    status_pill(ui, None, color, text, tooltip);
}

/// Shown beside the connection status when the server ignores hand gestures, which otherwise
/// looks like a broken pinch: the hand closes on screen and SteamVR never sees a click.
fn hand_interaction_warning(ui: &mut Ui) {
    status_pill(
        ui,
        Some(Icon::NoHand),
        icons::error_color(ui.visuals()),
        "Hand clicks off",
        "Turn on ALVR Dashboard / Settings / Headset / Controllers / Hand tracking interaction"
            .to_owned(),
    );
}

/// A rounded label led by a dot, or by `icon` when given, on a faint wash of its own colour,
/// explained on hover.
fn status_pill(ui: &mut Ui, icon: Option<Icon>, color: Color32, text: &str, tooltip: String) {
    let size = icons::size_factor(ui);
    let galley = egui::WidgetText::from(RichText::new(text).color(color).small()).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    );

    let padding = egui::vec2(9.0, 3.0) * size;
    // An icon needs more room than the dot to stay legible; both are centred in the same slot.
    let dot_radius = if icon.is_some() { 6.0 } else { 3.5 } * size;
    let gap = 6.0 * size;
    let pill_size = egui::vec2(
        padding.x * 2.0 + dot_radius * 2.0 + gap + galley.size().x,
        ui.spacing().interact_size.y - 4.0 * size,
    );

    let (pill, response) = ui.allocate_exact_size(pill_size, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(pill, pill.height() / 2.0, color.gamma_multiply(0.14));
    let marker = egui::pos2(pill.left() + padding.x + dot_radius, pill.center().y);
    match icon {
        Some(icon) => icons::paint_into(
            painter,
            egui::Rect::from_center_size(marker, egui::Vec2::splat(dot_radius * 2.0)),
            icon,
            color,
        ),
        None => {
            painter.circle_filled(marker, dot_radius, color);
        }
    }
    painter.galley(
        egui::pos2(
            pill.left() + padding.x + dot_radius * 2.0 + gap,
            pill.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );

    response.on_hover_text(tooltip);
}

/// Short name of a passthrough mode, for the stats overlay and the API.
fn passthrough_name(mode: &PassthroughMode) -> &'static str {
    match mode {
        PassthroughMode::Blend { .. } => "blend",
        PassthroughMode::RgbChromaKey(_) => "RGB chroma key",
        PassthroughMode::HsvChromaKey(_) => "HSV chroma key",
        PassthroughMode::AlphaStream(_) => "alpha stream",
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

/// Gap between a stat's label and its value, and between one stat and the next.
const STATS_LABEL_GAP: f32 = 5.0;
const STATS_ENTRY_GAP: f32 = 18.0;
/// Vertical gap between lines, when the stats wrap.
const STATS_LINE_GAP: f32 = 4.0;

/// Frame pacing readout, drawn as a rounded strip over the top of the view.
///
/// Painted rather than built from widgets: it is a pure readout with no interaction, and painting
/// it means the strip is centred correctly on the very first frame. An `Area` would have to be
/// measured once before it could be placed, which is the same trap the controller panels avoid by
/// positioning themselves from explicit coordinates.
///
/// Labels are dimmed and values are not, so the numbers read at a glance and the words stay out of
/// the way.
fn draw_stats_overlay(ctx: &egui::Context, view_rect: egui::Rect, entries: &[(String, String)]) {
    if entries.is_empty() {
        return;
    }

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("stats overlay"),
    ));

    let label_font = egui::FontId::proportional(12.0);
    let value_font = egui::FontId::monospace(12.0);
    let label_color = Color32::from_rgb(150, 155, 165);

    // Laid out before anything is drawn, so the strip can be sized and centred to fit.
    let galleys = entries
        .iter()
        .map(|(label, value)| {
            (
                painter.layout_no_wrap(label.clone(), label_font.clone(), label_color),
                painter.layout_no_wrap(value.clone(), value_font.clone(), Color32::WHITE),
            )
        })
        .collect::<Vec<_>>();

    // Each value gets a slot as wide as the widest it has been, rather than its current width.
    // The values change length from frame to frame ("4.5" against "10.2", a growing frame count),
    // and sizing the centred strip to the current text made the whole strip resize and every entry
    // slide sideways each frame. Slots only grow, so the layout settles within a moment and stays.
    let widths_id = egui::Id::new("stats overlay value widths");
    let mut value_widths: Vec<f32> = ctx
        .data(|data| data.get_temp(widths_id))
        .unwrap_or_default();
    // A different set of entries is a different layout; start it afresh.
    value_widths.resize(galleys.len(), 0.0);
    for (width, (_, value)) in value_widths.iter_mut().zip(&galleys) {
        *width = width.max(value.size().x);
    }
    ctx.data_mut(|data| data.insert_temp(widths_id, value_widths.clone()));

    let entry_widths = galleys
        .iter()
        .zip(&value_widths)
        .map(|((label, _), value_width)| label.size().x + STATS_LABEL_GAP + value_width)
        .collect::<Vec<_>>();
    let line_height = galleys
        .iter()
        .map(|(label, value)| label.size().y.max(value.size().y))
        .fold(0.0, f32::max);

    let margin = egui::vec2(12.0, 6.0);

    // Entries wrap onto further lines rather than running off the sides of a narrow view. Packed
    // greedily in order; an entry wider than the view still gets a line of its own. The slot
    // widths only grow, so once they settle the breaks stop moving too.
    let max_width = (view_rect.width() - 20.0 - margin.x * 2.0).max(0.0);
    let mut lines: Vec<(Vec<usize>, f32)> = vec![];
    for (index, &width) in entry_widths.iter().enumerate() {
        match lines.last_mut() {
            Some((entries, line_width)) if *line_width + STATS_ENTRY_GAP + width <= max_width => {
                entries.push(index);
                *line_width += STATS_ENTRY_GAP + width;
            }
            _ => lines.push((vec![index], width)),
        }
    }

    let content_width = lines.iter().map(|(_, width)| *width).fold(0.0, f32::max);
    let content_height =
        line_height * lines.len() as f32 + STATS_LINE_GAP * (lines.len() - 1) as f32;

    let strip = egui::Rect::from_center_size(
        egui::pos2(
            view_rect.center().x,
            view_rect.top() + 10.0 + content_height / 2.0 + margin.y,
        ),
        egui::vec2(content_width, content_height) + margin * 2.0,
    );

    painter.rect_filled(
        strip,
        egui::CornerRadius::same(8),
        Color32::from_black_alpha(170),
    );

    let mut galleys = galleys.into_iter().map(Some).collect::<Vec<_>>();
    for (line, (entries, _)) in lines.iter().enumerate() {
        let mut cursor = strip.left_top()
            + margin
            + egui::vec2(0.0, line as f32 * (line_height + STATS_LINE_GAP));

        for &index in entries {
            let Some((label, value)) = galleys[index].take() else {
                continue;
            };
            let label_width = label.size().x;

            painter.galley(cursor, label, label_color);
            painter.galley(
                cursor + egui::vec2(label_width + STATS_LABEL_GAP, 0.0),
                value,
                Color32::WHITE,
            );

            cursor.x += entry_widths[index] + STATS_ENTRY_GAP;
        }
    }
}

/// View matrix of one eye, from the world-space eye pose the server reported for a video frame.
fn eye_view_matrix(params: &alvr_common::ViewParams) -> Mat4 {
    Mat4::from_rotation_translation(params.pose.orientation, params.pose.position).inverse()
}

/// The sub-rectangles the view is split into, with the projection aspect ratio and the eye view
/// matrix that map world space onto each rectangle.
///
/// With the video shown (`video_eye_aspect` set), each rectangle is the letterboxed video area,
/// and the projection is the advertised FOV's — the one the server rendered the video with. The
/// scene fills the viewport and is projected at the rectangle's own aspect ratio.
fn view_sub_rects(
    view_mode: ViewMode,
    rect: egui::Rect,
    video_eye_aspect: Option<f32>,
    eye_views: [Mat4; 2],
) -> Vec<(egui::Rect, f32, Mat4)> {
    let outer: Vec<(Eye, egui::Rect)> = match view_mode {
        ViewMode::Left => vec![(Eye::Left, rect)],
        ViewMode::Right => vec![(Eye::Right, rect)],
        ViewMode::Stereo => {
            let half = rect.width() / 2.0;
            vec![
                (
                    Eye::Left,
                    egui::Rect::from_min_size(rect.min, egui::vec2(half, rect.height())),
                ),
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
        .map(|(eye, rect)| {
            let view = eye_views[match eye {
                Eye::Left => 0,
                Eye::Right => 1,
            }];

            match video_eye_aspect {
                Some(aspect) => {
                    let (left, top, width, height) =
                        fit_viewport(rect.left(), rect.top(), rect.width(), rect.height(), aspect);

                    (
                        egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height)),
                        Camera::fov_aspect_ratio(),
                        view,
                    )
                }
                None => (rect, rect.width() / rect.height().max(1.0), view),
            }
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

/// The emulator icon for the taskbar and Alt-Tab, the same file `build.rs` embeds in the executable.
fn window_icon() -> egui::IconData {
    let icon = ico::IconDir::read(std::io::Cursor::new(include_bytes!(
        "../resources/emulator.ico"
    )))
    .ok()
    .and_then(|dir| {
        // The largest entry, so Windows scales down rather than up.
        let entry = dir.entries().iter().max_by_key(|entry| entry.width())?;
        entry.decode().ok()
    });

    match icon {
        Some(image) => egui::IconData {
            rgba: image.rgba_data().to_owned(),
            width: image.width(),
            height: image.height(),
        },
        None => {
            alvr_common::warn!("Cannot decode the window icon");
            egui::IconData::default()
        }
    }
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
        frame_timing: FrameTiming::default(),
        source: ViewSource::Mixed,
        passthrough: None,
        alpha_frames: 0,
    }));

    if let Err(e) = api::spawn(Arc::clone(&shared), port) {
        error!("{e}");
    }

    info!("Starting ALVR client emulator");

    let result = eframe::run_native(
        "ALVR Client Emulator",
        NativeOptions {
            // Wide enough for every toolbar group on one line. Undecorated: the app draws its
            // own title bar; see `draw_title_bar`.
            viewport: ViewportBuilder::default()
                .with_inner_size((1440.0, 810.0))
                .with_min_inner_size((640.0, 400.0))
                .with_decorations(false)
                .with_icon(window_icon()),
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
