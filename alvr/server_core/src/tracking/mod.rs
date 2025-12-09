mod body;
mod face;
mod vmc;

pub use body::*;
pub use face::*;
pub use vmc::*;

use crate::{
    ConnectionContext, SESSION_MANAGER, ServerCoreEvent,
    connection::STREAMING_RECV_TIMEOUT,
    hand_gestures::{self, HAND_GESTURE_BUTTON_SET, HandGestureManager},
    input_mapping::ButtonMappingManager,
};
use alvr_common::{
    AngleSlidingWindowAverage, ConnectionError, DEVICE_ID_TO_PATH, DeviceMotion, Pose,
    SlidingWindowAverage, ViewParams,
    glam::{Quat, Vec2, Vec3},
    inputs as inp,
    parking_lot::Mutex,
};
use alvr_events::{EventType, TrackingEvent};
use alvr_packets::TrackingData;
use alvr_session::{
    BodyTrackingConfig, HeadsetConfig, MarkerColocationConfig, RecenteringMode, Settings,
    VMCConfig, settings_schema::Switch,
};
use alvr_sockets::StreamReceiver;
use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    f32::consts::{FRAC_PI_2, FRAC_PI_4, PI},
    sync::Arc,
    time::Duration,
};

const DEG_TO_RAD: f32 = PI / 180.0;

#[derive(Debug)]
pub enum HandType {
    Left = 0,
    Right = 1,
}

struct RecenteringMarker {
    string: String,
    average_angle: AngleSlidingWindowAverage,
    average_position: SlidingWindowAverage<Vec2>,
}

// todo: Move this struct to Settings and use it for every tracked device
#[derive(Default)]
struct MotionConfig {
    // Position offset applied after rotation offset
    pose_offset: Pose,
    linear_velocity_cutoff: f32,
    angular_velocity_cutoff: f32,
}

pub struct TrackingManager {
    last_head_pose: Pose,             // client's reference space
    inverse_recentering_origin: Pose, // client's reference space
    device_motions_history: HashMap<u64, VecDeque<(Duration, DeviceMotion)>>,
    recentering_marker: Option<RecenteringMarker>,
    markers: HashMap<String, Pose>,
    hand_skeletons_history: [VecDeque<(Duration, [Pose; 26])>; 2],
    max_history_size: usize,
}

impl TrackingManager {
    pub fn new(max_history_size: usize) -> TrackingManager {
        TrackingManager {
            last_head_pose: Pose::IDENTITY,
            inverse_recentering_origin: Pose::IDENTITY,
            device_motions_history: HashMap::new(),
            recentering_marker: None,
            markers: HashMap::new(),
            hand_skeletons_history: [VecDeque::new(), VecDeque::new()],
            max_history_size,
        }
    }

    pub fn recenter(&mut self, recentering_mode: &RecenteringMode) {
        let position = match recentering_mode {
            RecenteringMode::Stage => Vec3::ZERO,
            RecenteringMode::LocalFloor => {
                let mut pos = self.last_head_pose.position;
                pos.y = 0.0;

                pos
            }
            RecenteringMode::Local { view_height } | RecenteringMode::Tilted { view_height } => {
                self.last_head_pose.position - Vec3::new(0.0, *view_height, 0.0)
            }
        };

        let orientation = match recentering_mode {
            RecenteringMode::Stage => Quat::IDENTITY,
            RecenteringMode::LocalFloor | RecenteringMode::Local { .. } => {
                let mut rot = self.last_head_pose.orientation;
                // extract yaw rotation
                rot.x = 0.0;
                rot.z = 0.0;
                rot = rot.normalize();

                rot
            }
            RecenteringMode::Tilted { .. } => self.last_head_pose.orientation,
        };

        self.inverse_recentering_origin = Pose {
            position,
            orientation,
        }
        .inverse();
    }

