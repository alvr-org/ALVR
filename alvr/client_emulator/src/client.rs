//! ALVR connection. Wraps `ClientCoreContext` and feeds it head poses from the emulator camera.
//!
//! No video is decoded in this iteration: the emulator renders its own scene, and the connection
//! exists to exercise the protocol, tracking and connection state. Because `set_decoder_input_callback`
//! is never registered, the server's video packets are simply dropped by the client core.

use crate::camera::{Camera, IPD};
use alvr_client_core::{ClientCapabilities, ClientCoreContext, ClientCoreEvent};
use alvr_common::{
    DeviceMotion, HEAD_ID, Pose, RelaxedAtomic, ViewParams,
    glam::{UVec2, Vec3},
    info,
    parking_lot::RwLock,
};
use alvr_packets::{FaceData, TrackingData};
use alvr_session::CodecType;
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Default view resolution advertised to the server, matching a Quest-class headset.
const DEFAULT_VIEW_RESOLUTION: UVec2 = UVec2::new(1920, 1832);

/// Connection state mirrored out of the client core for the UI and the HTTP API.
#[derive(Clone)]
pub struct ClientStatus {
    /// Message the client core wants shown while not streaming (server address, errors, ...).
    pub hud_message: String,
    pub streaming: bool,
    pub view_resolution: UVec2,
    pub refresh_rate: f32,
    pub codec: Option<CodecType>,
}

impl Default for ClientStatus {
    fn default() -> Self {
        Self {
            hud_message: String::new(),
            streaming: false,
            view_resolution: UVec2::ZERO,
            refresh_rate: 0.0,
            codec: None,
        }
    }
}

/// The pose the tracking thread reads. Written by the UI thread every frame.
#[derive(Clone, Copy)]
pub struct TrackedPose {
    pub position: Vec3,
    pub orientation: alvr_common::glam::Quat,
}

impl Default for TrackedPose {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 1.6, 0.0),
            orientation: alvr_common::glam::Quat::IDENTITY,
        }
    }
}

pub struct EmulatedClient {
    context: Arc<ClientCoreContext>,
    status: ClientStatus,
    /// Shared with the tracking thread.
    pose: Arc<RwLock<TrackedPose>>,
    streaming: Arc<RelaxedAtomic>,
    tracking_thread: Option<JoinHandle<()>>,
    /// Timestamp of the frame currently being "presented", for the timing reports.
    current_frame: Duration,
}

impl EmulatedClient {
    pub fn new() -> Self {
        let capabilities = ClientCapabilities {
            platform: alvr_system_info::platform(None, None),
            default_view_resolution: DEFAULT_VIEW_RESOLUTION,
            max_view_resolution: DEFAULT_VIEW_RESOLUTION,
            refresh_rates: vec![60.0, 72.0, 80.0, 90.0, 120.0],
            foveated_encoding: false,
            encoder_high_profile: false,
            encoder_10_bits: false,
            encoder_av1: false,
            prefer_10bit: false,
            preferred_encoding_gamma: 1.0,
            prefer_hdr: false,
        };

        let context = Arc::new(ClientCoreContext::new(capabilities));
        context.resume();

        Self {
            context,
            status: ClientStatus::default(),
            pose: Arc::new(RwLock::new(TrackedPose::default())),
            streaming: Arc::new(RelaxedAtomic::new(false)),
            tracking_thread: None,
            current_frame: Duration::ZERO,
        }
    }

    pub fn status(&self) -> &ClientStatus {
        &self.status
    }

    /// Publishes the current camera pose for the tracking thread to send.
    pub fn set_pose(&self, camera: &Camera) {
        *self.pose.write() = TrackedPose {
            position: camera.position,
            orientation: camera.orientation(),
        };
    }

