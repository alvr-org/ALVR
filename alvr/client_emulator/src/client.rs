//! ALVR connection. Wraps `ClientCoreContext`, feeds it head poses from the emulator camera, and
//! routes the incoming video stream into a decoder.
//!
//! The decoder is created when the server announces the codec, and registering its input callback
//! is also what asks the server for the first keyframe.

use crate::{
    camera::{Camera, IPD},
    decoder::{self, DecodedFrame, DecoderKind, VideoDecoder},
};
use alvr_client_core::{ClientCapabilities, ClientCoreContext, ClientCoreEvent};
use alvr_common::{
    DeviceMotion, HEAD_ID, Pose, RelaxedAtomic, ViewParams,
    error,
    glam::{UVec2, Vec3},
    info,
    parking_lot::{Mutex, RwLock},
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
    /// Created once the codec is known. Shared because the client core pushes NAL units from its
    /// own thread while the render thread drains decoded frames.
    decoder: Arc<Mutex<Option<Box<dyn VideoDecoder>>>>,
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
            decoder: Arc::new(Mutex::new(None)),
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
                ClientCoreEvent::DecoderConfig { codec, config_nal } => {
                    // The decoder is created once, but the parameter sets are refreshed every time.
                    // The server repeats them whenever it is asked for a recovery keyframe, and that
                    // keyframe cannot be decoded without them: dropping the repeat is what makes
                    // corruption look permanent rather than clearing on the next keyframe.
                    if let Some(decoder) = self.decoder.lock().as_mut() {
                        decoder.set_config_nal(&config_nal);
                        continue;
                    }

                    self.status.codec = Some(codec);

                    // Reported straight through from the decoder, so decode timing is measured
                    // where it actually happens rather than when a frame reaches the screen.
                    let stats_context = Arc::clone(&self.context);
                    let on_frame_decoded = Box::new(move |timestamp| {
                        stats_context.report_frame_decoded(timestamp);
                    });

                    match decoder::create(
                        DecoderKind::preferred(),
                        codec,
                        &config_nal,
                        on_frame_decoded,
                    ) {
                        Ok(decoder) => {
                            *self.decoder.lock() = Some(decoder);

                            // Registering the callback also asks the server for an IDR frame, which
                            // is required: without a keyframe the decoder has nothing to start from
                            // and the stream silently stays black.
                            let decoder_slot = Arc::clone(&self.decoder);
                            self.context
                                .set_decoder_input_callback(Box::new(move |timestamp, nal| {
                                    let mut lock = decoder_slot.lock();

                                    // Returning false tells the client core the frame was not
                                    // queued, so it can account for the drop.
                                    lock.as_mut()
                                        .is_some_and(|decoder| decoder.push_nal(timestamp, nal))
                                }));
                        }
                        Err(e) => error!("Cannot create video decoder: {e}"),
                    }
                }
                ClientCoreEvent::Haptics { .. } | ClientCoreEvent::RealTimeConfig(_) => (),
            }
        }
    }

    /// Takes the most recent decoded frame, discarding any older ones still queued.
    ///
    /// Showing the newest frame rather than the oldest keeps the view current when rendering falls
    /// behind decoding, which matters more for a debugging view than displaying every frame.
    pub fn take_latest_frame(&mut self) -> Option<DecodedFrame> {
        let mut latest = None;

        {
            let mut lock = self.decoder.lock();
            let decoder = lock.as_mut()?;

            while let Some(frame) = decoder.poll_frame() {
                latest = Some(frame);
            }
        }

        latest
    }

    /// Reports frame pacing to the server, once per newly decoded frame.
    ///
    /// This mirrors `client_openxr`: `report_compositor_start` and `report_submit` are called only
    /// when a new frame was obtained from the decoder, never with a repeated timestamp, because the
    /// statistics are keyed by that timestamp and a repeat matches an entry that has already been
    /// accounted for.
    ///
    /// These reports feed the server's bitrate adaptation and the dashboard graphs. They do not gate
    /// frame production: the server paces itself on a fixed timer derived from the negotiated
    /// framerate, so missing reports degrade adaptation rather than stopping the video.
    pub fn report_frame(&mut self, displayed_frame: Option<Duration>) {
        if !self.status.streaming {
            return;
        }

        let Some(timestamp) = displayed_frame else {
            return;
        };

        if timestamp == self.current_frame {
            return;
        }
        self.current_frame = timestamp;

        let interval = if self.status.refresh_rate > 0.0 {
            Duration::from_secs_f32(1.0 / self.status.refresh_rate)
        } else {
            Duration::from_millis(16)
        };

        self.context.report_compositor_start(timestamp);
        // Stands in for the wait until the next vsync, which a real compositor measures and this
        // emulator cannot know.
        self.context.report_submit(timestamp, interval);
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
