use alvr_common::{
    glam::{UVec2, Vec2},
    ConnectionState, DeviceMotion, Fov, LogEntry, LogSeverity, Pose,
};
use alvr_session::{CodecType, SessionConfig};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Debug},
    net::IpAddr,
    path::PathBuf,
    time::Duration,
};

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
        client_protocol_id: u64,
        display_name: String,
        server_ip: IpAddr,
        streaming_capabilities: Option<VideoStreamingCapabilities>,
    },
    ClientStandby,
}

#[derive(Serialize, Deserialize)]
pub struct StreamConfigPacket {
    pub session: String, // JSON session that allows for extrapolation
    pub negotiated: String, // JSON dictionary containing negotiated configuration. Can be extended
                         // without a breaking protocol change, but entries can't be removed.
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DecoderInitializationConfig {
    pub codec: CodecType,
    pub config_buffer: Vec<u8>, // e.g. SPS + PPS NALs
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    StartStream,
    InitializeDecoder(DecoderInitializationConfig),
    Restarting,
    KeepAlive,
    ServerPredictionAverage(Duration), // todo: remove
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ButtonValue {
    Binary(bool),
    Scalar(f32),
}

#[derive(Serialize, Deserialize)]
pub struct ButtonEntry {
    pub path_id: u64,
    pub value: ButtonValue,
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    PlayspaceSync(Option<Vec2>),
    RequestIdr,
    KeepAlive,
    StreamReady, // This flag notifies the server the client streaming socket is ready listening
    ViewsConfig(ViewsConfig),
    Battery(BatteryPacket),
    VideoErrorReport, // legacy
    Buttons(Vec<ButtonEntry>),
    ActiveInteractionProfile { device_id: u64, profile_id: u64 },
    Log { level: LogSeverity, message: String },
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize, Default)]
pub struct FaceData {
    pub eye_gazes: [Option<Pose>; 2],
    pub fb_face_expression: Option<Vec<f32>>, // issue: Serialize does not support [f32; 63]
    pub htc_eye_expression: Option<Vec<f32>>,
    pub htc_lip_expression: Option<Vec<f32>>, // issue: Serialize does not support [f32; 37]
}

#[derive(Serialize, Deserialize)]
pub struct VideoPacketHeader {
    pub timestamp: Duration,
    pub is_idr: bool,
}

// Note: face_data does not respect target_timestamp.
#[derive(Serialize, Deserialize, Default)]
pub struct Tracking {
    pub target_timestamp: Duration,
    pub device_motions: Vec<(u64, DeviceMotion)>,
    pub hand_skeletons: [Option<[Pose; 26]>; 2],
    pub face_data: FaceData,
}

#[derive(Serialize, Deserialize)]
pub struct Haptics {
    pub device_id: u64,
    pub duration: Duration,
    pub frequency: f32,
    pub amplitude: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioDevicesList {
    pub output: Vec<String>,
    pub input: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum PathSegment {
    Name(String),
    Index(usize),
}

impl Debug for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::Name(name) => write!(f, "{}", name),
            PathSegment::Index(index) => write!(f, "[{}]", index),
        }
    }
}

impl From<&str> for PathSegment {
    fn from(value: &str) -> Self {
        PathSegment::Name(value.to_owned())
    }
}

impl From<String> for PathSegment {
    fn from(value: String) -> Self {
        PathSegment::Name(value)
    }
}

impl From<usize> for PathSegment {
    fn from(value: usize) -> Self {
        PathSegment::Index(value)
    }
}

// todo: support indices
pub fn parse_path(path: &str) -> Vec<PathSegment> {
    path.split('.').map(|s| s.into()).collect()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientListAction {
    AddIfMissing {
        trusted: bool,
        manual_ips: Vec<IpAddr>,
    },
    SetDisplayName(String),
    Trust,
    SetManualIps(Vec<IpAddr>),
    RemoveEntry,
    UpdateCurrentIp(Option<IpAddr>),
    SetConnectionState(ConnectionState),
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathValuePair {
    pub path: Vec<PathSegment>,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FirewallRulesAction {
    Add,
    Remove,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerRequest {
    Log(LogEntry),
    GetSession,
    UpdateSession(Box<SessionConfig>),
    SetValues(Vec<PathValuePair>),
    UpdateClientList {
        hostname: String,
        action: ClientListAction,
    },
    GetAudioDevices,
    CaptureFrame,
    InsertIdr,
    StartRecording,
    StopRecording,
    FirewallRules(FirewallRulesAction),
    RegisterAlvrDriver,
    UnregisterDriver(PathBuf),
    GetDriverList,
    RestartSteamvr,
    ShutdownSteamvr,
}