    /// Drains client core events and keeps [`ClientStatus`] in step. Call once per frame.
    pub fn poll_events(&mut self) {
        while let Some(event) = self.context.poll_event() {
            match event {
                ClientCoreEvent::UpdateHudMessage(message) => {
                    self.status.hud_message = message;
                }
                ClientCoreEvent::StreamingStarted(config) => {
                    info!("Streaming started");

                    self.status.streaming = true;
                    self.status.view_resolution = config.negotiated_config.view_resolution;
                    self.status.refresh_rate = config.negotiated_config.refresh_rate_hint;

                    self.start_tracking(config.negotiated_config.refresh_rate_hint);
                }
                ClientCoreEvent::StreamingStopped => {
                    info!("Streaming stopped");

                    self.status.streaming = false;
                    self.status.view_resolution = UVec2::ZERO;
                    self.status.refresh_rate = 0.0;
                    self.status.codec = None;

                    self.stop_tracking();
                }
                ClientCoreEvent::DecoderConfig { codec, .. } => {
                    // Recorded for display only. No decoder is created in this iteration, so the
                    // video stream is received and discarded by the client core.
                    self.status.codec = Some(codec);
                }
                ClientCoreEvent::Haptics { .. } | ClientCoreEvent::RealTimeConfig(_) => (),
            }
        }
    }

    /// Reports frame pacing to the server so its latency estimate and adaptive bitrate behave as
    /// they would with a real client. Without these the server has no pipeline latency to work with.
    pub fn report_frame(&mut self, frame_interval: Duration) {
        if !self.status.streaming {
            return;
        }

        self.context.report_compositor_start(self.current_frame);
        self.context.report_submit(self.current_frame, frame_interval);

        self.current_frame += frame_interval;
    }

    fn start_tracking(&mut self, refresh_rate: f32) {
        self.stop_tracking();
        self.streaming.set(true);

        // Local (head-relative) per-eye parameters. The server combines these with the head pose,
        // so only the lateral IPD offset and the FOV are needed here.
        self.context.send_view_params([
            ViewParams {
                pose: Pose {
                    orientation: alvr_common::glam::Quat::IDENTITY,
                    position: Vec3::new(-IPD / 2.0, 0.0, 0.0),
                },
                fov: Camera::fov(),
            },
            ViewParams {
                pose: Pose {
                    orientation: alvr_common::glam::Quat::IDENTITY,
                    position: Vec3::new(IPD / 2.0, 0.0, 0.0),
                },
                fov: Camera::fov(),
            },
        ]);

        let context = Arc::clone(&self.context);
        let streaming = Arc::clone(&self.streaming);
        let pose = Arc::clone(&self.pose);

        self.tracking_thread = Some(thread::spawn(move || {
            tracking_loop(context, streaming, pose, refresh_rate)
        }));
    }

    fn stop_tracking(&mut self) {
        self.streaming.set(false);

        if let Some(thread) = self.tracking_thread.take() {
            thread.join().ok();
        }
    }

    /// Shuts the connection down, and reports whether the client core can then be dropped safely.
    ///
    /// `pause()` stops streaming and lets the server observe the disconnect rather than timing the
    /// client out. It returns promptly, and the context is now safe to drop: `AnnouncerSocket` shuts
    /// its mdns-sd daemon down on drop, so the connection thread that owns it can finish and the
    /// join inside `ClientCoreContext::drop` completes. Before that fix this returned `false` and
    /// the caller had to leak the context, because the daemon left a thread parked forever in a
    /// blocking receive and the join never returned.
    #[must_use = "the caller must leak the context when this returns false"]
    pub fn shutdown(&mut self) -> bool {
        self.stop_tracking();
        self.context.pause();

        true
    }
}

/// Sends head poses at several times the display rate, as a real client does, so the server always
/// has fresh tracking to predict from.
fn tracking_loop(
    context: Arc<ClientCoreContext>,
    streaming: Arc<RelaxedAtomic>,
    pose: Arc<RwLock<TrackedPose>>,
    refresh_rate: f32,
) {
    let origin = Instant::now();
    let interval = Duration::from_secs_f32(1.0 / refresh_rate / 3.0);
    let mut deadline = Instant::now();

    while streaming.value() {
        let tracked = *pose.read();

        context.send_tracking(TrackingData {
            poll_timestamp: origin.elapsed(),
            device_motions: vec![(
                *HEAD_ID,
                DeviceMotion {
                    pose: Pose {
                        orientation: tracked.orientation,
                        position: tracked.position,
                    },
                    // The emulator camera is moved directly rather than simulated, so there is no
                    // meaningful velocity to report. The server falls back to pose extrapolation.
                    linear_velocity: Vec3::ZERO,
                    angular_velocity: Vec3::ZERO,
                },
            )],
            hand_skeletons: [None, None],
            face: FaceData::default(),
            body: None,
        });

        deadline += interval;
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
}
