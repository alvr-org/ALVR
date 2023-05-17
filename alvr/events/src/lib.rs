use alvr_common::{prelude::*, DeviceMotion, Pose};
use alvr_packets::{AudioDevicesList, ButtonValue};
use alvr_session::SessionDesc;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Statistics {
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
    pub battery_left: u32,
    pub battery_right: u32,
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackingEvent {
    pub head_motion: Option<DeviceMotion>,
    pub controller_motions: [Option<DeviceMotion>; 2],
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "id", content = "data")]
pub enum EventType {
    Log(LogEntry),
    Session(Box<SessionDesc>),
    Statistics(Statistics),
    GraphStatistics(GraphStatistics),
    Tracking(Box<TrackingEvent>),
    Buttons(Vec<ButtonEvent>),
    Haptics(HapticsEvent),
    AudioDevices(AudioDevicesList),
    DriversList(Vec<PathBuf>),
    ServerRequestsSelfRestart,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub timestamp: String,
    pub event_type: EventType,
}

pub fn send_event(event_type: EventType) {
    info!("{}", serde_json::to_string(&event_type).unwrap());
}
