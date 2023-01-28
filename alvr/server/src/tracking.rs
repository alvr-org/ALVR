use crate::{to_ffi_quat, FfiHandSkeleton};
use alvr_common::{
    glam::{Quat, Vec3},
    HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID,
};
use alvr_session::HeadsetDesc;
use alvr_sockets::{DeviceMotion, Pose};
use settings_schema::Switch;
use std::f32::consts::{FRAC_PI_2, PI};

pub struct TrackingManager {
    settings: HeadsetDesc,
}

impl TrackingManager {
    pub fn new(settings: HeadsetDesc) -> TrackingManager {
        TrackingManager { settings }
    }

    // todo: extend this method for space recalibration.
    pub fn filter_map_motion(
        &self,
        device_id: u64,
        mut motion: DeviceMotion,
    ) -> Option<DeviceMotion> {
        if device_id == *HEAD_ID {
            if self.settings.force_3dof {
                motion.pose.position = Vec3::ZERO;
            }

            Some(motion)
        } else if device_id == *LEFT_HAND_ID || device_id == *RIGHT_HAND_ID {
            matches!(self.settings.controllers, Switch::Enabled(_)).then(|| motion)
        } else {
            Some(motion)
        }
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
