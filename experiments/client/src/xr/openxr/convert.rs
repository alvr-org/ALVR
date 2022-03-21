use alvr_common::glam::{Quat, Vec3};
use alvr_session::Fov;
use openxr as xr;

pub fn from_xr_orientation(quat: xr::Quaternionf) -> Quat {
    Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w)
}

pub fn to_xr_orientation(quat: Quat) -> xr::Quaternionf {
    xr::Quaternionf {
        x: quat.x,
        y: quat.y,
        z: quat.z,
        w: quat.w,
    }
}

pub fn from_xr_vec3(v: xr::Vector3f) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

pub fn to_xr_vec3(v: Vec3) -> xr::Vector3f {
    xr::Vector3f {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

pub fn from_xr_fov(fov: xr::Fovf) -> Fov {
    Fov {
        left: fov.angle_left,
        right: fov.angle_right,
        top: fov.angle_up,
        bottom: fov.angle_down,
    }
}

pub fn to_xr_fov(fov: Fov) -> xr::Fovf {
    xr::Fovf {
        angle_left: fov.left,
        angle_right: fov.right,
        angle_up: fov.top,
        angle_down: fov.bottom,
    }
}
