use super::{settings::*, *};
use bitflags::bitflags;
use nalgebra::{Point3, UnitQuaternion, Vector3};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

#[derive(Serialize, Deserialize)]
pub struct HandshakePacket {
    pub alvr_name: String,
    pub version: Version,
    pub identity: Option<PublicIdentity>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HeadsetInfoPacket {
    pub device_name: String,
    pub native_eye_resolution: (u32, u32),
    pub native_eyes_fov: [Fov; 2],
    pub native_fps: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ClientConfigPacket {
    pub settings: Settings,
    pub eye_resolution: (u32, u32),
    pub eyes_fov: [Fov; 2],
    pub fps: u32,
    pub web_gui_url: String,
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    Restarting,
    Shutdown,
}

#[derive(Serialize, Deserialize)]
pub struct ClientStatistics {
    pub packets_lost_total: u64,
    pub packets_lost_per_second: u32,

    pub average_total_latency: Duration,
    pub average_transport_latency: Duration,
    pub average_decode_latency: Duration,

    pub fps: u32,
}

#[derive(Serialize, Deserialize)]
pub struct PlayspaceSyncPacket {
    pub position: Point3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub space_rectangle: (f32, f32),
    pub points: Vec<Point3<f32>>,
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    Statistics(ClientStatistics),
    PlayspaceSync(PlayspaceSyncPacket),
    RequestIdrFrame,
    Disconnect,
}

#[derive(Serialize, Deserialize)]
pub struct VideoPacket {
    pub packet_index: u64,
    pub tracking_index: u64,

    #[serde(with = "serde_bytes")]
    pub buffer: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AudioPacket {
    pub packet_index: u64,
    pub presentation_time: Duration,

    #[serde(with = "serde_bytes")]
    pub buffer: Vec<u8>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
#[repr(i8)]
pub enum TrackedDeviceType {
    LeftController,
    RightController,
    Gamepad,
    GenericTracker1,
    GenericTracker2,
    GenericTracker3,
    GenericTracker4,
    GenericTracker5,
    GenericTracker6,
    GenericTracker7,
    GenericTracker8,
    GenericTracker9,
    GenericTracker10,
    GenericTracker11,
    GenericTracker12,
}

#[derive(Serialize, Deserialize)]
pub struct HapticsPacket {
    pub amplitude: f32,
    pub duration: f32,
    pub frequency: f32,
    pub device: TrackedDeviceType,
}

#[derive(Serialize, Deserialize)]
pub struct Pose {
    pub position: Point3<f32>,
    pub orientation: UnitQuaternion<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct MotionDesc {
    pub timestamp: Duration,
    pub pose: Pose,
    pub linear_velocity: Vector3<f32>,
    pub angular_velocity: Vector3<f32>,
}

bitflags! {
    // Target: XBox controller
    #[derive(Serialize, Deserialize)]
    pub struct GamepadDigitalInput: u16 {
        const A = 0x0001;
        const B = 0x0002;
        const X = 0x0004;
        const Y = 0x0008;
        const DPAD_LEFT = 0x0010;
        const DPAD_RIGHT = 0x0020;
        const DPAD_UP = 0x0040;
        const DPAD_DOWN = 0x0080;
        const JOYSTICK_LEFT_CLICK = 0x0100;
        const JOYSTICK_RIGHT_CLICK = 0x0200;
        const SHOULDER_LEFT = 0x0400;
        const SHOULDER_RIGHT = 0x0800;
        const MENU = 0x1000;
        const VIEW = 0x2000;
        const HOME = 0x4000;
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct OculusTouchDigitalInput: u8 {
        const PRIMARY_BUTTON_CLICK = 0x01;
        const PRIMARY_BUTTON_TOUCH = 0x02;
        const SECONDARY_BUTTON_CLICK = 0x04;
        const SECONDARY_BUTTON_TOUCH = 0x08;
        const THUMBSTICK_CLICK = 0x10;
        const THUMBSTICK_TOUCH = 0x20;
        const TRIGGER_TOUCH = 0x40;
        const META = 0x80;
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct OculusHandConfidence: u8 {
        const HAND_HIGH = 0x01;
        const THUMB_HIGH = 0x02;
        const INDEX_HIGH = 0x04;
        const MIDDLE_HIGH = 0x08;
        const RING_HIGH = 0x10;
        const PINKY_HIGH = 0x20;
    }
}

#[derive(Serialize, Deserialize)]
pub struct OculusTouchInput {
    pub thumbstick_coord: (f32, f32),
    pub trigger: f32,
    pub grip: f32,
    pub digital_input: OculusTouchDigitalInput,
    pub battery_percentage: u8,
}

#[derive(Serialize, Deserialize)]
pub struct OculusHand {
    pub bone_rotations: [UnitQuaternion<f32>; 24],
    pub confidence: OculusHandConfidence,
}

#[derive(Serialize, Deserialize)]
pub enum InputDeviceData {
    Gamepad {
        thumbstick_left_coord: (f32, f32),
        thumbstick_right_coord: (f32, f32),
        trigger_left: f32,
        trigger_right: f32,
        digital_input: GamepadDigitalInput,
        battery_percentage: u8,
    },
    OculusTouchPair([OculusTouchInput; 2]),
    OculusHands(Box<[OculusHand; 2]>),
}

#[derive(Serialize, Deserialize)]
pub struct InputPacket {
    pub client_time: u64,
    pub frame_index: u64,

    pub head_motion: MotionDesc,
    pub device_motions: HashMap<TrackedDeviceType, MotionDesc>,
    pub input_data: InputDeviceData,
    pub input_data_timestamp: Duration,

    // some fields are already covered in InputDeviceData. The following fields are used to pass
    // data until the input handling code is rewritten
    pub buttons: [u64; 2],
    pub bone_rotations: [[UnitQuaternion<f32>; 19]; 2],
    pub bone_positions_base: [[Point3<f32>; 19]; 2],
    pub bone_root_oritentation: [UnitQuaternion<f32>; 2],
    pub bone_root_position: [Point3<f32>; 2],
    pub input_state_status: [u32; 2],
    pub finger_pinch_strength: [[f32; 4]; 2],
    pub hand_finger_confidences: [u32; 2],
}
