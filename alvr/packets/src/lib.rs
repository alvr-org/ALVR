use alvr_common::{
    anyhow::Result,
    glam::{UVec2, Vec2},
    ConnectionState, DeviceMotion, Fov, LogEntry, LogSeverity, Pose, ToAny,
};
use alvr_session::{CodecType, SessionConfig, Settings};
use serde::{Deserialize, Serialize};
use serde_json as json;
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

// todo: use simple string
#[derive(Serialize, Deserialize, Clone)]
pub struct VideoStreamingCapabilitiesLegacy {
    pub default_view_resolution: UVec2,
    pub supported_refresh_rates_plus_extra_data: Vec<f32>,
    pub microphone_sample_rate: u32,
}

// Note: not a network packet
#[derive(Serialize, Deserialize, Clone)]
pub struct VideoStreamingCapabilities {
    pub default_view_resolution: UVec2,
    pub supported_refresh_rates: Vec<f32>, // todo rename
    pub microphone_sample_rate: u32,
    pub supports_foveated_encoding: bool, // todo rename
    pub encoder_high_profile: bool,
    pub encoder_10_bits: bool,
    pub encoder_av1: bool,
}

// Nasty workaround to make the packet extensible, pushing the limits of protocol compatibility
// Todo: replace VideoStreamingCapabilitiesLegacy with simple json string
pub fn encode_video_streaming_capabilities(
    caps: &VideoStreamingCapabilities,
) -> Result<VideoStreamingCapabilitiesLegacy> {
    let caps_json = json::to_value(caps)?;

    let mut supported_refresh_rates_plus_extra_data = vec![];
    for rate in caps_json["supported_refresh_rates"].as_array().to_any()? {
        supported_refresh_rates_plus_extra_data.push(rate.as_f64().to_any()? as f32);
    }
    for byte in json::to_string(caps)?.as_bytes() {
        // using negative values is not going to trigger strange behavior for old servers
        supported_refresh_rates_plus_extra_data.push(-(*byte as f32));
    }

    let default_view_resolution = json::from_value(caps_json["default_view_resolution"].clone())?;
    let microphone_sample_rate = caps_json["microphone_sample_rate"].as_u64().to_any()? as u32;

    Ok(VideoStreamingCapabilitiesLegacy {
        default_view_resolution,
        supported_refresh_rates_plus_extra_data,
        microphone_sample_rate,
    })
}

pub fn decode_video_streaming_capabilities(
    legacy: &VideoStreamingCapabilitiesLegacy,
) -> Result<VideoStreamingCapabilities> {
    let mut json_bytes = vec![];
    let mut supported_refresh_rates = vec![];
    for rate in &legacy.supported_refresh_rates_plus_extra_data {
        if *rate < 0.0 {
            json_bytes.push((-*rate) as u8)
        } else {
            supported_refresh_rates.push(*rate);
        }
    }

    let caps_json = json::from_str::<json::Value>(&String::from_utf8(json_bytes)?)?;

    Ok(VideoStreamingCapabilities {
        default_view_resolution: legacy.default_view_resolution,
        supported_refresh_rates,
        microphone_sample_rate: legacy.microphone_sample_rate,
        supports_foveated_encoding: caps_json["supports_foveated_encoding"]
            .as_bool()
            .unwrap_or(true),
        encoder_high_profile: caps_json["encoder_high_profile"].as_bool().unwrap_or(true),
        encoder_10_bits: caps_json["encoder_10_bits"].as_bool().unwrap_or(true),
        encoder_av1: caps_json["encoder_av1"].as_bool().unwrap_or(true),
    })
}

#[derive(Serialize, Deserialize)]
pub enum ClientConnectionResult {
    ConnectionAccepted {
        client_protocol_id: u64,
        display_name: String,
        server_ip: IpAddr,
        streaming_capabilities: Option<VideoStreamingCapabilitiesLegacy>, // todo: use String
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

pub fn decode_stream_config(
    packet: &StreamConfigPacket,
) -> Result<(Settings, NegotiatedStreamingConfig)> {
    let mut session_config = SessionConfig::default();
    session_config.merge_from_json(&json::from_str(&packet.session)?)?;
    let settings = session_config.to_settings();

    let negotiated_json = json::from_str::<json::Value>(&packet.negotiated)?;

    let view_resolution = json::from_value(negotiated_json["view_resolution"].clone())?;
    let refresh_rate_hint = json::from_value(negotiated_json["refresh_rate_hint"].clone())?;
    let game_audio_sample_rate =
        json::from_value(negotiated_json["game_audio_sample_rate"].clone())?;
    let enable_foveated_encoding =
        json::from_value(negotiated_json["enable_foveated_encoding"].clone())
            .unwrap_or_else(|_| settings.video.foveated_encoding.enabled());

    Ok((
        settings,
        NegotiatedStreamingConfig {
            view_resolution,
            refresh_rate_hint,
            game_audio_sample_rate,
            enable_foveated_encoding,
        },
    ))
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
