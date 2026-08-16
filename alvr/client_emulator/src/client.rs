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
    glam::{Quat, UVec2, Vec3},
    info,
    parking_lot::{Mutex, RwLock},
};
use alvr_packets::{ButtonEntry, ButtonValue, FaceData, TrackingData};
use alvr_session::CodecType;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Default view resolution advertised to the server, matching a Quest-class headset.
const DEFAULT_VIEW_RESOLUTION: UVec2 = UVec2::new(1920, 1832);

/// Tracking packets go out at this multiple of the stream refresh rate, matching `client_openxr`.
///
/// It looks like it should pay to send faster. ALVR's driver never submits an HMD velocity
/// (`HMD.cpp` fills in `qRotation` and `vecPosition` and leaves the zero-initialised `vecVelocity`
/// and `poseTimeOffset` alone), so SteamVR cannot extrapolate the head at all: every frame is
/// rendered with whichever tracking packet arrived last, held until the next one. The send
/// interval is therefore the time quantum of the rendered world — 4.6 ms of a 13.9 ms frame here.
///
/// Raising it to 500 Hz was measured and is **worse**: per-frame rotation jitter went from 0.155°
/// to 0.205° at a 60 °/s turn, consistently across windows, and the frame timestamps themselves
/// spread from ±0.10 ms to ±0.31 ms. The server runs its whole tracking path per received packet —
/// receive, settings lookup, `SetTracking`, three `TrackedDevicePoseUpdated` calls — and the extra
/// load disturbs its own frame timing by more than the finer quantum recovers. Do not raise this
/// without measuring; see the frame timing readout.
const TRACKING_RATE_MULTIPLIER: f32 = 3.0;

/// How many published states the reconstruction keeps; see [`TrackingWindow`].
const PUBLISH_HISTORY: usize = 8;

/// Bounds on the reconstruction lag, which is otherwise derived from the measured publish jitter.
const MIN_LAG: Duration = Duration::from_millis(2);
const MAX_LAG: Duration = Duration::from_millis(40);

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

/// State of one emulated controller, as the tracking thread sends it.
///
/// The pose is **head-relative**, and the tracking thread composes it with the head pose it is
/// sending in the same packet. Publishing world-space poses instead — as this did — means the
/// reconstruction blends the head and the controller as two independent world-space paths, so a
/// controller held still in the hand traces a chord while the head rotates along an arc. Keeping
/// the pose in head space makes the two exactly rigid at any blend factor, including when the
/// reconstruction extrapolates past the newest publish.
#[derive(Clone, Copy)]
pub struct TrackedController {
    pub enabled: bool,
    /// Relative to the head pose in the same [`TrackedState`].
    pub pose: Pose,
}

