use alvr_common::Fov;
use nalgebra::{UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub enum TrackerType {
    LeftHand,
    RightHand,
    Generic(usize),
}

pub enum ServerPacket {
    Settings {
        width: u32,
        height: u32,
        fov: [Fov; 2],
        ipd_m: f32,
        fps: f32,
    },
    HeadTrackingData {
        position: Vector3<f32>,
        orientation: UnitQuaternion<f32>,
        target_time_offset: Duration, // controls black pull
        phase_shift: Duration,        // adjusts latency, always positive
    },
    TrackerData {
        tracker_type: TrackerType,
        position: Vector3<f32>,
        orientation: UnitQuaternion<f32>,
        linear_velocity: Vector3<f32>,
        angular_velocity: Vector3<f32>,
        target_time_offset: Duration,
    },
    LayersConsumed,
    Restart,
}

// Note: this can be reused by the vulkan layer
#[derive(Serialize, Deserialize)]
pub struct Layer {
    views: Vec<u64>, // Windows HANDLEs or file descriptors
    orientation: UnitQuaternion<f32>,
}

#[derive(Serialize, Deserialize)]
pub enum DriverPacket {
    Layers(Vec<Layer>),
}
