use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::{ops::Mul, time::Duration};

// Field of view in radians
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct Fov {
    pub left: f32,
    pub right: f32,
    pub up: f32,
    pub down: f32,
}

impl Default for Fov {
    fn default() -> Self {
        Fov {
            left: -1.0,
            right: 1.0,
            up: 1.0,
            down: -1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug)]
pub struct Pose {
    pub orientation: Quat, // NB: default Quat is identity
    pub position: Vec3,
}

impl Pose {
    pub fn inverse(&self) -> Pose {
        let inverse_orientation = self.orientation.conjugate();
        Pose {
            orientation: inverse_orientation,
            position: inverse_orientation * -self.position,
        }
    }
}

impl Mul<Pose> for Pose {
    type Output = Pose;

    fn mul(self, rhs: Pose) -> Pose {
        Pose {
            orientation: self.orientation * rhs.orientation,
            position: self.position + self.orientation * rhs.position,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug)]
pub struct DeviceMotion {
    pub pose: Pose,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}

impl Mul<DeviceMotion> for Pose {
    type Output = DeviceMotion;

    fn mul(self, rhs: DeviceMotion) -> DeviceMotion {
        DeviceMotion {
            pose: self * rhs.pose,
            linear_velocity: self.orientation * rhs.linear_velocity,
            angular_velocity: self.orientation * rhs.angular_velocity,
        }
    }
}

// Calculate difference ensuring maximum precision is preserved
fn difference_seconds(from: Duration, to: Duration) -> f32 {
    to.saturating_sub(from).as_secs_f32() - from.saturating_sub(to).as_secs_f32()
}

impl DeviceMotion {
    pub fn predict(&self, from_timestamp: Duration, to_timestamp: Duration) -> Self {
        let delta_time_s = difference_seconds(from_timestamp, to_timestamp);

        let delta_position = self.linear_velocity * delta_time_s;
        let delta_orientation = Quat::from_scaled_axis(self.angular_velocity * delta_time_s);

        DeviceMotion {
            pose: Pose {
                orientation: delta_orientation * self.pose.orientation,
                position: self.pose.position + delta_position,
            },
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
        }
    }
}

// Per eye view parameters
// todo: send together with video frame
#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct ViewParams {
    pub pose: Pose,
    pub fov: Fov,
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
