use crate::{to_ffi_quat, FfiBodyTracker, FfiDeviceMotion, FfiHandSkeleton};
use alvr_common::{
    glam::{EulerRot, Quat, Vec3},
    once_cell::sync::Lazy,
    DeviceMotion, Pose, BODY_CHEST_ID, BODY_HIPS_ID, BODY_LEFT_ELBOW_ID, BODY_LEFT_FOOT_ID,
    BODY_LEFT_KNEE_ID, BODY_RIGHT_ELBOW_ID, BODY_RIGHT_FOOT_ID, BODY_RIGHT_KNEE_ID, HAND_LEFT_ID,
    HAND_RIGHT_ID, HEAD_ID,
};
use alvr_session::{
    settings_schema::Switch, HeadsetConfig, PositionRecenteringMode, RotationRecenteringMode,
};
use std::{
    collections::HashMap,
    f32::consts::{FRAC_PI_2, PI},
};

const DEG_TO_RAD: f32 = PI / 180.0;

static BODY_TRACKER_ID_MAP: Lazy<HashMap<u64, u32>> = Lazy::new(|| {
    HashMap::from([
        // Upper body
        (*BODY_CHEST_ID, 0),
        (*BODY_HIPS_ID, 1),
        (*BODY_LEFT_ELBOW_ID, 2),
        (*BODY_RIGHT_ELBOW_ID, 3),
        // Legs
        (*BODY_LEFT_KNEE_ID, 4),
        (*BODY_LEFT_FOOT_ID, 5),
        (*BODY_RIGHT_KNEE_ID, 6),
        (*BODY_RIGHT_FOOT_ID, 7),
    ])
});

fn get_hand_skeleton_offsets(config: &HeadsetConfig) -> (Pose, Pose) {
    let left_offset;
    let right_offset;
    if let Switch::Enabled(controllers) = &config.controllers {
        let t = controllers.left_hand_tracking_position_offset;
        let r = controllers.left_hand_tracking_rotation_offset;

        left_offset = Pose {
            orientation: Quat::from_euler(
                EulerRot::XYZ,
                r[0] * DEG_TO_RAD,
                r[1] * DEG_TO_RAD,
                r[2] * DEG_TO_RAD,
            ),
            position: Vec3::new(t[0], t[1], t[2]),
        };
        right_offset = Pose {
            orientation: Quat::from_euler(
                EulerRot::XYZ,
                r[0] * DEG_TO_RAD,
                -r[1] * DEG_TO_RAD,
                -r[2] * DEG_TO_RAD,
            ),
            position: Vec3::new(-t[0], t[1], t[2]),
        };
    } else {
        left_offset = Pose::default();
        right_offset = Pose::default();
    }

    (left_offset, right_offset)
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
    last_head_pose: Pose,     // client's reference space
    recentering_origin: Pose, // client's reference space
}

impl TrackingManager {
    pub fn new() -> TrackingManager {
        TrackingManager {
            last_head_pose: Pose::default(),
            recentering_origin: Pose::default(),
        }
    }

    pub fn recenter(
        &mut self,
        position_recentering_mode: PositionRecenteringMode,
        rotation_recentering_mode: RotationRecenteringMode,
    ) {
        self.recentering_origin.position = match position_recentering_mode {
            PositionRecenteringMode::Disabled => Vec3::ZERO,
            PositionRecenteringMode::LocalFloor => {
                let mut pos = self.last_head_pose.position;
                pos.y = 0.0;

                pos
            }
            PositionRecenteringMode::Local { view_height } => {
                self.last_head_pose.position - Vec3::new(0.0, view_height, 0.0)
            }
        };

        self.recentering_origin.orientation = match rotation_recentering_mode {
            RotationRecenteringMode::Disabled => Quat::IDENTITY,
            RotationRecenteringMode::Yaw => {
                let mut rot = self.last_head_pose.orientation;
                // extract yaw rotation
                rot.x = 0.0;
                rot.z = 0.0;
                rot = rot.normalize();

                rot
            }
            RotationRecenteringMode::Tilted => self.last_head_pose.orientation,
        };
    }

    pub fn recenter_pose(&self, pose: Pose) -> Pose {
        let inverse_origin_orientation = self.recentering_origin.orientation.conjugate();

        Pose {
            orientation: inverse_origin_orientation * pose.orientation,
            position: inverse_origin_orientation
                * (pose.position - self.recentering_origin.position),
        }
    }

