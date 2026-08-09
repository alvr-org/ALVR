// Hide the console window in release builds, matching alvr_dashboard. Debug builds keep it so log
// output stays visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! ALVR client emulator.
//!
//! A desktop application that connects to an ALVR server as if it were a headset, renders a glTF
//! environment from a first person camera, and exposes an HTTP API so the emulated headset can be
//! inspected and driven programmatically.
//!
//! This iteration renders its own scene only: the video stream from the server is not decoded.

mod api;
mod camera;
mod client;
mod render;
mod scene;

use crate::{
    api::{CaptureRequestKind, SharedState, StateResponse},
    camera::{Camera, CameraInput, Eye},
    client::{ClientStatus, EmulatedClient},
    render::{CaptureKind, SceneRenderer, capture_stereo},
    scene::Scene,
};
use alvr_common::{error, glam::Vec3, info};
use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{self, Color32, Key, PointerButton, RichText, Sense, Ui, ViewportBuilder},
    egui_wgpu,
};
use std::{
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

/// Resolution of one eye in the capture endpoints, used when no stream has been negotiated yet.
const FALLBACK_CAPTURE_RESOLUTION: (u32, u32) = (960, 916);

/// What the pointer asked for this frame. Collected inside the panel closure, which cannot borrow
/// the app mutably, and applied afterwards.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LookRequest {
    None,
    /// Stay in look mode until Escape.
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
    /// Latched look mode: left-click the view to enter, Escape to leave.
    look_latched: bool,
    /// Momentary look mode: active only while the right button is held.
    look_held: bool,
    /// Mirrors the grab state actually sent to the window, so the command is only sent on change.
    cursor_grabbed: bool,
    last_frame: Instant,
    shared: Arc<SharedState>,
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

        Self {
            client: Some(EmulatedClient::new()),
            camera,
            scene,
            scene_error,
            environment_path,
            view_mode: ViewMode::Stereo,
            look_latched: false,
            look_held: false,
            cursor_grabbed: false,
            last_frame: Instant::now(),
            shared,
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

            let pixels = capture_stereo(
                &state.device,
                &state.queue,
                &renderer,
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
        self.apply_pending_moves();
        self.update_camera(&ui_context, delta_seconds);
        let camera = self.camera;
        self.client().set_pose(&camera);

        self.draw_toolbar(ui);

        let scene_error = self.scene_error.clone();
        let view_mode = self.view_mode;
        let camera_snapshot = CameraSnapshot {
            position: self.camera.position,
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
            roll: self.camera.roll,
        };
        let mut look_requested = LookRequest::None;

        egui::CentralPanel::default()
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
                    return;
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
                    },
                ));
            });

        match look_requested {
            LookRequest::Latch => self.look_latched = true,
            LookRequest::Hold => self.look_held = true,
            LookRequest::None => (),
        }

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

        // Frame pacing reports keep the server's latency estimate sane. Use the negotiated refresh
        // rate when streaming so the reported interval matches what the server expects.
        let status_rate = self.status().refresh_rate;
        let interval = if status_rate > 0.0 {
            Duration::from_secs_f32(1.0 / status_rate)
        } else {
            Duration::from_millis(16)
        };
        self.client().report_frame(interval);

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
        if let Some(renderer) = resources.get::<Arc<SceneRenderer>>() {
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

            // Both slots are written even in single-eye mode; the draw call picks one.
            renderer.set_view(queue, &camera, Eye::Left, aspect_ratio);
            renderer.set_view(queue, &camera, Eye::Right, aspect_ratio);

            let _ = screen_descriptor;
        }

        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(renderer) = resources.get::<Arc<SceneRenderer>>() {
            let viewport = info.viewport_in_pixels();
            let left = viewport.left_px as f32;
            let top = viewport.top_px as f32;
            let width = viewport.width_px as f32;
            let height = viewport.height_px as f32;

            match self.view_mode {
                ViewMode::Left => {
                    pass.set_viewport(left, top, width, height, 0.0, 1.0);
                    renderer.draw(pass, Eye::Left);
                }
                ViewMode::Right => {
                    pass.set_viewport(left, top, width, height, 0.0, 1.0);
                    renderer.draw(pass, Eye::Right);
                }
                ViewMode::Stereo => {
                    // Side by side, each eye in its own half of the viewport.
                    let half = width / 2.0;

                    pass.set_viewport(left, top, half, height, 0.0, 1.0);
                    renderer.draw(pass, Eye::Left);

                    pass.set_viewport(left + half, top, half, height, 0.0, 1.0);
                    renderer.draw(pass, Eye::Right);
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