impl Default for TrackedController {
    fn default() -> Self {
        Self {
            enabled: false,
            pose: Pose::IDENTITY,
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

/// The recently published states with the instants they were published at, so the tracking thread
/// can reconstruct the pose at any moment they cover.
///
/// The UI publishes poses once per frame while the tracking thread sends several times faster, so
/// sending the latest state verbatim produces a stair-step signal: the same pose repeated, then a
/// jump. Reading the path back at a fixed lag turns it into the continuous signal the camera
/// actually described.
///
/// It keeps a *history* rather than the last two samples because the UI frame interval is nowhere
/// near regular — measured at 8.3 ms with 4.3 ms of mean deviation, so intervals run from about 4
/// to 17 ms. With only two samples the evaluation point falls outside them whenever a frame runs
/// long or short, and the reconstruction has to clamp: it freezes on one endpoint and then jumps,
/// modulating the apparent speed of the whole world at the UI frame rate. With enough history the
/// evaluation point lands inside a real segment and the path comes back exactly.
struct TrackingWindow {
    /// Newest last.
    samples: VecDeque<(Instant, TrackedState)>,
    /// Recent publish intervals in milliseconds, which set the lag and drive the readout.
    intervals: Samples,
}

impl Default for TrackingWindow {
    fn default() -> Self {
        Self {
            samples: VecDeque::from(vec![(Instant::now(), TrackedState::default())]),
            intervals: Samples::default(),
        }
    }
}

impl TrackingWindow {
    fn publish(&mut self, state: TrackedState, at: Instant) {
        if let Some((last_at, _)) = self.samples.back() {
            let interval = at.saturating_duration_since(*last_at).as_secs_f32() * 1000.0;

            self.intervals.push(interval.clamp(0.1, 100.0));
        }

        self.samples.push_back((at, state));

        while self.samples.len() > PUBLISH_HISTORY {
            self.samples.pop_front();
        }
    }

    /// How far behind the newest publish to read the path.
    ///
    /// Sized from the *spread* of the publish interval, not just its average: the point is to sit
    /// far enough back that a long UI frame still leaves the evaluation point inside a segment
    /// that has both ends. Every millimetre of this is latency, so it tracks the measured jitter
    /// rather than being set to a safe constant.
    fn lag(&self) -> Duration {
        let interval = self.intervals.stat();

        Duration::from_secs_f32((interval.mean + interval.deviation * 2.0) / 1000.0)
            .clamp(MIN_LAG, MAX_LAG)
    }

    /// Reconstructs the published path at `at`.
    fn sample(&self, at: Instant) -> TrackedState {
        let Some(&(newest_at, newest)) = self.samples.back() else {
            return TrackedState::default();
        };

        // Past everything published: the UI stalled for longer than the lag allows for. Hold,
        // rather than extrapolate towards a point that has not been decided yet — guessing here is
        // how a stall turns into an overshoot and then a visible snap back.
        if at >= newest_at {
            return newest;
        }

        for (&(previous_at, previous), &(current_at, current)) in
            self.samples.iter().zip(self.samples.iter().skip(1))
        {
            if at < current_at {
                let span = current_at.saturating_duration_since(previous_at);
                let elapsed = at.saturating_duration_since(previous_at);
                let alpha = if span.is_zero() {
                    1.0
                } else {
                    elapsed.as_secs_f32() / span.as_secs_f32()
                };

                return interpolate_state(&previous, &current, alpha.clamp(0.0, 1.0));
            }
        }

        // Older than anything retained, which needs the history to be shorter than the lag spans.
        self.samples
            .front()
            .map(|(_, state)| *state)
            .unwrap_or_default()
    }
}

/// A mean and its mean absolute deviation, over a rolling window of samples.
///
/// Mean absolute deviation rather than standard deviation because one dropped frame should not
/// dominate a number being read off a toolbar.
#[derive(Serialize, Clone, Copy, Default)]
pub struct Stat {
    pub mean: f32,
    pub deviation: f32,
}

/// Rolling window of samples, in whatever unit the caller pushes.
#[derive(Default)]
struct Samples(VecDeque<f32>);

/// About two seconds at stream rate: long enough to average out, short enough to react.
const TIMING_WINDOW: usize = 150;

impl Samples {
    fn push(&mut self, sample: f32) {
        // A non-finite sample would poison the mean for the whole window.
        self.0.push_back(if sample.is_finite() { sample } else { 0.0 });

        if self.0.len() > TIMING_WINDOW {
            self.0.pop_front();
        }
    }

    fn stat(&self) -> Stat {
        if self.0.is_empty() {
            return Stat::default();
        }

        let inverse_count = 1.0 / self.0.len() as f32;
        let mean = self.0.iter().sum::<f32>() * inverse_count;
        let deviation = self.0.iter().map(|sample| (sample - mean).abs()).sum::<f32>();

        Stat {
            mean,
            deviation: deviation * inverse_count,
        }
    }
}

/// How evenly the streamed world is advancing, measured entirely from the client side.
///
/// Every displayed frame carries the timestamp of the tracking sample the server rendered it from,
/// and `report_compositor_start` returns that frame's head pose, so the emulator can measure both
/// how much *world time* each frame advanced by and how far the head actually moved. Under steady
/// motion those should be constant; the deviations are the judder, split by where it came from.
///
/// The split is the useful part. `sent_*` is measured on packets leaving, everything else on
/// frames coming back — an even step going out with an uneven one coming back means the signal was
/// clean when it left and something downstream resampled it.
#[derive(Serialize, Clone, Copy, Default)]
pub struct FrameTiming {
    /// Interval at which the UI publishes poses for the tracking thread, in milliseconds. This is
    /// what the reconstruction has to absorb, and what sizes its lag.
    pub publish_ms: Stat,
    /// Interval between outgoing tracking packets, in milliseconds.
    pub sent_ms: Stat,
    /// Head rotation between consecutive outgoing packets, in degrees.
    pub sent_step_deg: Stat,
    /// Tracking time between consecutive displayed frames, in milliseconds.
    pub world_ms: Stat,
    /// Head rotation between consecutive displayed frames, in degrees. **This is the judder the
    /// eye sees while turning**: during a steady turn every frame should advance by the same
    /// angle, and whatever it varies by is scene movement with nothing behind it.
    pub step_deg: Stat,
    /// Head translation between consecutive displayed frames, in millimetres.
    pub step_mm: Stat,
    /// Real time each displayed frame was on screen, in milliseconds.
    pub screen_ms: Stat,
    /// Fraction of displayed frames whose head pose came back bit-identical to the previous
    /// frame's. While the camera is moving the server cannot legitimately render two frames from
    /// the same pose, so anything above zero means `report_compositor_start` failed to find the
    /// frame's view parameters and handed back the last ones it did find.
    pub repeated_view_ratio: f32,
}

/// Rolling window over the frames the emulator displayed.
#[derive(Default)]
struct FrameTimingTracker {
    previous: Option<(Duration, Instant, Pose)>,
    world: Samples,
    screen: Samples,
    rotation: Samples,
    translation: Samples,
    repeated: Samples,
}

impl FrameTimingTracker {
    fn record(&mut self, timestamp: Duration, at: Instant, head: Pose) {
        if let Some((previous_timestamp, previous_at, previous_head)) = self.previous {
            // Frames are stamped with the client's own monotonic tracking clock, so a later frame
            // always has a later timestamp; going backwards means a reconnect, which resets this.
            if let Some(world) = timestamp.checked_sub(previous_timestamp) {
                self.world.push(world.as_secs_f32() * 1000.0);
                self.screen
                    .push(at.saturating_duration_since(previous_at).as_secs_f32() * 1000.0);
                self.rotation.push(
                    previous_head
                        .orientation
                        .angle_between(head.orientation)
                        .to_degrees(),
                );
                self.translation
                    .push(previous_head.position.distance(head.position) * 1000.0);

                let repeated = previous_head.orientation == head.orientation
                    && previous_head.position == head.position;
                self.repeated.push(if repeated { 1.0 } else { 0.0 });
            }
        }

        self.previous = Some((timestamp, at, head));
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Rolling window over the tracking packets the sending thread put on the wire.
///
/// Sampled on the send side deliberately: everything else is measured on frames coming back, which
/// cannot tell a signal that left badly formed apart from one that was mishandled later.
#[derive(Default)]
struct SendTracker {
    previous: Option<(Instant, Quat)>,
    intervals: Samples,
    rotation: Samples,
}

impl SendTracker {
    fn record(&mut self, at: Instant, orientation: Quat) {
        if let Some((previous_at, previous_orientation)) = self.previous {
            self.intervals
                .push(at.saturating_duration_since(previous_at).as_secs_f32() * 1000.0);
            self.rotation
                .push(previous_orientation.angle_between(orientation).to_degrees());
        }

        self.previous = Some((at, orientation));
    }
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
    tracking: Arc<RwLock<TrackingWindow>>,
    streaming: Arc<RelaxedAtomic>,
    tracking_thread: Option<JoinHandle<()>>,
    /// Timestamp of the frame currently being "presented", for the timing reports.
    current_frame: Duration,
    /// World-space eye poses the currently displayed frame was rendered with.
    displayed_views: Option<[ViewParams; 2]>,
    /// Whether the currently displayed frame still owes the server its submit report.
    pending_submit: bool,
    /// How evenly displayed frames have been advancing the world, for the toolbar readout.
    frame_timing: FrameTimingTracker,
    /// The same, measured on the outgoing tracking packets. Shared with the tracking thread.
    send_timing: Arc<Mutex<SendTracker>>,
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
            tracking: Arc::new(RwLock::new(TrackingWindow::default())),
            streaming: Arc::new(RelaxedAtomic::new(false)),
            tracking_thread: None,
            current_frame: Duration::ZERO,
            displayed_views: None,
            pending_submit: false,
            frame_timing: FrameTimingTracker::default(),
            send_timing: Arc::new(Mutex::new(SendTracker::default())),
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
        self.tracking.write().publish(state, Instant::now());
    }

    /// How evenly the world has been advancing over the last couple of seconds of video.
    ///
    /// Assembled from three trackers because the three stages run on different threads: the UI
    /// publishes, the tracking thread sends, and the render thread displays.
    pub fn frame_timing(&self) -> FrameTiming {
        let displayed = &self.frame_timing;
        let sent = self.send_timing.lock();

        FrameTiming {
            publish_ms: self.tracking.read().intervals.stat(),
            sent_ms: sent.intervals.stat(),
            sent_step_deg: sent.rotation.stat(),
            world_ms: displayed.world.stat(),
            step_deg: displayed.rotation.stat(),
            step_mm: displayed.translation.stat(),
            screen_ms: displayed.screen.stat(),
            repeated_view_ratio: displayed.repeated.stat().mean,
        }
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
                    self.displayed_views = None;
                    self.pending_submit = false;
                    self.frame_timing.reset();

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

    /// Reports which frame is being presented this repaint, and returns the world-space eye poses
    /// that frame was rendered with — for drawing overlays in lockstep with the video content.
    ///
    /// This mirrors `client_openxr`: `report_compositor_start` is called only when a new frame was
    /// obtained from the decoder, never with a repeated timestamp, because the statistics are
    /// keyed by that timestamp and a repeat matches an entry that has already been accounted for.
    ///
    /// These reports feed the server's bitrate adaptation and the dashboard graphs. They do not
    /// gate frame production: the server paces itself on a fixed timer derived from the negotiated
    /// framerate, so missing reports degrade adaptation rather than stopping the video.
    pub fn displayed_frame_views(
        &mut self,
        displayed_frame: Option<Duration>,
    ) -> Option<[ViewParams; 2]> {
        if !self.status.streaming {
            return None;
        }

        let timestamp = displayed_frame?;

        if timestamp != self.current_frame {
            self.current_frame = timestamp;

            let views = self.context.report_compositor_start(timestamp);
            self.displayed_views = Some(views);
            self.pending_submit = true;

            // The head pose the server rendered this frame from, midway between the eyes.
            self.frame_timing.record(
                timestamp,
                Instant::now(),
                Pose {
                    orientation: views[0].pose.orientation,
                    position: (views[0].pose.position + views[1].pose.position) / 2.0,
                },
            );
        }

        self.displayed_views
    }

    /// Completes the pacing report begun by [`Self::displayed_frame_views`]. Called at the end of
    /// the repaint, once the frame has actually been handed to the screen.
    pub fn finish_frame(&mut self) {
        if !self.pending_submit || !self.status.streaming {
            return;
        }
        self.pending_submit = false;

        let interval = if self.status.refresh_rate > 0.0 {
            Duration::from_secs_f32(1.0 / self.status.refresh_rate)
        } else {
            Duration::from_millis(16)
        };

        // Stands in for the wait until the next vsync, which a real compositor measures and this
        // emulator cannot know.
        self.context.report_submit(self.current_frame, interval);
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
        let send_timing = Arc::clone(&self.send_timing);

        *send_timing.lock() = SendTracker::default();

        self.tracking_thread = Some(thread::spawn(move || {
            tracking_loop(context, streaming, tracking, send_timing, refresh_rate)
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
/// has fresh tracking to render from; see [`TRACKING_RATE_MULTIPLIER`].
fn tracking_loop(
    context: Arc<ClientCoreContext>,
    streaming: Arc<RelaxedAtomic>,
    tracking: Arc<RwLock<TrackingWindow>>,
    send_timing: Arc<Mutex<SendTracker>>,
    refresh_rate: f32,
) {
    let origin = Instant::now();
    let interval = Duration::from_secs_f32(1.0 / (refresh_rate * TRACKING_RATE_MULTIPLIER));
    let mut deadline = Instant::now();

    while streaming.value() {
        // Read back at a lag behind the UI, so consecutive packets describe the camera's actual
        // continuous path rather than stair-stepping at the UI frame rate; see [`TrackingWindow`].
        let tracked = {
            let window = tracking.read();
            let at = Instant::now() - window.lag();

            window.sample(at)
        };

        let head_pose = Pose {
            orientation: tracked.head.orientation,
            position: tracked.head.position,
        };

        send_timing
            .lock()
            .record(Instant::now(), head_pose.orientation);

        let mut device_motions = vec![(
            *HEAD_ID,
            DeviceMotion {
                pose: head_pose,
                // Zero on purpose, controllers too, and the asymmetry in the driver is why.
                // `Hmd::OnPoseUpdated` submits no velocity at all, so SteamVR can never
                // extrapolate the head; `Controller::OnPoseUpdate` submits velocity *and* a
                // `poseTimeOffset` of `steamvr_pipeline_frames` frames, so SteamVR does
                // extrapolate the controllers, by an amount that depends on its own fluctuating
                // time-to-photon estimate. Any nonzero controller velocity therefore makes them
                // swim against a head that is physically unable to follow. With every velocity
                // zero nothing is extrapolated anywhere and the rig is exactly the sent poses,
                // which measured perfectly rigid. Reintroducing velocities means fixing the
                // driver's head path first.
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

                // Composed here rather than by the UI, so the controller is rigid against exactly
                // the head pose going out in this packet; see [`TrackedController`].
                device_motions.push((
                    device_id,
                    DeviceMotion {
                        pose: head_pose * controller.pose,
                        linear_velocity: Vec3::ZERO,
                        angular_velocity: Vec3::ZERO,
                    },
                ));
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

        // A missed deadline must not turn into a burst of catch-up packets, which would put two
        // poses on the wire microseconds apart and leave a full gap after them — the opposite of
        // what the rate is for.
        let now = Instant::now();
        if deadline < now {
            deadline = now + interval;
        }

        // Plain sleep, deliberately. Spinning the last 300 µs of each interval does tighten the
        // send pacing — measured 0.155 ms of deviation down to 0.086 ms — but the judder that
        // actually reaches the screen did not improve, and came out marginally worse. Same lesson
        // as the send rate above: precision on this side is not what the picture is limited by,
        // and it is not worth a busy-wait.
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
}

/// Blends between two published tracking states.
///
/// Every quantity uses the same factor, so the head and the controllers stay consistent with each
/// other. The controller poses are head-relative and usually identical between the two states, in
/// which case they come through untouched and the controllers are exactly rigid against whatever
/// the head does.
fn interpolate_state(previous: &TrackedState, current: &TrackedState, alpha: f32) -> TrackedState {
    TrackedState {
        head: TrackedPose {
            position: previous.head.position.lerp(current.head.position, alpha),
            orientation: previous
                .head
                .orientation
                .slerp(current.head.orientation, alpha),
        },
        controllers: std::array::from_fn(|hand| {
            let old = &previous.controllers[hand];
            let new = &current.controllers[hand];

            // A controller that just appeared has no old pose to blend from; blending with the
            // placeholder would sweep it in from the head's origin.
            if !(old.enabled && new.enabled) {
                return *new;
            }

            TrackedController {
                enabled: true,
                pose: Pose {
                    orientation: old.pose.orientation.slerp(new.pose.orientation, alpha),
                    position: old.pose.position.lerp(new.pose.position, alpha),
                },
            }
        }),
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
