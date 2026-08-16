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
    BUTTON_INFO, ButtonType, DeviceMotion, HAND_LEFT_ID, HAND_RIGHT_ID, HEAD_ID, Pose,
    RelaxedAtomic, ViewParams,
    error,
    glam::{UVec2, Vec3},
    info,
    parking_lot::{Mutex, RwLock},
};
use alvr_packets::{ButtonEntry, ButtonValue, FaceData, TrackingData};
use alvr_session::CodecType;
use std::{
    collections::{HashMap, HashSet},
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

/// World-space state of one emulated controller, as the tracking thread sends it.
#[derive(Clone, Copy)]
pub struct TrackedController {
    pub enabled: bool,
    pub motion: DeviceMotion,
}

impl Default for TrackedController {
    fn default() -> Self {
        Self {
            enabled: false,
            motion: DeviceMotion::IDENTITY,
        }
    }
}

/// Everything the tracking thread sends, written by the UI thread as one value.
///
/// One value on purpose: with the head and the controllers in separate slots, a tracking packet
/// could pair a fresh head pose with last frame's controller poses, which made head-relative
/// controllers visibly flicker against the view while the camera moved.
#[derive(Clone, Copy, Default)]
pub struct TrackedState {
    pub head: TrackedPose,
    /// Left is index 0.
    pub controllers: [TrackedController; 2],
}

/// A haptics pulse received from the server, for the UI to visualise.
#[derive(Clone, Copy)]
pub struct HapticsEvent {
    pub duration: Duration,
    pub frequency: f32,
    pub amplitude: f32,
}

pub struct EmulatedClient {
    context: Arc<ClientCoreContext>,
    status: ClientStatus,
    /// Shared with the tracking thread.
    tracking: Arc<RwLock<TrackedState>>,
    streaming: Arc<RelaxedAtomic>,
    tracking_thread: Option<JoinHandle<()>>,
    /// Timestamp of the frame currently being "presented", for the timing reports.
    current_frame: Duration,
    /// Created once the codec is known. Shared because the client core pushes NAL units from its
    /// own thread while the render thread drains decoded frames.
    decoder: Arc<Mutex<Option<Box<dyn VideoDecoder>>>>,
    /// Button values as last sent, so only changes go on the wire, as the real client does.
    sent_buttons: HashMap<u64, ButtonValue>,
    /// Input id set last announced via the active interaction profile packet.
    announced_inputs: Option<HashSet<u64>>,
    /// Latest haptics pulse per hand, held until the UI takes it. Left is index 0.
    haptics: [Option<HapticsEvent>; 2],
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
            tracking: Arc::new(RwLock::new(TrackedState::default())),
            streaming: Arc::new(RelaxedAtomic::new(false)),
            tracking_thread: None,
            current_frame: Duration::ZERO,
            decoder: Arc::new(Mutex::new(None)),
            sent_buttons: HashMap::new(),
            announced_inputs: None,
            haptics: [None; 2],
        }
    }

    pub fn status(&self) -> &ClientStatus {
        &self.status
    }

    /// Publishes the head pose and controller motions for the tracking thread to send.
    pub fn set_tracking(&self, state: TrackedState) {
        *self.tracking.write() = state;
    }

    /// Takes the pending haptics pulses, one slot per hand, for the UI to visualise.
    pub fn take_haptics(&mut self) -> [Option<HapticsEvent>; 2] {
        [self.haptics[0].take(), self.haptics[1].take()]
    }

    /// Sends the difference between `desired` and the previously sent button values.
    ///
    /// Mirrors the real client, which reports only inputs whose state changed since the last sync.
    /// Inputs that disappear from `desired` are released explicitly, so a caller only ever
    /// describes what is currently held.
    pub fn sync_buttons(&mut self, desired: &HashMap<u64, ButtonValue>) {
        if !self.status.streaming {
            return;
        }

        let mut entries = Vec::new();

        for (&path_id, &value) in desired {
            let already_sent = self
                .sent_buttons
                .get(&path_id)
                .is_some_and(|sent| button_values_equal(*sent, value));

            if !already_sent {
                entries.push(ButtonEntry { path_id, value });
            }
        }

        for &path_id in self.sent_buttons.keys() {
            if !desired.contains_key(&path_id)
                && let Some(info) = BUTTON_INFO.get(&path_id)
            {
                let value = match info.button_type {
                    ButtonType::Binary => ButtonValue::Binary(false),
                    ButtonType::Scalar => ButtonValue::Scalar(0.0),
                };

                entries.push(ButtonEntry { path_id, value });
            }
        }

        if !entries.is_empty() {
            self.context.send_buttons(entries);
        }

        self.sent_buttons = desired.clone();
    }

    /// Announces the set of inputs the emulated controllers can produce, when it changes.
    ///
    /// The server rebuilds its button mapping from each announcement and keeps only the latest,
    /// whichever hand it names, so both hands' inputs travel as one union set rather than one
    /// packet per hand. `profile_id` is transmitted for completeness but the server ignores it.
    pub fn sync_interaction_profile(&mut self, profile_id: u64, input_ids: HashSet<u64>) {
        if !self.status.streaming || input_ids.is_empty() {
            return;
        }

        if self.announced_inputs.as_ref() == Some(&input_ids) {
            return;
        }

        self.context
            .send_active_interaction_profile(*HAND_LEFT_ID, profile_id, input_ids.clone());

        self.announced_inputs = Some(input_ids);
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

                    // A new stream starts from a clean slate on the server, so the full button
                    // state and the interaction profile must go out again.
                    self.sent_buttons.clear();
                    self.announced_inputs = None;

                    self.start_tracking(config.negotiated_config.refresh_rate_hint);
                }
                ClientCoreEvent::StreamingStopped => {
                    info!("Streaming stopped");

                    self.status.streaming = false;
                    self.status.view_resolution = UVec2::ZERO;
                    self.status.refresh_rate = 0.0;
                    self.status.codec = None;

                    self.sent_buttons.clear();
                    self.announced_inputs = None;

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
                ClientCoreEvent::Haptics {
                    device_id,
                    duration,
                    frequency,
                    amplitude,
                } => {
                    // Any id other than the left hand's maps to the right, as the real client
                    // resolves it.
                    let hand = if device_id == *HAND_LEFT_ID { 0 } else { 1 };

                    self.haptics[hand] = Some(HapticsEvent {
                        duration,
                        frequency,
                        amplitude,
                    });
                }
                ClientCoreEvent::RealTimeConfig(_) => (),
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
        let tracking = Arc::clone(&self.tracking);

        self.tracking_thread = Some(thread::spawn(move || {
            tracking_loop(context, streaming, tracking, refresh_rate)
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
    tracking: Arc<RwLock<TrackedState>>,
    refresh_rate: f32,
) {
    let origin = Instant::now();
    let interval = Duration::from_secs_f32(1.0 / refresh_rate / 3.0);
    let mut deadline = Instant::now();

    while streaming.value() {
        let tracked = *tracking.read();

        let mut device_motions = vec![(
            *HEAD_ID,
            DeviceMotion {
                pose: Pose {
                    orientation: tracked.head.orientation,
                    position: tracked.head.position,
                },
                // The emulator camera is moved directly rather than simulated, so there is no
                // meaningful velocity to report. The server falls back to pose extrapolation.
                linear_velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
            },
        )];

        // A disabled controller is simply absent, which is also how a real client reports a
        // controller that is switched off or out of tracking.
        for (hand, controller) in tracked.controllers.iter().enumerate() {
            if controller.enabled {
                let device_id = if hand == 0 {
                    *HAND_LEFT_ID
                } else {
                    *HAND_RIGHT_ID
                };

                device_motions.push((device_id, controller.motion));
            }
        }

        context.send_tracking(TrackingData {
            poll_timestamp: origin.elapsed(),
            device_motions,
            hand_skeletons: [None, None],
            face: FaceData::default(),
            body: None,
        });

        deadline += interval;
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
}

/// `ButtonValue` compares by variant and payload; it does not implement `PartialEq` itself.
fn button_values_equal(a: ButtonValue, b: ButtonValue) -> bool {
    match (a, b) {
        (ButtonValue::Binary(a), ButtonValue::Binary(b)) => a == b,
        (ButtonValue::Scalar(a), ButtonValue::Scalar(b)) => a == b,
        _ => false,
    }
}