    pub fn recenter_from_marker(&mut self, config: &MarkerColocationConfig) {
        if let Some(marker_pose) = self.markers.get(&config.qr_code_string) {
            // Detect if the marker is vertical or horizontal, and use two different
            // robust methods to extract the recentering orientation.
            let marker_z_axis = marker_pose.orientation * Vec3::Z;
            let angle_from_y = Vec3::angle_between(marker_z_axis, Vec3::Y);

            let marker_y_angle = if (angle_from_y - FRAC_PI_2).abs() < FRAC_PI_4 {
                // The marker is vertical
                Vec2::new(marker_z_axis.x, marker_z_axis.z)
                    .normalize()
                    .angle_to(Vec2::Y) // (this Y is on the XZ plane -> Z)
            } else {
                let marker_x_axis = marker_pose.orientation * Vec3::X;
                Vec2::new(marker_x_axis.x, marker_x_axis.z)
                    .normalize()
                    .angle_to(Vec2::X)
            };
            let marker_floor_position = Vec2::new(marker_pose.position.x, marker_pose.position.z);

            self.recentering_marker
                .take_if(|rm| rm.string != config.qr_code_string);
            let recentering_marker = if let Some(rm) = &mut self.recentering_marker {
                rm.average_angle.submit_sample(marker_y_angle);
                rm.average_position.submit_sample(marker_floor_position);
                rm
            } else {
                self.recentering_marker.insert(RecenteringMarker {
                    string: config.qr_code_string.clone(),
                    average_angle: AngleSlidingWindowAverage::new(
                        marker_y_angle,
                        self.max_history_size,
                    ),
                    average_position: SlidingWindowAverage::new(
                        marker_floor_position,
                        self.max_history_size,
                    ),
                })
            };

            let average_angle =
                Quat::from_rotation_y(recentering_marker.average_angle.get_average());
            let position = {
                let marker_offset_2d = Vec2::from_array(config.floor_offset);

                let offset_2d =
                    recentering_marker.average_position.get_average() - marker_offset_2d;
                Vec3::new(offset_2d.x, 0.0, offset_2d.y)
            };
            alvr_common::debug!(
                "Recentering from marker. Angle: {average_angle}, Position: {position}"
            );

            let recentering_origin = Pose {
                position,
                orientation: Quat::from_rotation_y(recentering_marker.average_angle.get_average()),
            };

            self.inverse_recentering_origin = recentering_origin.inverse();
        }

        // In case the marker is not found, abort recentering, we still want to use
        // the last marker recentering origin.
    }

    pub fn recenter_pose(&self, pose: Pose) -> Pose {
        self.inverse_recentering_origin * pose
    }

    pub fn recenter_motion(&self, motion: DeviceMotion) -> DeviceMotion {
        self.inverse_recentering_origin * motion
    }

