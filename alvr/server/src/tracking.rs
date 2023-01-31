use crate::{to_ffi_quat, FfiDeviceMotion, FfiHandSkeleton};
use alvr_common::{
    glam::{EulerRot, Quat, Vec3},
    HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_session::HeadsetDesc;
use alvr_sockets::{DeviceMotion, Pose};
use settings_schema::Switch;
use std::{
    collections::HashMap,
    f32::consts::{FRAC_PI_2, PI},
};

// todo: Move this struct to Settings and use it for every tracked device
#[derive(Default)]
struct MotionConfig {
    rotation_offset: Quat,
    // Position offset applied after rotation_offset
    local_position_offset: Vec3,
    linear_velocity_cutoff: f32,
    angular_velocity_cutoff: f32,
}

pub struct TrackingManager {
    device_motion_configs: HashMap<u64, MotionConfig>,
}

impl TrackingManager {
    pub fn new(settings: HeadsetDesc) -> TrackingManager {
        let mut device_motion_configs = HashMap::new();
        device_motion_configs.insert(*HEAD_ID, MotionConfig::default());

        if let Switch::Enabled(controllers) = &settings.controllers {
            const DEG_TO_RAD: f32 = PI / 180.0;

            let t = controllers.position_offset_left;
            let r = controllers.rotation_offset_left;

            device_motion_configs.insert(
                *LEFT_HAND_ID,
                MotionConfig {
                    local_position_offset: Vec3::new(t[0], t[1], t[2]),
                    rotation_offset: Quat::from_euler(
                        EulerRot::XYZ,
                        r[0] * DEG_TO_RAD,
                        r[1] * DEG_TO_RAD,
                        r[2] * DEG_TO_RAD,
                    ),
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );

            device_motion_configs.insert(
                *RIGHT_HAND_ID,
                MotionConfig {
                    local_position_offset: Vec3::new(-t[0], t[1], t[2]),
                    rotation_offset: Quat::from_euler(
                        EulerRot::XYZ,
                        r[0] * DEG_TO_RAD,
                        -r[1] * DEG_TO_RAD,
                        -r[2] * DEG_TO_RAD,
                    ),
                    linear_velocity_cutoff: controllers.linear_velocity_cutoff,
                    angular_velocity_cutoff: controllers.angular_velocity_cutoff * DEG_TO_RAD,
                },
            );
        };

        TrackingManager {
            device_motion_configs,
        }
    }

    // Performs all kinds of tracking transformations, driven by settings, and convert to ffi.
    pub fn transform_motions(
        &self,
        device_motions: &[(u64, DeviceMotion)],
        left_hand_skeleton_enabled: bool,
        right_hand_skeleton_enabled: bool,
    ) -> Vec<FfiDeviceMotion> {
        let mut ffi_motions = vec![];
        for &(device_id, mut motion) in device_motions {
            if let Some(config) = self.device_motion_configs.get(&device_id) {
                if (device_id != *LEFT_HAND_ID || !left_hand_skeleton_enabled)
                    && (device_id != *RIGHT_HAND_ID || !right_hand_skeleton_enabled)
                {
                    motion.pose.orientation *= config.rotation_offset;
                    motion.pose.position += motion.pose.orientation * config.local_position_offset;
                    motion.linear_velocity = config.rotation_offset * motion.linear_velocity;
                    motion.angular_velocity = config.rotation_offset * motion.angular_velocity;
                }

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

                ffi_motions.push(FfiDeviceMotion {
                    deviceID: device_id,
                    orientation: to_ffi_quat(motion.pose.orientation),
                    position: motion.pose.position.to_array(),
                    linearVelocity: motion.linear_velocity.to_array(),
                    angularVelocity: motion.angular_velocity.to_array(),
                })
            }
        }

        ffi_motions
    }
}

pub fn to_openvr_hand_skeleton(device_id: u64, hand_skeleton: [Pose; 26]) -> FfiHandSkeleton {
    // Convert from global to local joint pose. The orientation frame of reference is also converted
    // from OpenXR to SteamVR (hand-specific!)
    pub fn local_pose(id: u64, parent: Pose, current: Pose) -> Pose {
        let o = parent.orientation.conjugate() * current.orientation;
        let p = parent.orientation.conjugate() * (current.position - parent.position);

        // Convert to SteamVR frame of reference
        let (orientation, position) = if id == *LEFT_HAND_ID {
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

    let id = device_id;

    // global joints
    let gj = hand_skeleton;

    let fixed_g_wrist = Pose {
        orientation: gj[1].orientation
            * Quat::from_rotation_y(-FRAC_PI_2)
            * Quat::from_rotation_x(FRAC_PI_2),
        position: gj[1].position,
    };

    let local_joints = vec![
        // Palm
        Pose::default(),
        // Wrist
        {
            let position = gj[0].orientation.conjugate() * (gj[1].position - gj[0].position);

            let sign = if id == *LEFT_HAND_ID { -1.0 } else { 1.0 };
            let orientation = gj[0].orientation.conjugate()
                * gj[1].orientation
                * Quat::from_rotation_x(PI)
                * Quat::from_rotation_z(sign * FRAC_PI_2);

            Pose {
                orientation,
                position,
            }
        },
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
    ];

    FfiHandSkeleton {
        jointRotations: local_joints
            .iter()
            .map(|j| to_ffi_quat(j.orientation))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        jointPositions: local_joints
            .iter()
            .map(|j| j.position.to_array())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    }
}
