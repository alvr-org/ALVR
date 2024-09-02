use alvr_common::{
    glam::{EulerRot, Quat, Vec3},
    DeviceMotion, Pose, BODY_CHEST_ID, BODY_HIPS_ID, BODY_LEFT_ELBOW_ID, BODY_LEFT_FOOT_ID,
    BODY_LEFT_KNEE_ID, BODY_RIGHT_ELBOW_ID, BODY_RIGHT_FOOT_ID, BODY_RIGHT_KNEE_ID, HAND_LEFT_ID,
    HAND_RIGHT_ID, HEAD_ID,
};
use alvr_session::{
    settings_schema::Switch, HeadsetConfig, PositionRecenteringMode, RotationRecenteringMode,
};
use std::{collections::HashMap, f32::consts::PI};

const DEG_TO_RAD: f32 = PI / 180.0;

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
    ) -> Vec<(u64, DeviceMotion)> {
        let mut device_motion_configs = HashMap::new();
        device_motion_configs.insert(*HEAD_ID, MotionConfig::default());
        device_motion_configs.extend([
            (*BODY_CHEST_ID, MotionConfig::default()),
            (*BODY_HIPS_ID, MotionConfig::default()),
            (*BODY_LEFT_ELBOW_ID, MotionConfig::default()),
            (*BODY_RIGHT_ELBOW_ID, MotionConfig::default()),
            (*BODY_LEFT_KNEE_ID, MotionConfig::default()),
            (*BODY_LEFT_FOOT_ID, MotionConfig::default()),
            (*BODY_RIGHT_KNEE_ID, MotionConfig::default()),
            (*BODY_RIGHT_FOOT_ID, MotionConfig::default()),
        ]);

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
                motion.pose.orientation *= config.pose_offset.orientation;
                motion.pose.position += motion.pose.orientation * config.pose_offset.position;

                motion.linear_velocity += motion
                    .angular_velocity
                    .cross(motion.pose.orientation * config.pose_offset.position);
                motion.angular_velocity =
                    motion.pose.orientation.conjugate() * motion.angular_velocity;

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

                transformed_motions.push((device_id, motion));
            }
        }

        transformed_motions
    }

    pub fn transform_hand_skeleton(&self, mut skeleton: [Pose; 26]) -> [Pose; 26] {
        for pose in &mut skeleton {
            *pose = self.recenter_pose(*pose);
        }

        skeleton
    }
}

// Head and eyes must be in the same (not recentered) convention
pub fn to_local_eyes(
    raw_global_head: Pose,
    raw_global_eyes: [Option<Pose>; 2],
) -> [Option<Pose>; 2] {
    [
        raw_global_eyes[0].map(|e| raw_global_head.inverse() * e),
        raw_global_eyes[1].map(|e| raw_global_head.inverse() * e),
    ]
}
