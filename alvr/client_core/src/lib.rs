#![allow(
    non_upper_case_globals,
    non_snake_case,
    clippy::missing_safety_doc,
    clippy::unseparated_literal_suffix
)]

mod c_api;
mod connection;
mod logging_backend;
mod sockets;
mod statistics;
mod storage;

#[cfg(target_os = "android")]
mod audio;

pub mod video_decoder;

use alvr_common::{
    ConnectionState, DeviceMotion, Fov, HEAD_ID, LifecycleState, Pose, ViewParams, dbg_client_core,
    error,
    glam::{Quat, UVec2, Vec2, Vec3},
    parking_lot::{Mutex, RwLock},
    warn,
};
use alvr_packets::{
    BatteryInfo, ButtonEntry, ClientControlPacket, FaceData, RealTimeConfig,
    ReservedClientControlPacket, StreamConfig, Tracking, ViewsConfig,
};
use alvr_session::CodecType;
use connection::{ConnectionContext, DecoderCallback};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};
use storage::Config;

pub use logging_backend::init_logging;

pub enum ClientCoreEvent {
    UpdateHudMessage(String),
    StreamingStarted(Box<StreamConfig>),
    StreamingStopped,
    Haptics {
        device_id: u64,
        duration: Duration,
        frequency: f32,
        amplitude: f32,
    },
    // Note: All subsequent DecoderConfig events should be ignored until reconnection
    DecoderConfig {
        codec: CodecType,
        config_nal: Vec<u8>,
    },
    RealTimeConfig(RealTimeConfig),
}

// Note: this struct may change without breaking network protocol changes
#[derive(Clone)]
pub struct ClientCapabilities {
    pub default_view_resolution: UVec2,
    pub refresh_rates: Vec<f32>,
    pub foveated_encoding: bool,
    pub encoder_high_profile: bool,
    pub encoder_10_bits: bool,
    pub encoder_av1: bool,
    pub prefer_10bit: bool,
    pub prefer_full_range: bool,
    pub preferred_encoding_gamma: f32,
    pub prefer_hdr: bool,
}

