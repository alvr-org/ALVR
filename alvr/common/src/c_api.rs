use crate::{Fov, Pose, ViewParams};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

#[repr(C)]
pub struct AlvrFov {
    /// Negative, radians
    pub left: f32,
    /// Positive, radians
    pub right: f32,
    /// Positive, radians
    pub up: f32,
    /// Negative, radians
    pub down: f32,
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct AlvrQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct AlvrPose {
    pub orientation: AlvrQuat,
    pub position: [f32; 3],
}

#[repr(C)]
pub struct AlvrViewParams {
    pub pose: AlvrPose,
    pub fov: AlvrFov,
}

/// Server-aligned FFR parameters shared by the encoder and client inverse transform.
#[repr(C)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct AlvrFoveatedEncodingParams {
    pub encoded_view_resolution: [u32; 2],
    pub view_ratio: [f32; 2],
    pub center_size: [f32; 2],
    /// Left/right eye offsets, each in X/Y order. Consumers must not align these again.
    pub center_shifts: [[f32; 2]; 2],
    pub edge_ratio: [f32; 2],
}

#[repr(u8)]
pub enum AlvrCodecType {
    H264 = 0,
    Hevc = 1,
    AV1 = 2,
}

pub fn to_capi_fov(fov: &Fov) -> AlvrFov {
    AlvrFov {
        left: fov.left,
        right: fov.right,
        up: fov.up,
        down: fov.down,
    }
}

pub fn from_capi_fov(fov: &AlvrFov) -> Fov {
    Fov {
        left: fov.left,
        right: fov.right,
        up: fov.up,
        down: fov.down,
    }
}

pub fn from_capi_quat(quat: &AlvrQuat) -> Quat {
    Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w)
}

pub fn to_capi_quat(quat: &Quat) -> AlvrQuat {
    AlvrQuat {
        x: quat.x,
        y: quat.y,
        z: quat.z,
        w: quat.w,
    }
}

pub fn to_capi_pose(pose: &Pose) -> AlvrPose {
    AlvrPose {
        orientation: to_capi_quat(&pose.orientation),
        position: pose.position.to_array(),
    }
}

pub fn from_capi_pose(pose: &AlvrPose) -> Pose {
    Pose {
        orientation: from_capi_quat(&pose.orientation),
        position: Vec3::from_slice(&pose.position),
    }
}

pub fn to_capi_view_params(view_params: &ViewParams) -> AlvrViewParams {
    AlvrViewParams {
        pose: to_capi_pose(&view_params.pose),
        fov: to_capi_fov(&view_params.fov),
    }
}

pub fn from_capi_view_params(view_params: &AlvrViewParams) -> ViewParams {
    ViewParams {
        pose: from_capi_pose(&view_params.pose),
        fov: from_capi_fov(&view_params.fov),
    }
}
