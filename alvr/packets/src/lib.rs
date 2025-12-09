use alvr_common::{
    BodySkeleton, ConnectionState, DeviceMotion, LogSeverity, Pose, ViewParams,
    anyhow::Result,
    glam::{Quat, UVec2, Vec2},
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
    time::Duration,
};

pub const TRACKING: u16 = 0;
pub const HAPTICS: u16 = 1;
pub const AUDIO: u16 = 2;
pub const VIDEO: u16 = 3;
pub const STATISTICS: u16 = 4;

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoStreamingCapabilitiesExt {
    // Nothing for now
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoStreamingCapabilities {
    pub default_view_resolution: UVec2,
    pub max_view_resolution: UVec2,
    pub refresh_rates: Vec<f32>,
    pub microphone_sample_rate: u32,
    pub foveated_encoding: bool,
    pub encoder_high_profile: bool,
    pub encoder_10_bits: bool,
    pub encoder_av1: bool,
    pub prefer_10bit: bool,
    pub preferred_encoding_gamma: f32,
    pub prefer_hdr: bool,
    pub ext_str: String,
}

impl VideoStreamingCapabilities {
    pub fn with_ext(self, ext: VideoStreamingCapabilitiesExt) -> Self {
        Self {
            ext_str: json::to_string(&ext).unwrap(),
            ..self
        }
    }

    pub fn ext(&self) -> Result<VideoStreamingCapabilitiesExt> {
        let _ext_json = json::from_str::<json::Value>(&self.ext_str)?;

        // decode values here

        Ok(VideoStreamingCapabilitiesExt {})
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConnectionAcceptedInfo {
    pub client_protocol_id: u64,
    pub platform_string: String,
    pub server_ip: IpAddr,
    pub streaming_capabilities: Option<VideoStreamingCapabilities>,
}

#[derive(Serialize, Deserialize)]
pub enum ClientConnectionResult {
    ConnectionAccepted(Box<ConnectionAcceptedInfo>),
    ClientStandby,
}

#[derive(Serialize, Deserialize)]
pub struct NegotiatedStreamingConfigExt {
    // Nothing for now
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NegotiatedStreamingConfig {
    pub view_resolution: UVec2,
    pub refresh_rate_hint: f32,
    pub game_audio_sample_rate: u32,
    pub enable_foveated_encoding: bool,
    pub encoding_gamma: f32,
    pub enable_hdr: bool,
    pub wired: bool,
    pub ext_str: String,
}

impl NegotiatedStreamingConfig {
    pub fn with_ext(self, ext: NegotiatedStreamingConfigExt) -> Self {
        Self {
            ext_str: json::to_string(&ext).unwrap(),
            ..self
        }
    }

    pub fn ext(&self) -> Result<NegotiatedStreamingConfigExt> {
        let _ext_json = json::from_str::<json::Value>(&self.ext_str)?;

        // decode values here

        Ok(NegotiatedStreamingConfigExt {})
    }
}

#[derive(Serialize, Deserialize)]
pub struct StreamConfigPacket {
    pub session: String, // JSON session that allows for extrapolation
    pub negotiated: NegotiatedStreamingConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StreamConfig {
    pub server_version: Version,
    pub settings: Settings,
    pub negotiated_config: NegotiatedStreamingConfig,
}

impl StreamConfigPacket {
    pub fn new(session: &SessionConfig, negotiated: NegotiatedStreamingConfig) -> Result<Self> {
        Ok(Self {
            session: json::to_string(session)?,
            negotiated,
        })
    }

    pub fn to_stream_config(self) -> Result<StreamConfig> {
        let mut session_config = SessionConfig::default();
        session_config.merge_from_json(&json::from_str(&self.session)?)?;
        let settings = session_config.to_settings();

        Ok(StreamConfig {
            server_version: session_config.server_version,
            settings,
            negotiated_config: self.negotiated,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DecoderInitializationConfig {
    pub codec: CodecType,
    pub config_buffer: Vec<u8>, // e.g. SPS + PPS NALs
    pub ext_str: String,
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    StartStream,
    DecoderConfig(DecoderInitializationConfig),
    Restarting,
    KeepAlive,
    RealTimeConfig(RealTimeConfig),
    Reserved(String),
    ReservedBuffer(Vec<u8>),
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

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    PlayspaceSync(Option<Vec2>),
    RequestIdr,
    KeepAlive,
    StreamReady, // This flag notifies the server the client streaming socket is ready listening
    LocalViewParams([ViewParams; 2]), // In relation to head
    Battery(BatteryInfo),
    Buttons(Vec<ButtonEntry>),
    ActiveInteractionProfile {
        device_id: u64,
        profile_id: u64,
        input_ids: HashSet<u64>,
    },
    Log {
        level: LogSeverity,
        message: String,
    },
    ProximityState(bool),
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FaceExpressions {
    Fb(Vec<f32>),   // 70 values
    Pico(Vec<f32>), // 52 values
    Htc {
        eye: Option<Vec<f32>>, // 14 values
        lip: Option<Vec<f32>>, // 37 values
    },
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct FaceData {
    // Can be used for foveated eye tracking
    pub eyes_combined: Option<Quat>,
    // Should be used only for social presence
    pub eyes_social: [Option<Quat>; 2],

    pub face_expressions: Option<FaceExpressions>,
}

#[derive(Serialize, Deserialize)]
pub struct TrackingData {
    pub poll_timestamp: Duration,
    pub device_motions: Vec<(u64, DeviceMotion)>,
    pub hand_skeletons: [Option<[Pose; 26]>; 2],
    pub face: FaceData,
    pub body: Option<BodySkeleton>,
    pub markers: Vec<(String, Pose)>,
}

#[derive(Serialize, Deserialize)]
pub struct VideoPacketHeader {
    pub timestamp: Duration,
    pub global_view_params: [ViewParams; 2],
    pub is_idr: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Haptics {
    pub device_id: u64,
    pub duration: Duration,
    pub frequency: f32,
    pub amplitude: f32,
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
pub enum ClientConnectionsAction {
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

// Note: server sends a packet to the client at low frequency, binary encoding, without ensuring
// compatibility between different versions, even if within the same major version.
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct RealTimeConfig {
    pub passthrough: Option<PassthroughMode>,
    pub clientside_post_processing: Option<ClientsidePostProcessingConfig>,
    pub ext_str: String,
}

impl RealTimeConfig {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            passthrough: settings.video.passthrough.clone().into_option(),
            clientside_post_processing: settings
                .video
                .clientside_post_processing
                .clone()
                .into_option(),
            ext_str: String::new(), // No extensions for now
        }
    }
}