pub struct ClientCoreContext {
    lifecycle_state: Arc<RwLock<LifecycleState>>,
    event_queue: Arc<Mutex<VecDeque<ClientCoreEvent>>>,
    connection_context: Arc<ConnectionContext>,
    connection_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ClientCoreContext {
    pub fn new(capabilities: ClientCapabilities) -> Self {
        dbg_client_core!("Create");

        // Make sure to reset config in case of version compat mismatch.
        if Config::load().protocol_id != alvr_common::protocol_id() {
            // NB: Config::default() sets the current protocol ID
            Config::default().store();
        }

        #[cfg(target_os = "android")]
        {
            dbg_client_core!("Getting permissions");
            alvr_system_info::try_get_permission(alvr_system_info::MICROPHONE_PERMISSION);
            alvr_system_info::set_wifi_lock(true);
        }

        let lifecycle_state = Arc::new(RwLock::new(LifecycleState::Idle));
        let event_queue = Arc::new(Mutex::new(VecDeque::new()));
        let connection_context = Arc::new(ConnectionContext::default());
        let connection_thread = thread::spawn({
            let lifecycle_state = Arc::clone(&lifecycle_state);
            let connection_context = Arc::clone(&connection_context);
            let event_queue = Arc::clone(&event_queue);
            move || {
                connection::connection_lifecycle_loop(
                    capabilities,
                    connection_context,
                    lifecycle_state,
                    event_queue,
                )
            }
        });

        Self {
            lifecycle_state,
            event_queue,
            connection_context,
            connection_thread: Arc::new(Mutex::new(Some(connection_thread))),
        }
    }

    pub fn resume(&self) {
        dbg_client_core!("resume");

        *self.lifecycle_state.write() = LifecycleState::Resumed;
    }

    pub fn pause(&self) {
        dbg_client_core!("pause");

        let mut connection_state_lock = self.connection_context.state.write();

        *self.lifecycle_state.write() = LifecycleState::Idle;

        // We want to shutdown streaming when pausing.
        if *connection_state_lock != ConnectionState::Disconnected {
            alvr_common::wait_rwlock(
                &self.connection_context.disconnected_notif,
                &mut connection_state_lock,
            );
        }
    }

    pub fn poll_event(&self) -> Option<ClientCoreEvent> {
        dbg_client_core!("poll_event");

        self.event_queue.lock().pop_front()
    }

    pub fn send_battery(&self, device_id: u64, gauge_value: f32, is_plugged: bool) {
        dbg_client_core!("send_battery");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::Battery(BatteryInfo {
                    device_id,
                    gauge_value,
                    is_plugged,
                }))
                .ok();
        }
    }

    pub fn send_playspace(&self, area: Option<Vec2>) {
        dbg_client_core!("send_playspace");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::PlayspaceSync(area)).ok();
        }
    }

    pub fn send_active_interaction_profile(&self, device_id: u64, profile_id: u64) {
        dbg_client_core!("send_active_interaction_profile");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::ActiveInteractionProfile {
                    device_id,
                    profile_id,
                })
                .ok();
        }
    }

    pub fn send_custom_interaction_profile(&self, device_id: u64, input_ids: HashSet<u64>) {
        dbg_client_core!("send_custom_interaction_profile");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&alvr_packets::encode_reserved_client_control_packet(
                    &ReservedClientControlPacket::CustomInteractionProfile {
                        device_id,
                        input_ids,
                    },
                ))
                .ok();
        }
    }

    pub fn send_buttons(&self, entries: Vec<ButtonEntry>) {
        dbg_client_core!("send_buttons");

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::Buttons(entries)).ok();
        }
    }

    // These must be in its local space, as if the head pose is in the origin.
    pub fn send_view_params(&self, views: [ViewParams; 2]) {
        dbg_client_core!("send_view_params");

        // TODO(shinyquagsire23): Make this a configurable slider.
        let comfort = 1.0;

        // HACK: OpenVR for various reasons expects orthogonal view transforms, so we
        // toss out the orientation and fix the FoVs if applicable.
        let views_openvr = [
            canted_view_to_proportional_circumscribed_orthogonal(views[0], comfort),
            canted_view_to_proportional_circumscribed_orthogonal(views[1], comfort),
        ];

        *self.connection_context.view_params.write() = views_openvr;

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender
                .send(&ClientControlPacket::ViewsConfig(ViewsConfig {
                    fov: [views_openvr[0].fov, views_openvr[1].fov],
                    ipd_m: (views_openvr[0].pose.position - views_openvr[1].pose.position).length(),
                }))
                .ok();
        }
    }

    pub fn send_tracking(
        &self,
        poll_timestamp: Duration,
        mut device_motions: Vec<(u64, DeviceMotion)>,
        hand_skeletons: [Option<[Pose; 26]>; 2],
        face_data: FaceData,
    ) {
        dbg_client_core!("send_tracking");

        let max_prediction = *self.connection_context.max_prediction.read();

        let target_timestamp = if let Some(stats) =
            &*self.connection_context.statistics_manager.lock()
        {
            poll_timestamp + Duration::min(stats.average_total_pipeline_latency(), max_prediction)
        } else {
            poll_timestamp
        };

        // Guarantee that sent timestamps never go backwards by sending the poll time
        let reported_timestamp = poll_timestamp;

        for (id, motion) in &mut device_motions {
            let velocity_multiplier = *self.connection_context.velocities_multiplier.read();
            motion.linear_velocity *= velocity_multiplier;
            motion.angular_velocity *= velocity_multiplier;

            if *id == *HEAD_ID {
                *motion = motion.predict(poll_timestamp, target_timestamp);

                let mut head_pose_queue = self.connection_context.head_pose_queue.write();

                head_pose_queue.push_back((reported_timestamp, motion.pose));

                while head_pose_queue.len() > 1024 {
                    head_pose_queue.pop_front();
                }

                // This is done for backward compatibiity for the v20 protocol. Will be removed with the
                // tracking rewrite protocol extension.
                motion.linear_velocity = Vec3::ZERO;
                motion.angular_velocity = Vec3::ZERO;
            } else if let Some(stats) = &*self.connection_context.statistics_manager.lock() {
                let tracker_timestamp = poll_timestamp
                    + Duration::min(stats.tracker_prediction_offset(), max_prediction);

                *motion = motion.predict(poll_timestamp, tracker_timestamp);
            }
        }

        if let Some(sender) = &mut *self.connection_context.tracking_sender.lock() {
            sender
                .send_header(&Tracking {
                    target_timestamp: reported_timestamp,
                    device_motions,
                    hand_skeletons,
                    face_data,
                })
                .ok();

            if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
                stats.report_input_acquired(reported_timestamp);
            }
        }
    }

    pub fn get_total_prediction_offset(&self) -> Duration {
        dbg_client_core!("get_total_prediction_offset");

        if let Some(stats) = &*self.connection_context.statistics_manager.lock() {
            stats.average_total_pipeline_latency()
        } else {
            Duration::ZERO
        }
    }

    /// The callback should return true if the frame was successfully submitted to the decoder
    pub fn set_decoder_input_callback(&self, callback: Box<DecoderCallback>) {
        dbg_client_core!("set_decoder_input_callback");

        *self.connection_context.decoder_callback.lock() = Some(callback);

        if let Some(sender) = &mut *self.connection_context.control_sender.lock() {
            sender.send(&ClientControlPacket::RequestIdr).ok();
        }
    }

    pub fn report_frame_decoded(&self, timestamp: Duration) {
        dbg_client_core!("report_frame_decoded");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_frame_decoded(timestamp);
        }
    }

    pub fn report_fatal_decoder_error(&self, error: &str) {
        error!("Fatal decoder error, restarting connection: {error}");

        // The connection loop observes changes on this value
        *self.connection_context.state.write() = ConnectionState::Disconnecting;
    }

    pub fn report_compositor_start(&self, timestamp: Duration) -> [ViewParams; 2] {
        dbg_client_core!("report_compositor_start");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_compositor_start(timestamp);
        }

        let mut head_pose = *self.connection_context.last_good_head_pose.read();
        for (ts, pose) in &*self.connection_context.head_pose_queue.read() {
            if *ts == timestamp {
                head_pose = *pose;
                break;
            }
        }
        let view_params = self.connection_context.view_params.read();

        [
            ViewParams {
                pose: head_pose * view_params[0].pose,
                fov: view_params[0].fov,
            },
            ViewParams {
                pose: head_pose * view_params[1].pose,
                fov: view_params[1].fov,
            },
        ]
    }

    pub fn report_submit(&self, timestamp: Duration, vsync_queue: Duration) {
        dbg_client_core!("report_submit");

        if let Some(stats) = &mut *self.connection_context.statistics_manager.lock() {
            stats.report_submit(timestamp, vsync_queue);

            if let Some(sender) = &mut *self.connection_context.statistics_sender.lock() {
                if let Some(stats) = stats.summary(timestamp) {
                    sender.send_header(&stats).ok();
                } else {
                    warn!("Statistics summary not ready!");
                }
            }
        }
    }
}