    // Performs all kinds of tracking transformations, driven by settings.
    pub fn report_device_motions(
        &mut self,
        headset_config: &HeadsetConfig,
        timestamp: Duration,
        device_motions: &[(u64, DeviceMotion)],
    ) {
        let mut device_motion_configs = HashMap::new();
        device_motion_configs.insert(*inp::HEAD_ID, MotionConfig::default());
        device_motion_configs.extend([
            (*inp::BODY_CHEST_ID, MotionConfig::default()),
            (*inp::BODY_HIPS_ID, MotionConfig::default()),
            (*inp::BODY_LEFT_ELBOW_ID, MotionConfig::default()),
            (*inp::BODY_RIGHT_ELBOW_ID, MotionConfig::default()),
            (*inp::BODY_LEFT_KNEE_ID, MotionConfig::default()),
            (*inp::BODY_LEFT_FOOT_ID, MotionConfig::default()),
            (*inp::BODY_RIGHT_KNEE_ID, MotionConfig::default()),
            (*inp::BODY_RIGHT_FOOT_ID, MotionConfig::default()),
        ]);

        if let Switch::Enabled(controllers) = &headset_config.controllers {
            device_motion_configs.insert(
                *inp::HAND_LEFT_ID,
                MotionConfig {
                    pose_offset: Pose::IDENTITY,
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );

            device_motion_configs.insert(
                *inp::HAND_RIGHT_ID,
                MotionConfig {
                    pose_offset: Pose::IDENTITY,
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );
        }

        for &(device_id, mut motion) in device_motions {
            if device_id == *inp::HEAD_ID {
                self.last_head_pose = motion.pose;
            }

            if let Some(config) = device_motion_configs.get(&device_id) {
                motion = self.recenter_motion(motion);

                motion.pose = motion.pose * config.pose_offset;

                fn cutoff(v: Vec3, threshold: f32) -> Vec3 {
                    if v.length_squared() > threshold * threshold {
                        v
                    } else {
                        Vec3::ZERO
                    }
                }

                motion.linear_velocity =
                    cutoff(motion.linear_velocity, config.linear_velocity_cutoff);
                motion.angular_velocity =
                    cutoff(motion.angular_velocity, config.angular_velocity_cutoff);
            }

            if let Some(motions) = self.device_motions_history.get_mut(&device_id) {
                motions.push_front((timestamp, motion));

                if motions.len() > self.max_history_size {
                    motions.pop_back();
                }
            } else {
                self.device_motions_history
                    .insert(device_id, VecDeque::from(vec![(timestamp, motion)]));
            }
        }
    }

    // If the exact sample_timestamp is not found, use the closest one if it's not older. This makes
    // sure that we return None if there is no newer sample and always return Some otherwise.
    pub fn get_device_motion(
        &self,
        device_id: u64,
        sample_timestamp: Duration,
    ) -> Option<DeviceMotion> {
        self.device_motions_history
            .get(&device_id)
            .and_then(|motions| {
                // Get first element to initialize a valid motion reference
                if let Some((_, motion)) = motions.front() {
                    let mut best_timestamp_diff = Duration::MAX;
                    let mut best_motion_ref = motion;

                    // Note: we are iterating from most recent to oldest
                    for (ts, m) in motions {
                        match ts.cmp(&sample_timestamp) {
                            Ordering::Equal => return Some(*m),
                            Ordering::Greater => {
                                let diff = ts.saturating_sub(sample_timestamp);
                                if diff < best_timestamp_diff {
                                    best_timestamp_diff = diff;
                                    best_motion_ref = m;
                                }
                            }
                            Ordering::Less => continue,
                        }
                    }

                    (best_timestamp_diff != Duration::MAX).then_some(*best_motion_ref)
                } else {
                    None
                }
            })
    }

    pub fn report_hand_skeleton(
        &mut self,
        hand_type: HandType,
        timestamp: Duration,
        mut skeleton: [Pose; 26],
    ) {
        for pose in &mut skeleton {
            *pose = self.recenter_pose(*pose);
        }

        let skeleton_history = &mut self.hand_skeletons_history[hand_type as usize];

        skeleton_history.push_back((timestamp, skeleton));

        if skeleton_history.len() > self.max_history_size {
            skeleton_history.pop_front();
        }
    }

    pub fn get_hand_skeleton(
        &self,
        hand_type: HandType,
        sample_timestamp: Duration,
    ) -> Option<&[Pose; 26]> {
        self.hand_skeletons_history[hand_type as usize]
            .iter()
            .find(|(timestamp, _)| *timestamp == sample_timestamp)
            .map(|(_, skeleton)| skeleton)
    }

    pub fn unrecenter_view_params(&self, view_params: &mut [ViewParams; 2]) {
        for params in view_params {
            params.pose = self.inverse_recentering_origin.inverse() * params.pose;
        }
    }

    fn report_markers(&mut self, markers: Vec<(String, Pose)>) {
        self.markers = markers.into_iter().collect();
    }
}

pub fn tracking_loop(
    ctx: &ConnectionContext,
    initial_settings: Settings,
    hand_gesture_manager: Arc<Mutex<HandGestureManager>>,
    mut tracking_receiver: StreamReceiver<TrackingData>,
    is_streaming: impl Fn() -> bool,
) {
    let mut gestures_button_mapping_manager =
        initial_settings
            .headset
            .controllers
            .as_option()
            .map(|config| {
                ButtonMappingManager::new_automatic(
                    &HAND_GESTURE_BUTTON_SET,
                    &config.emulation_mode,
                    &config.button_mapping_config,
                )
            });

    let mut face_tracking_sink = initial_settings
        .headset
        .face_tracking
        .into_option()
        .and_then(|config| {
            FaceTrackingSink::new(config.sink, initial_settings.connection.osc_local_port).ok()
        });

    let mut body_tracking_sink = initial_settings
        .headset
        .body_tracking
        .into_option()
        .and_then(|config| {
            BodyTrackingSink::new(config.sink, initial_settings.connection.osc_local_port).ok()
        });

    let mut vmc_sink = initial_settings
        .headset
        .vmc
        .into_option()
        .and_then(|config| VMCSink::new(config).ok());

    while is_streaming() {
        let data = match tracking_receiver.recv(STREAMING_RECV_TIMEOUT) {
            Ok(tracking) => tracking,
            Err(ConnectionError::TryAgain(_)) => continue,
            Err(ConnectionError::Other(_)) => return,
        };
        let Ok(mut tracking) = data.get_header() else {
            return;
        };

        let timestamp = tracking.poll_timestamp;

        if let Some(stats) = &mut *ctx.statistics_manager.write() {
            stats.report_tracking_received(timestamp);
        }

        let controllers_config = {
            let data_lock = SESSION_MANAGER.read();
            data_lock
                .settings()
                .headset
                .controllers
                .clone()
                .into_option()
        };

        let device_motion_keys;
        {
            let mut tracking_manager_lock = ctx.tracking_manager.write();
            let session_manager_lock = SESSION_MANAGER.read();
            let headset_config = &session_manager_lock.settings().headset;

            tracking.device_motions.extend_from_slice(
                &body::get_default_body_trackers_from_detached_controllers(
                    &tracking.device_motions,
                ),
            );
            tracking.device_motions.extend_from_slice(
                &body::get_default_body_trackers_from_motion_trackers_bd(&tracking.device_motions),
            );
            if let Some(skeleton) = &tracking.body {
                tracking
                    .device_motions
                    .extend_from_slice(&body::extract_default_trackers(skeleton));
            }

            device_motion_keys = tracking
                .device_motions
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>();

            let velocity_multiplier = session_manager_lock.settings().extra.velocities_multiplier;
            tracking.device_motions.iter_mut().for_each(|(_, motion)| {
                motion.linear_velocity *= velocity_multiplier;
                motion.angular_velocity *= velocity_multiplier;
            });

            tracking_manager_lock.report_device_motions(
                headset_config,
                timestamp,
                &tracking.device_motions,
            );

            if !tracking.markers.is_empty() {
                tracking_manager_lock.report_markers(tracking.markers);

                if let Some(config) = headset_config.marker_colocation.as_option() {
                    tracking_manager_lock.recenter_from_marker(config);
                };
            }

            if let Some(skeleton) = tracking.hand_skeletons[0] {
                tracking_manager_lock.report_hand_skeleton(HandType::Left, timestamp, skeleton);
            }
            if let Some(skeleton) = tracking.hand_skeletons[1] {
                tracking_manager_lock.report_hand_skeleton(HandType::Right, timestamp, skeleton);
            }

            if let Some(sink) = &mut face_tracking_sink {
                sink.send_tracking(&tracking.face);
            }

            if session_manager_lock.settings().extra.logging.log_tracking {
                let device_motions = device_motion_keys
                    .iter()
                    .filter_map(move |id| {
                        Some((
                            (*DEVICE_ID_TO_PATH.get(id)?).into(),
                            tracking_manager_lock
                                .get_device_motion(*id, timestamp)
                                .unwrap(),
                        ))
                    })
                    .collect::<Vec<(String, DeviceMotion)>>();

                alvr_events::send_event(EventType::Tracking(Box::new(TrackingEvent {
                    device_motions,
                    hand_skeletons: tracking.hand_skeletons,
                    face: tracking.face,
                })))
            }
        };

        // Handle hand gestures
        if let (Some(gestures_config), Some(gestures_button_mapping_manager)) = (
            controllers_config
                .as_ref()
                .and_then(|c| c.hand_tracking_interaction.as_option()),
            &mut gestures_button_mapping_manager,
        ) {
            let mut hand_gesture_manager_lock = hand_gesture_manager.lock();

            if !device_motion_keys.contains(&*inp::HAND_LEFT_ID)
                && let Some(hand_skeleton) = tracking.hand_skeletons[0]
            {
                ctx.events_sender
                    .send(ServerCoreEvent::Buttons(
                        hand_gestures::trigger_hand_gesture_actions(
                            gestures_button_mapping_manager,
                            *inp::HAND_LEFT_ID,
                            &hand_gesture_manager_lock.get_active_gestures(
                                &hand_skeleton,
                                gestures_config,
                                *inp::HAND_LEFT_ID,
                            ),
                            gestures_config.only_touch,
                        ),
                    ))
                    .ok();
            }
            if !device_motion_keys.contains(&*inp::HAND_RIGHT_ID)
                && let Some(hand_skeleton) = tracking.hand_skeletons[1]
            {
                ctx.events_sender
                    .send(ServerCoreEvent::Buttons(
                        hand_gestures::trigger_hand_gesture_actions(
                            gestures_button_mapping_manager,
                            *inp::HAND_RIGHT_ID,
                            &hand_gesture_manager_lock.get_active_gestures(
                                &hand_skeleton,
                                gestures_config,
                                *inp::HAND_RIGHT_ID,
                            ),
                            gestures_config.only_touch,
                        ),
                    ))
                    .ok();
            }
        }

        ctx.events_sender
            .send(ServerCoreEvent::Tracking {
                poll_timestamp: tracking.poll_timestamp,
            })
            .ok();

        let publish_vmc = matches!(
            SESSION_MANAGER.read().settings().headset.vmc,
            Switch::Enabled(VMCConfig { publish: true, .. })
        );
        if publish_vmc {
            let orientation_correction = matches!(
                SESSION_MANAGER.read().settings().headset.vmc,
                Switch::Enabled(VMCConfig {
                    orientation_correction: true,
                    ..
                })
            );

            if let Some(sink) = &mut vmc_sink {
                let tracking_manager_lock = ctx.tracking_manager.read();
                let device_motions = device_motion_keys
                    .iter()
                    .map(move |id| {
                        (
                            *id,
                            tracking_manager_lock
                                .get_device_motion(*id, timestamp)
                                .unwrap(),
                        )
                    })
                    .collect::<Vec<(u64, DeviceMotion)>>();

                if let Some(skeleton) = tracking.hand_skeletons[0] {
                    sink.send_hand_tracking(HandType::Left, &skeleton, orientation_correction);
                }
                if let Some(skeleton) = tracking.hand_skeletons[1] {
                    sink.send_hand_tracking(HandType::Right, &skeleton, orientation_correction);
                }
                sink.send_tracking(&device_motions, orientation_correction);
            }
        }

        let track_body = matches!(
            SESSION_MANAGER.read().settings().headset.body_tracking,
            Switch::Enabled(BodyTrackingConfig { tracked: true, .. })
        );
        if track_body && let Some(sink) = &mut body_tracking_sink {
            let tracking_manager_lock = ctx.tracking_manager.read();
            let device_motions = device_motion_keys
                .iter()
                .map(move |id| {
                    (
                        *id,
                        tracking_manager_lock
                            .get_device_motion(*id, timestamp)
                            .unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            sink.send_tracking(&device_motions);
        }
    }
}