    // Performs all kinds of tracking transformations, driven by settings.
    pub fn transform_motions(
        &mut self,
        config: &HeadsetConfig,
        device_motions: &[(u64, DeviceMotion)],
        hand_skeletons_enabled: [bool; 2],
    ) -> Vec<(u64, DeviceMotion)> {
        let mut device_motion_configs = HashMap::new();
        device_motion_configs.insert(*HEAD_ID, MotionConfig::default());

        if let Switch::Enabled(controllers) = &config.controllers {
            let t = controllers.left_controller_position_offset;
            let r = controllers.left_controller_rotation_offset;

            device_motion_configs.insert(
                *HAND_LEFT_ID,
                MotionConfig {
                    pose_offset: Pose {
                        orientation: Quat::from_euler(
                            EulerRot::XYZ,
                            r[0] * DEG_TO_RAD,
                            r[1] * DEG_TO_RAD,
                            r[2] * DEG_TO_RAD,
                        ),
                        position: Vec3::new(t[0], t[1], t[2]),
                    },
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );

            device_motion_configs.insert(
                *HAND_RIGHT_ID,
                MotionConfig {
                    pose_offset: Pose {
                        orientation: Quat::from_euler(
                            EulerRot::XYZ,
                            r[0] * DEG_TO_RAD,
                            -r[1] * DEG_TO_RAD,
                            -r[2] * DEG_TO_RAD,
                        ),
                        position: Vec3::new(-t[0], t[1], t[2]),
                    },
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );
        }

        let (left_hand_skeleton_offset, right_hand_skeleton_offset) =
            get_hand_skeleton_offsets(config);

        let mut transformed_motions = vec![];
        for &(device_id, mut motion) in device_motions {
            if device_id == *HEAD_ID {
                self.last_head_pose = motion.pose;
            }

            if let Some(config) = device_motion_configs.get(&device_id) {
                // Recenter
                motion.pose = self.recenter_pose(motion.pose);

                let inverse_origin_orientation = self.recentering_origin.orientation.conjugate();
                motion.linear_velocity = inverse_origin_orientation * motion.linear_velocity;
                motion.angular_velocity = inverse_origin_orientation * motion.angular_velocity;

                // Apply custom transform
                let pose_offset = if device_id == *HAND_LEFT_ID && hand_skeletons_enabled[0] {
                    left_hand_skeleton_offset
                } else if device_id == *HAND_RIGHT_ID && hand_skeletons_enabled[1] {
                    right_hand_skeleton_offset
                } else {
                    config.pose_offset
                };
                motion.pose.orientation *= pose_offset.orientation;
                motion.pose.position += motion.pose.orientation * pose_offset.position;

                motion.linear_velocity += motion
                    .angular_velocity
                    .cross(motion.pose.orientation * pose_offset.position);
                motion.angular_velocity =
                    motion.pose.orientation.conjugate() * motion.angular_velocity;

                fn cutoff(v: Vec3, threshold: f32) -> Vec3 {
                    if v.length_squared() > threshold * threshold {
                        v
                    } else {
                        Vec3::ZERO
                    }
                }

                if (device_id == *HAND_LEFT_ID && hand_skeletons_enabled[0])
                    || (device_id == *HAND_RIGHT_ID && hand_skeletons_enabled[1])
                {
                    // On hand tracking, velocities seem to make hands overly jittery
                    motion.linear_velocity = Vec3::ZERO;
                    motion.angular_velocity = Vec3::ZERO;
                } else {
                    motion.linear_velocity =
                        cutoff(motion.linear_velocity, config.linear_velocity_cutoff);
                    motion.angular_velocity =
                        cutoff(motion.angular_velocity, config.angular_velocity_cutoff);
                }

                transformed_motions.push((device_id, motion));
            }
        }

        transformed_motions
    }
}

pub fn to_openvr_hand_skeleton(
    config: &HeadsetConfig,
    device_id: u64,
    hand_skeleton: [Pose; 26],
) -> [Pose; 31] {
    let (left_hand_skeleton_offset, right_hand_skeleton_offset) = get_hand_skeleton_offsets(config);
    let id = device_id;

    // global joints
    let gj = hand_skeleton;

    // Correct the orientation for auxiliary bones.
    pub fn aux_orientation(id: u64, pose: Pose) -> Pose {
        let o = pose.orientation;
        let p = pose.position;

        // Convert to SteamVR basis orientations
        let (orientation, position) = if id == *HAND_LEFT_ID {
            (
                Quat::from_xyzw(o.x, o.y, o.z, o.w)
                    * Quat::from_euler(EulerRot::YXZ, -FRAC_PI_2, FRAC_PI_2, 0.0),
                Vec3::new(p.x, p.y, p.z),
            )
        } else {
            (
                Quat::from_xyzw(o.x, o.y, o.z, o.w)
                    * Quat::from_euler(EulerRot::YXZ, FRAC_PI_2, -FRAC_PI_2, 0.0),
                Vec3::new(p.x, p.y, p.z),
            )
        };

        Pose {
            orientation,
            position,
        }
    }

    // Convert from global to local joint pose. The orientation frame of reference is also
    // converted from OpenXR to SteamVR (hand-specific!)
    pub fn local_pose(id: u64, parent: Pose, current: Pose) -> Pose {
        let o = parent.orientation.conjugate() * current.orientation;
        let p = parent.orientation.conjugate() * (current.position - parent.position);

        // Convert to SteamVR frame of reference
        let (orientation, position) = if id == *HAND_LEFT_ID {
            (
                Quat::from_xyzw(-o.z, -o.y, -o.x, o.w),
                Vec3::new(-p.z, -p.y, -p.x),
            )
        } else {
            (
                Quat::from_xyzw(o.z, o.y, -o.x, o.w),
                Vec3::new(p.z, p.y, -p.x),
            )
        };

        Pose {
            orientation,
            position,
        }
    }

    // Adjust hand position based on the emulated controller for joints
    // parented to the root.
    let root_parented_pose = |pose: Pose| -> Pose {
        let pose_offset = if id == *HAND_LEFT_ID {
            left_hand_skeleton_offset
        } else {
            right_hand_skeleton_offset
        };

        let sign = if id == *HAND_LEFT_ID { -1.0 } else { 1.0 };
        let orientation = pose_offset.orientation.conjugate()
            * gj[0].orientation.conjugate()
            * pose.orientation
            * Quat::from_euler(EulerRot::XZY, PI, sign * FRAC_PI_2, 0.0);

        let position = -pose_offset.position
            + pose_offset.orientation.conjugate()
                * gj[0].orientation.conjugate()
                * (pose.position - gj[0].position);

        Pose {
            orientation,
            position,
        }
    };

    let fixed_g_wrist = Pose {
        orientation: gj[1].orientation
            * Quat::from_euler(EulerRot::YXZ, -FRAC_PI_2, FRAC_PI_2, 0.0),
        position: gj[1].position,
    };

    [
        // Palm. NB: this is ignored by SteamVR
        Pose::default(),
        // Wrist
        root_parented_pose(gj[1]),
        // Thumb
        local_pose(id, fixed_g_wrist, gj[2]),
        local_pose(id, gj[2], gj[3]),
        local_pose(id, gj[3], gj[4]),
        local_pose(id, gj[4], gj[5]),
        // Index
        local_pose(id, fixed_g_wrist, gj[6]),
        local_pose(id, gj[6], gj[7]),
        local_pose(id, gj[7], gj[8]),
        local_pose(id, gj[8], gj[9]),
        local_pose(id, gj[9], gj[10]),
        // Middle
        local_pose(id, fixed_g_wrist, gj[11]),
        local_pose(id, gj[11], gj[12]),
        local_pose(id, gj[12], gj[13]),
        local_pose(id, gj[13], gj[14]),
        local_pose(id, gj[14], gj[15]),
        // Ring
        local_pose(id, fixed_g_wrist, gj[16]),
        local_pose(id, gj[16], gj[17]),
        local_pose(id, gj[17], gj[18]),
        local_pose(id, gj[18], gj[19]),
        local_pose(id, gj[19], gj[20]),
        // Little
        local_pose(id, fixed_g_wrist, gj[21]),
        local_pose(id, gj[21], gj[22]),
        local_pose(id, gj[22], gj[23]),
        local_pose(id, gj[23], gj[24]),
        local_pose(id, gj[24], gj[25]),
        // Aux bones
        aux_orientation(id, root_parented_pose(gj[4])),
        aux_orientation(id, root_parented_pose(gj[9])),
        aux_orientation(id, root_parented_pose(gj[14])),
        aux_orientation(id, root_parented_pose(gj[19])),
        aux_orientation(id, root_parented_pose(gj[24])),
    ]
}

pub fn to_ffi_motion(device_id: u64, motion: DeviceMotion) -> FfiDeviceMotion {
    FfiDeviceMotion {
        deviceID: device_id,
        orientation: to_ffi_quat(motion.pose.orientation),
        position: motion.pose.position.to_array(),
        linearVelocity: motion.linear_velocity.to_array(),
        angularVelocity: motion.angular_velocity.to_array(),
    }
}

pub fn to_ffi_skeleton(skeleton: [Pose; 31]) -> FfiHandSkeleton {
    FfiHandSkeleton {
        jointRotations: skeleton
            .iter()
            .map(|j| to_ffi_quat(j.orientation))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        jointPositions: skeleton
            .iter()
            .map(|j| j.position.to_array())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    }
}

pub fn to_ffi_body_trackers(
    device_motions: &[(u64, DeviceMotion)],
    tracking_manager: &TrackingManager,
    tracking: bool,
) -> Option<Vec<FfiBodyTracker>> {
    let mut trackers = Vec::<FfiBodyTracker>::new();

    for (id, motion) in device_motions.iter() {
        if BODY_TRACKER_ID_MAP.contains_key(id) {
            let pose = tracking_manager.recenter_pose(motion.pose);
            trackers.push(FfiBodyTracker {
                trackerID: *BODY_TRACKER_ID_MAP.get(id).unwrap(),
                orientation: to_ffi_quat(pose.orientation),
                position: pose.position.to_array(),
                tracking: tracking.into(),
            });
        }
    }

    Some(trackers)
}

// Head and eyesmust be in the same (nt recentered) convention
pub fn to_local_eyes(
    raw_global_head: Pose,
    raw_global_eyes: [Option<Pose>; 2],
) -> [Option<Pose>; 2] {
    [
        raw_global_eyes[0].map(|e| raw_global_head.inverse() * e),
        raw_global_eyes[1].map(|e| raw_global_head.inverse() * e),
    ]
}