impl Drop for ClientCoreContext {
    fn drop(&mut self) {
        dbg_client_core!("Drop");

        *self.lifecycle_state.write() = LifecycleState::ShuttingDown;

        if let Some(thread) = self.connection_thread.lock().take() {
            thread.join().ok();
        }

        #[cfg(target_os = "android")]
        alvr_system_info::set_wifi_lock(false);
    }
}

// Calculates a view transform which is orthogonal (with no rotational component),
// with the same aspect ratio, and can inscribe the rotated view transform inside itself.
// Useful for converting canted transforms to ones compatible with SteamVR and legacy runtimes.
pub fn canted_view_to_proportional_circumscribed_orthogonal(
    view_canted: ViewParams,
    fov_post_scale: f32,
) -> ViewParams {
    let viewpose_orth = Pose {
        orientation: Quat::IDENTITY,
        position: view_canted.pose.position,
    };

    // Calculate unit vectors for the corner of the view space
    let v0 = Vec3::new(view_canted.fov.left, view_canted.fov.down, -1.0);
    let v1 = Vec3::new(view_canted.fov.right, view_canted.fov.down, -1.0);
    let v2 = Vec3::new(view_canted.fov.right, view_canted.fov.up, -1.0);
    let v3 = Vec3::new(view_canted.fov.left, view_canted.fov.up, -1.0);

    // Our four corners in world space
    let w0 = view_canted.pose.orientation * v0;
    let w1 = view_canted.pose.orientation * v1;
    let w2 = view_canted.pose.orientation * v2;
    let w3 = view_canted.pose.orientation * v3;

    // Project into 2D space
    let pt0 = Vec2::new(w0.x * (-1.0 / w0.z), w0.y * (-1.0 / w0.z));
    let pt1 = Vec2::new(w1.x * (-1.0 / w1.z), w1.y * (-1.0 / w1.z));
    let pt2 = Vec2::new(w2.x * (-1.0 / w2.z), w2.y * (-1.0 / w2.z));
    let pt3 = Vec2::new(w3.x * (-1.0 / w3.z), w3.y * (-1.0 / w3.z));

    // Find the minimum/maximum point values for our new frustum
    let pts_x = [pt0.x, pt1.x, pt2.x, pt3.x];
    let pts_y = [pt0.y, pt1.y, pt2.y, pt3.y];
    let inscribed_left = pts_x.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let inscribed_right = pts_x.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let inscribed_up = pts_y.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let inscribed_down = pts_y.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    let fov_orth = Fov {
        left: inscribed_left,
        right: inscribed_right,
        up: inscribed_up,
        down: inscribed_down,
    };

    // Last step: Preserve the aspect ratio, so that we don't have to deal with non-square pixel issues.
    let fov_orth_width = fov_orth.right.abs() + fov_orth.left.abs();
    let fov_orth_height = fov_orth.up.abs() + fov_orth.down.abs();
    let fov_orig_width = view_canted.fov.right.abs() + view_canted.fov.left.abs();
    let fov_orig_height = view_canted.fov.up.abs() + view_canted.fov.down.abs();
    let scales = [
        fov_orth_width / fov_orig_width,
        fov_orth_height / fov_orig_height,
    ];

    let fov_inscribe_scale = scales
        .iter()
        .fold(f32::NEG_INFINITY, |a, &b| a.max(b))
        .max(1.0);
    let fov_orth_corrected = Fov {
        left: view_canted.fov.left * fov_inscribe_scale * fov_post_scale,
        right: view_canted.fov.right * fov_inscribe_scale * fov_post_scale,
        up: view_canted.fov.up * fov_inscribe_scale * fov_post_scale,
        down: view_canted.fov.down * fov_inscribe_scale * fov_post_scale,
    };

    ViewParams {
        pose: viewpose_orth,
        fov: fov_orth_corrected,
    }
}
