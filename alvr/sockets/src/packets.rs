use std::{net::IpAddr, time::Duration};

use alvr_common::{
    glam::{Quat, UVec2, Vec2, Vec3},
    Fov,
};
use alvr_events::{ButtonValue, EventSeverity};
use serde::{Deserialize, Serialize};

pub const TRACKING: u16 = 0;
pub const HAPTICS: u16 = 1;
pub const AUDIO: u16 = 2;
pub const VIDEO: u16 = 3;
pub const STATISTICS: u16 = 4;

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoStreamingCapabilities {
    pub default_view_resolution: UVec2,
    pub supported_refresh_rates: Vec<f32>,
    pub microphone_sample_rate: u32,
}

#[derive(Serialize, Deserialize)]
pub enum ClientConnectionResult {
    ConnectionAccepted {
        display_name: String,
        server_ip: IpAddr,
        streaming_capabilities: Option<VideoStreamingCapabilities>,
    },
    ClientStandby,
}

#[derive(Serialize, Deserialize)]
pub struct StreamConfigPacket {
    pub session_desc: String, // transfer session as string to allow for extrapolation
    pub view_resolution: UVec2,
    pub fps: f32,
    pub game_audio_sample_rate: u32,
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    StartStream,
    InitializeDecoder { config_buffer: Vec<u8> },
    Restarting,
    KeepAlive,
    ServerPredictionAverage(Duration),
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ViewsConfig {
    // Note: the head-to-eye transform is always a translation along the x axis
    pub ipd_m: f32,
    pub fov: [Fov; 2],
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BatteryPacket {
    pub device_id: u64,
    pub gauge_value: f32, // range [0, 1]
    pub is_plugged: bool,
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    PlayspaceSync(Vec2),
    RequestIdr,
    KeepAlive,
    StreamReady,
    ViewsConfig(ViewsConfig),
    Battery(BatteryPacket),
    VideoErrorReport, // legacy
    Button {
        path_id: u64,
        value: ButtonValue,
    },
    ActiveInteractionProfile {
        device_id: u64,
        profile_id: u64,
    },
    Log {
        level: EventSeverity,
        message: String,
    },
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Pose {
    pub orientation: Quat,
    pub position: Vec3,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct DeviceMotion {
    pub pose: Pose,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}

#[derive(Serialize, Deserialize)]
pub struct Tracking {
    pub target_timestamp: Duration,
    pub device_motions: Vec<(u64, DeviceMotion)>,
    pub left_hand_skeleton: Option<[Quat; 19]>, // legacy oculus hand
    pub right_hand_skeleton: Option<[Quat; 19]>, // legacy oculus hand
}

#[derive(Serialize, Deserialize)]
pub struct Haptics {
    pub path: u64,
    pub duration: Duration,
    pub frequency: f32,
    pub amplitude: f32,
}

#[derive(Serialize, Deserialize)]
pub struct AudioDevicesList {
    pub output: Vec<String>,
    pub input: Vec<String>,
}

pub enum GpuVendor {
    Nvidia,
    Amd,
    Other,
}

#[derive(Clone, Debug)]
pub enum PathSegment {
    Name(String),
    Index(usize),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ClientListAction {
    AddIfMissing,
    SetDisplayName(String),
    Trust,
    AddIp(IpAddr),
    RemoveIp(IpAddr),
    RemoveEntry,
    UpdateCurrentIp(Option<IpAddr>),
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ClientStatistics {
    pub target_timestamp: Duration, // identifies the frame
    pub frame_interval: Duration,
    pub video_decode: Duration,
    pub video_decoder_queue: Duration,
    pub rendering: Duration,
    pub vsync_queue: Duration,
    pub total_pipeline_latency: Duration,
}
