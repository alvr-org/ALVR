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
    pub total_packets: u64,
    pub packet_rate: u64,
    pub packets_lost_total: u64,
    pub packets_lost_per_second: u64,
    pub total_sent: u64,
    pub sent_rate: f32,
    pub total_latency: f32,
    pub encode_latency: f32,
    pub encode_latency_max: f32,
    pub transport_latency: f32,
    pub decode_latency: f32,
    pub fec_percentage: u32,
    pub fec_failure_total: u64,
    pub fec_failure_in_second: u64,
    pub client_f_p_s: u32, // the name will be fixed after the old dashboard is removed
    pub server_f_p_s: u32,
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
