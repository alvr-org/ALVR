use glam::{Quat, Vec3};
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

impl Fov {
    pub const DUMMY: Self = Fov {
        left: -1.0,
        right: 1.0,
        up: 1.0,
        down: -1.0,
    };
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug)]
pub struct Pose {
    pub orientation: Quat,
    pub position: Vec3,
}

impl Pose {
    pub const IDENTITY: Self = Pose {
        orientation: Quat::IDENTITY,
        position: Vec3::ZERO,
    };

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
    pub const IDENTITY: Self = DeviceMotion {
        pose: Pose::IDENTITY,
        linear_velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
    };

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
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct ViewParams {
    pub pose: Pose,
    pub fov: Fov,
}

impl ViewParams {
    pub const DUMMY: Self = ViewParams {
        pose: Pose::IDENTITY,
        fov: Fov::DUMMY,
    };
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BodySkeletonFb {
    pub upper_body: [Option<Pose>; 18],
    pub lower_body: Option<[Option<Pose>; 14]>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BodySkeletonBd(pub [Option<Pose>; 24]);

#[derive(Serialize, Deserialize, Clone)]
pub enum BodySkeleton {
    Fb(Box<BodySkeletonFb>),
    Bd(Box<BodySkeletonBd>),
}
