use alvr_common::{
    DeviceMotion, Pose,
    glam::{Quat, Vec3},
};
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, time::Duration};

const MAX_U16_F32: f32 = u16::MAX as f32;
const HALF_U16_F32: f32 = MAX_U16_F32 / 2.0;

pub fn pack_u16(val: f32, half_range: f32) -> u16 {
    (val / half_range * HALF_U16_F32 + HALF_U16_F32).clamp(0.0, MAX_U16_F32) as u16
}

pub fn unpack_u16(val: u16, half_range: f32) -> f32 {
    (val as f32 - HALF_U16_F32) * half_range / HALF_U16_F32
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct PackedVec3 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl PackedVec3 {
    pub fn from_vec3(vec: Vec3, half_range: f32) -> Self {
        PackedVec3 {
            x: pack_u16(vec.x, half_range),
            y: pack_u16(vec.y, half_range),
            z: pack_u16(vec.z, half_range),
        }
    }

    pub fn to_vec3(&self, half_range: f32) -> Vec3 {
        Vec3::new(
            unpack_u16(self.x, half_range),
            unpack_u16(self.y, half_range),
            unpack_u16(self.z, half_range),
        )
    }

    pub fn from_quat(quat: Quat) -> Self {
        let scaled_axis = quat.to_scaled_axis();

        PackedVec3 {
            x: pack_u16(scaled_axis.x, PI),
            y: pack_u16(scaled_axis.y, PI),
            z: pack_u16(scaled_axis.z, PI),
        }
    }

    pub fn to_quat(&self) -> Quat {
        let x = unpack_u16(self.x, PI);
        let y = unpack_u16(self.y, PI);
        let z = unpack_u16(self.z, PI);

        Quat::from_scaled_axis(Vec3::new(x, y, z))
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct PackedPose {
    pub orientation: PackedVec3,
    pub position: PackedVec3,
}

impl PackedPose {
    pub fn from_pose(pose: Pose, half_range: f32) -> Self {
        PackedPose {
            orientation: PackedVec3::from_quat(pose.orientation),
            position: PackedVec3::from_vec3(pose.position, half_range),
        }
    }

    pub fn to_pose(&self, half_range: f32) -> Pose {
        Pose {
            orientation: self.orientation.to_quat(),
            position: self.position.to_vec3(half_range),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct PackedDeviceMotion {
    pub pose: PackedPose,
    pub position_delta: PackedVec3,
    pub next_oriantation: PackedVec3,
}

impl PackedDeviceMotion {
    pub fn from_device_motion(
        motion: DeviceMotion,
        position_half_range: f32,
        max_linear_velocity: f32,
        max_angualr_velocity: f32,
    ) -> Self {
        // PackedDeviceMotion {
        //     pose: PackedPose::from_pose(motion.pose, position_half_range),
        //     position_delta: PackedVec3::from_vec3(motion.linear_velocity, max_linear_velocity),
        //     next_oriantation: PackedVec3::from_vec3(motion.angular_velocity, max_angualr_velocity),
        // }
        todo!()
    }

    pub fn to_device_motion(
        &self,
        position_half_range: f32,
        max_linear_velocity: f32,
        max_angualr_velocity: f32,
    ) -> DeviceMotion {
        // DeviceMotion {
        //     pose: self.pose.to_pose(position_half_range),
        //     linear_velocity: self.linear_velocity.to_vec3(max_linear_velocity),
        //     angular_velocity: self.angular_velocity.to_vec3(max_angualr_velocity),
        // }
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum FaceTracking {
    CombinedEyeGaze(Quat),
    EyeGazes([Quat; 2]),
    FbFaceExpressions(Vec<u8>),
    PicoFaceExpressions(Vec<u8>),
    HtcFaceExpressions {
        eye_expression: Option<Vec<u8>>,
        lip_expression: Option<Vec<u8>>,
    },
}

impl Default for FaceTracking {
    fn default() -> Self {
        FaceTracking::CombinedEyeGaze(Quat::IDENTITY)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum BodyTrackingType {
    MetaBodyTracking {
        upper_body: [PackedDeviceMotion; 18],
        lower_body: Option<[PackedDeviceMotion; 14]>,
    },
}

impl Default for BodyTrackingType {
    fn default() -> Self {
        BodyTrackingType::MetaBodyTracking {
            upper_body: [PackedDeviceMotion::default(); 18],
            lower_body: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct BodyTracking {
    tracking_type: BodyTrackingType,
    velocity_scaling: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct JointMotion {
    pose: PackedPose,
    next_orientation: PackedVec3,
}

// The velocity scaling factor is calculated so that the maximum angle between current and
// next_orientation is less than PI / 2, for unambiguous interpolation.
#[derive(Serialize, Deserialize, Default)]
pub struct HandSkeleton {
    pub root_motion: DeviceMotion,
    pub joints_motions: [JointMotion; 25],
    pub velocity_scaling: f32,
}

// Note: face_data does not respect target_timestamp.
#[derive(Serialize, Deserialize, Default)]
pub struct Tracking {
    pub target_timestamp: Duration,
    pub device_motions: Vec<(u64, DeviceMotion)>,
    pub compressed_device_motions: Vec<(u64, PackedDeviceMotion)>,
    pub hand_skeletons: [HandSkeleton; 2],
    pub face_data: Option<FaceTracking>,
    pub body_tracking: Option<BodyTracking>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_pose() {
        let tracking = Tracking {
            target_timestamp: Duration::from_secs(0),
            device_motions: vec![(0, DeviceMotion::default()); 3],
            compressed_device_motions: vec![],
            hand_skeletons: [HandSkeleton::default(), HandSkeleton::default()],
            face_data: Some(FaceTracking::FbFaceExpressions(vec![0; 69])),
            body_tracking: None,//Some(BodyTracking::default()),
        };

        let serialized = bincode::serialize(&tracking).unwrap();

        panic!("Serialized tracking size: {:?}", serialized.len());
    }
}
