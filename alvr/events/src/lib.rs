use alvr_common::prelude::*;
use alvr_session::SessionDesc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventSeverity {
    Error,
    Warning,
    Info,
    Debug,
}

// todo: remove some unused statistics
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")] // todo: remove casing conversion
pub struct Statistics {
    pub video_packets_total: usize,
    pub video_packets_per_sec: usize,
    pub video_mbytes_total: usize,
    pub video_mbits_per_sec: f32,
    pub total_latency_ms: f32,
    pub network_latency_ms: f32,
    pub encode_latency_ms: f32,
    pub decode_latency_ms: f32,
    pub fec_percentage: u32,
    pub fec_errors_total: usize,
    pub fec_errors_per_sec: usize,
    pub client_fps: u32, // the name will be fixed after the old dashboard is removed
    pub server_fps: u32,
    pub battery_hmd: u32,
    pub battery_left: u32,
    pub battery_right: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")] // todo: remove casing conversion
pub struct GraphStatistics {
    pub total_pipeline_latency_s: f32,
    pub game_time_s: f32,
    pub server_compositor_s: f32,
    pub encoder_s: f32,
    pub network_s: f32,
    pub decoder_s: f32,
    pub client_compositor_s: f32,
    pub vsync_queue_s: f32,
    pub client_fps: f32,
    pub server_fps: f32,
}

// This struct is temporary, until we switch to the new event system
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEvent {
    pub timestamp: String,
    pub severity: EventSeverity,
    pub content: String,
}

// Event is serialized as #{ "id": "..." [, "data": ...] }#
// Pound signs are used to identify start and finish of json
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "id", content = "data")]
pub enum EventType {
    Session(Box<SessionDesc>),
    SessionUpdated, // deprecated
    ClientFoundOk,
    ClientFoundInvalid,
    ClientFoundWrongVersion(String),
    ClientConnected,
    ClientDisconnected,
    UpdateDownloadedBytesCount(usize),
    UpdateDownloadError,
    Statistics(Statistics),
    GraphStatistics(GraphStatistics),
    ServerQuitting,
    Log(LogEvent),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub timestamp: String,
    pub event_type: EventType,
}

pub fn send_event(event_type: EventType) {
    info!("#{}#", serde_json::to_string(&event_type).unwrap());
}
