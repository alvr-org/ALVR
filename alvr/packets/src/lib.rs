use alvr_common::{
    ConnectionState, DeviceMotion, Fov, LogEntry, LogSeverity, Pose,
    anyhow::Result,
    glam::{UVec2, Vec2},
    semver::Version,
};
use alvr_session::{
    ClientsidePostProcessingConfig, CodecType, PassthroughMode, SessionConfig, Settings,
};
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::{
    collections::HashSet,
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

// Note: not a network packet
#[derive(Serialize, Deserialize, Clone)]
pub struct VideoStreamingCapabilities {
    pub default_view_resolution: UVec2,
    pub refresh_rates: Vec<f32>,
    pub microphone_sample_rate: u32,
    pub foveated_encoding: bool,
    pub encoder_high_profile: bool,
    pub encoder_10_bits: bool,
    pub encoder_av1: bool,
    pub prefer_10bit: bool,
    pub preferred_encoding_gamma: f32,
    pub prefer_hdr: bool,
}

pub fn decode_video_streaming_capabilities(cap_str: &str) -> Result<VideoStreamingCapabilities> {
    let caps_json = json::from_str::<json::Value>(cap_str)?;

    Ok(VideoStreamingCapabilities {
        default_view_resolution: json::from_value(caps_json["default_view_resolution"].clone())?,
        refresh_rates: json::from_value(caps_json["refresh_rates"].clone())?,
        microphone_sample_rate: json::from_value(caps_json["microphone_sample_rate"].clone())?,
        foveated_encoding: json::from_value(caps_json["foveated_encoding"].clone())?,
        encoder_high_profile: json::from_value(caps_json["encoder_high_profile"].clone())?,
        encoder_10_bits: json::from_value(caps_json["encoder_10_bits"].clone())?,
        encoder_av1: json::from_value(caps_json["encoder_av1"].clone())?,
        prefer_10bit: json::from_value(caps_json["prefer_10bit"].clone())?,
        preferred_encoding_gamma: json::from_value(caps_json["preferred_encoding_gamma"].clone())?,
        prefer_hdr: json::from_value(caps_json["prefer_hdr"].clone())?,
    })
}

#[derive(Serialize, Deserialize)]
pub enum ClientConnectionResult {
    ConnectionAccepted {
        client_protocol_id: u64,
        display_name: String,
        server_ip: IpAddr,
        streaming_capabilities: Option<String>,
    },
    ClientStandby,
}

// Note: not a network packet
#[derive(Serialize, Deserialize, Clone)]
pub struct NegotiatedStreamingConfig {
    pub view_resolution: UVec2,
    pub refresh_rate_hint: f32,
    pub game_audio_sample_rate: u32,
    pub enable_foveated_encoding: bool,
    pub encoding_gamma: f32,
    pub enable_hdr: bool,
    pub wired: bool,
}

#[derive(Serialize, Deserialize)]
pub struct StreamConfigPacket {
    pub session: String,    // JSON session that allows for extrapolation
    pub negotiated: String, // Encoded NegotiatedVideoStreamingConfig
}

pub fn encode_stream_config(
    session: &SessionConfig,
    negotiated: &NegotiatedStreamingConfig,
) -> Result<StreamConfigPacket> {
    Ok(StreamConfigPacket {
        session: json::to_string(session)?,
        negotiated: json::to_string(negotiated)?,
    })
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StreamConfig {
    pub server_version: Version,
    pub settings: Settings,
    pub negotiated_config: NegotiatedStreamingConfig,
}

pub fn decode_stream_config(packet: &StreamConfigPacket) -> Result<StreamConfig> {
    let mut session_config = SessionConfig::default();
    session_config.merge_from_json(&json::from_str(&packet.session)?)?;
    let settings = session_config.to_settings();

    let neg_json = json::from_str::<json::Value>(&packet.negotiated)?;

    Ok(StreamConfig {
        server_version: session_config.server_version,
        settings,
        negotiated_config: NegotiatedStreamingConfig {
            view_resolution: json::from_value(neg_json["view_resolution"].clone())?,
            refresh_rate_hint: json::from_value(neg_json["refresh_rate_hint"].clone())?,
            game_audio_sample_rate: json::from_value(neg_json["game_audio_sample_rate"].clone())?,
            enable_foveated_encoding: json::from_value(
                neg_json["enable_foveated_encoding"].clone(),
            )?,
            encoding_gamma: json::from_value(neg_json["encoding_gamma"].clone())?,
            enable_hdr: json::from_value(neg_json["enable_hdr"].clone())?,
            wired: json::from_value(neg_json["wired"].clone())?,
        },
    })
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DecoderInitializationConfig {
    pub codec: CodecType,
    pub config_buffer: Vec<u8>, // e.g. SPS + PPS NALs
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    StartStream,
    DecoderConfig(DecoderInitializationConfig),
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
pub struct BatteryInfo {
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

// to be de/serialized with ClientControlPacket::Reserved()
#[derive(Serialize, Deserialize)]
pub enum ReservedClientControlPacket {
    CustomInteractionProfile {
        device_id: u64,
        input_ids: HashSet<u64>,
    },
}

pub fn encode_reserved_client_control_packet(
    packet: &ReservedClientControlPacket,
) -> ClientControlPacket {
    ClientControlPacket::Reserved(json::to_string(packet).unwrap())
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    PlayspaceSync(Option<Vec2>),
    RequestIdr,
    KeepAlive,
    StreamReady, // This flag notifies the server the client streaming socket is ready listening
    ViewsConfig(ViewsConfig),
    Battery(BatteryInfo),
    Buttons(Vec<ButtonEntry>),
    ActiveInteractionProfile { device_id: u64, profile_id: u64 },
    Log { level: LogSeverity, message: String },
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Default)]
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
            PathSegment::Name(name) => write!(f, "{name}"),
            PathSegment::Index(index) => write!(f, "[{index}]"),
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
    pub value: json::Value,
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

// Note: server sends a packet to the client at low frequency, binary encoding, without ensuring
// compatibility between different versions, even if within the same major version.
#[derive(Serialize, Deserialize)]
pub struct RealTimeConfig {
    pub passthrough: Option<PassthroughMode>,
    pub clientside_post_processing: Option<ClientsidePostProcessingConfig>,
}

impl RealTimeConfig {
    pub fn encode(&self) -> Result<ServerControlPacket> {
        Ok(ServerControlPacket::ReservedBuffer(bincode::serialize(
            self,
        )?))
    }

    pub fn decode(buffer: &[u8]) -> Result<Self> {
        Ok(bincode::deserialize(buffer)?)
    }

    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            passthrough: settings.video.passthrough.clone().into_option(),
            clientside_post_processing: settings
                .video
                .clientside_post_processing
                .clone()
                .into_option(),
        }
    }
}
