use alvr_common::{info, DeviceMotion, LogEntry, LogSeverity, Pose};
use alvr_packets::{AudioDevicesList, ButtonValue};
use alvr_session::SessionConfig;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StatisticsSummary {
    pub video_packets_total: usize,
    pub video_packets_per_sec: usize,
    pub video_mbytes_total: usize,
    pub video_mbits_per_sec: f32,
    pub total_latency_ms: f32,
    pub network_latency_ms: f32,
    pub encode_latency_ms: f32,
    pub decode_latency_ms: f32,
    pub packets_lost_total: usize,
    pub packets_lost_per_sec: usize,
    pub client_fps: u32,
    pub server_fps: u32,
    pub battery_hmd: u32,
    pub hmd_plugged: bool,
}

// Bitrate statistics minus the empirical output value
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BitrateDirectives {
    pub scaled_calculated_throughput_bps: Option<f32>,
    pub decoder_latency_limiter_bps: Option<f32>,
    pub network_latency_limiter_bps: Option<f32>,
    pub encoder_latency_limiter_bps: Option<f32>,
    pub manual_max_throughput_bps: Option<f32>,
    pub manual_min_throughput_bps: Option<f32>,
    pub requested_bitrate_bps: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GraphStatistics {
    pub total_pipeline_latency_s: f32,
    pub game_time_s: f32,
    pub server_compositor_s: f32,
    pub encoder_s: f32,
    pub network_s: f32,
    pub decoder_s: f32,
    pub decoder_queue_s: f32,
    pub client_compositor_s: f32,
    pub vsync_queue_s: f32,
    pub client_fps: f32,
    pub server_fps: f32,
    pub bitrate_directives: BitrateDirectives,
    pub throughput_bps: f32,
    pub bitrate_bps: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackingEvent {
    pub device_motions: Vec<(String, DeviceMotion)>,
    pub hand_skeletons: [Option<[Pose; 26]>; 2],
    pub eye_gazes: [Option<Pose>; 2],
    pub fb_face_expression: Option<Vec<f32>>,
    pub htc_eye_expression: Option<Vec<f32>>,
    pub htc_lip_expression: Option<Vec<f32>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ButtonEvent {
    pub path: String,
    pub value: ButtonValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HapticsEvent {
    pub path: String,
    pub duration: Duration,
    pub frequency: f32,
    pub amplitude: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AdbEvent {
    pub download_progress: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "id", content = "data")]
pub enum EventType {
    Log(LogEntry),
    DebugGroup { group: String, message: String },
    Session(Box<SessionConfig>),
    StatisticsSummary(StatisticsSummary),
    GraphStatistics(GraphStatistics),
    Tracking(Box<TrackingEvent>),
    Buttons(Vec<ButtonEvent>),
    Haptics(HapticsEvent),
    AudioDevices(AudioDevicesList),
    DriversList(Vec<PathBuf>),
    ServerRequestsSelfRestart,
    Adb(AdbEvent),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub timestamp: String,
    pub event_type: EventType,
}

impl Event {
    pub fn event_type_string(&self) -> String {
        match &self.event_type {
            EventType::Log(entry) => match entry.severity {
                LogSeverity::Error => "ERROR".into(),
                LogSeverity::Warning => "WARNING".into(),
                LogSeverity::Info => "INFO".into(),
                LogSeverity::Debug => "DEBUG".into(),
            },
            EventType::DebugGroup { group, .. } => group.clone(),
            EventType::Session(_) => "SESSION".to_string(),
            EventType::StatisticsSummary(_) => "STATS".to_string(),
            EventType::GraphStatistics(_) => "GRAPH".to_string(),
            EventType::Tracking(_) => "TRACKING".to_string(),
            EventType::Buttons(_) => "BUTTONS".to_string(),
            EventType::Haptics(_) => "HAPTICS".to_string(),
            EventType::AudioDevices(_) => "AUDIO DEV".to_string(),
            EventType::DriversList(_) => "DRV LIST".to_string(),
            EventType::ServerRequestsSelfRestart => "RESTART".to_string(),
            EventType::Adb(_) => "ADB".to_string(),
        }
    }

    pub fn message(&self) -> String {
        match &self.event_type {
            EventType::Log(log_entry) => log_entry.content.clone(),
            EventType::DebugGroup { message, .. } => message.clone(),
            EventType::Session(_) => "Updated".into(),
            EventType::StatisticsSummary(_) => "".into(),
            EventType::GraphStatistics(_) => "".into(),
            EventType::Tracking(tracking) => serde_json::to_string(tracking).unwrap(),
            EventType::Buttons(buttons) => serde_json::to_string(buttons).unwrap(),
            EventType::Haptics(haptics) => serde_json::to_string(haptics).unwrap(),
            EventType::AudioDevices(devices) => serde_json::to_string(devices).unwrap(),
            EventType::DriversList(drivers) => serde_json::to_string(drivers).unwrap(),
            EventType::ServerRequestsSelfRestart => "Request for server restart".into(),
            EventType::Adb(adb) => serde_json::to_string(adb).unwrap(),
        }
    }
}

pub fn send_event(event_type: EventType) {
    info!("{}", serde_json::to_string(&event_type).unwrap());
}
