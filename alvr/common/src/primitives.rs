use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::ops::Mul;

// Field of view in radians
#[derive(Serialize, Deserialize, PartialEq, Default, Debug, Clone, Copy)]
pub struct Fov {
    pub left: f32,
    pub right: f32,
    pub up: f32,
    pub down: f32,
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
