use std::{net::IpAddr, time::Duration};

use alvr_common::{
    glam::{Quat, Vec2, Vec3},
    semver::Version,
};
use alvr_events::ButtonValue;
use serde::{Deserialize, Serialize};

pub const TRACKING: u16 = 0;
pub const HAPTICS: u16 = 1;
pub const AUDIO: u16 = 2;
pub const VIDEO: u16 = 3;
pub const STATISTICS: u16 = 4;

// Field of view in radians
#[derive(Serialize, Deserialize, PartialEq, Default, Clone, Copy)]
pub struct Fov {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

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
    pub microphone_sample_rate: u32,

    // reserved field is used to add features in a minor release that otherwise would break the
    // packets schema
    pub reserved: String,
}

#[derive(Serialize, Deserialize)]
pub enum ClientConnectionResult {
    ServerAccepted {
        headset_info: HeadsetInfoPacket,
        server_ip: IpAddr,
    },
    ClientStandby,
}

#[derive(Serialize, Deserialize)]
pub struct ClientConfigPacket {
    pub session_desc: String, // transfer session as string to allow for extrapolation
    pub dashboard_url: String,
    pub view_resolution_width: u32,
    pub view_resolution_height: u32,
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
    Button { path_id: u64, value: ButtonValue },
    ActiveInteractionProfile { device_id: u64, profile_id: u64 },
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

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct DeviceMotion {
    pub orientation: Quat,
    pub position: Vec3,
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

pub enum ClientListAction {
    AddIfMissing { display_name: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ClientStatistics {
    pub target_timestamp: Duration, // identifies the frame
    pub frame_interval: Duration,
    pub video_decode: Duration,
    pub rendering: Duration,
    pub vsync_queue: Duration,
    pub total_pipeline_latency: Duration,

    // Note: This is used for the controller prediction.
    // NB: This contains also the tracking packet send latency so it might lead to overprediction
    pub average_total_pipeline_latency: Duration,
}
