use std::{collections::HashMap, time::Duration};

use crate::StreamId;
use alvr_common::{
    glam::{Quat, Vec2, Vec3},
    semver::Version,
    Fov, MotionData,
};
use serde::{Deserialize, Serialize};

pub const INPUT: StreamId = 0; // tracking and buttons
pub const HAPTICS: StreamId = 1;
pub const AUDIO: StreamId = 2;
pub const VIDEO: StreamId = 3;

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientHandshakePacket {
    pub alvr_name: String,
    pub version: Version,
    pub device_name: String,
    pub hostname: String,

    // reserved field is used to add features between major releases: the schema of the packet
    // should never change anymore (required only for this packet).
    pub reserved1: String,
    pub reserved2: String,
}

// Since this packet is not essential, any change to it will not be a braking change
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerHandshakePacket {
    ClientUntrusted,
    IncompatibleVersions,
}

#[derive(Serialize, Deserialize)]
pub enum HandshakePacket {
    Client(ClientHandshakePacket),
    Server(ServerHandshakePacket),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HeadsetInfoPacket {
    pub recommended_eye_width: u32,
    pub recommended_eye_height: u32,
    pub available_refresh_rates: Vec<f32>,
    pub preferred_refresh_rate: f32,

    // reserved field is used to add features in a minor release that otherwise would break the
    // packets schema
    pub reserved: String,
}

#[derive(Serialize, Deserialize)]
pub struct ClientConfigPacket {
    pub session_desc: String, // transfer session as string to allow for extrapolation
    pub dashboard_url: String,
    pub eye_resolution_width: u32,
    pub eye_resolution_height: u32,
    pub fps: f32,
    pub game_audio_sample_rate: u32,
    pub reserved: String,
    pub server_version: Option<Version>,
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    StartStream,
    Restarting,
    KeepAlive,
    TimeSync(TimeSyncPacket), // legacy
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize)]
pub struct PlayspaceSyncPacket {
    pub position: Vec3,
    pub rotation: Quat,
    pub area_width: f32,
    pub area_height: f32,
    pub perimeter_points: Option<Vec<Vec2>>,
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    PlayspaceSync(PlayspaceSyncPacket),
    RequestIdr,
    KeepAlive,
    StreamReady,
    TimeSync(TimeSyncPacket), // legacy
    VideoErrorReport,         // legacy
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

// legacy video packet
#[derive(Serialize, Deserialize, Clone)]
pub struct VideoFrameHeaderPacket {
    pub packet_counter: u32,
    pub tracking_frame_index: u64,
    pub video_frame_index: u64,
    pub sent_time: u64,
    pub frame_byte_size: u32,
    pub fec_index: u32,
    pub fec_percentage: u16,
}

// legacy time sync packet
#[derive(Serialize, Deserialize, Default)]
pub struct TimeSyncPacket {
    pub mode: u32,
    pub server_time: u64,
    pub client_time: u64,
    pub packets_lost_total: u64,
    pub packets_lost_in_second: u64,
    pub average_send_latency: u32,
    pub average_transport_latency: u32,
    pub average_decode_latency: u64,
    pub idle_time: u32,
    pub fec_failure: u32,
    pub fec_failure_in_second: u64,
    pub fec_failure_total: u64,
    pub fps: f32,
    pub server_total_latency: u32,
    pub tracking_recv_frame_index: u64,
}

#[derive(Serialize, Deserialize)]
pub enum ButtonValue {
    Binary(bool),
    Scalar(f32),
}

#[derive(Serialize, Deserialize)]
pub struct HandTrackingInput {
    pub target_ray_motion: MotionData,
    pub skeleton_motion: Vec<MotionData>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ViewsConfig {
    // Note: the head-to-eye transform is always a translation along the x axis
    pub ipd_m: f32,
    pub fov: [Fov; 2],
}

#[derive(Serialize, Deserialize, Default)]
pub struct LegacyInput {
    pub flags: u32,
    pub client_time: u64,
    pub frame_index: u64,
    pub battery: u64,
    pub plugged: u8,
    pub mounted: u8,
    pub controller_flags: [u32; 2],
    pub buttons: [u64; 2],
    pub trackpad_position: [Vec2; 2],
    pub trigger_value: [f32; 2],
    pub grip_value: [f32; 2],
    pub controller_battery: [u8; 2],
    pub bone_rotations: [[Quat; 19]; 2],
    pub bone_positions_base: [[Vec3; 19]; 2],
    pub input_state_status: [u32; 2],
    pub finger_pinch_strengths: [[f32; 4]; 2],
    pub hand_finger_confience: [u32; 2],
}

#[derive(Serialize, Deserialize)]
pub struct Input {
    pub target_timestamp: Duration,
    pub device_motions: Vec<(u64, MotionData)>,
    pub left_hand_tracking: Option<HandTrackingInput>, // unused for now
    pub right_hand_tracking: Option<HandTrackingInput>, // unused for now
    pub views_config: ViewsConfig,
    pub button_values: HashMap<u64, ButtonValue>, // unused for now
    pub legacy: LegacyInput,
}
