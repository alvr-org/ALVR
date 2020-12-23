use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HandshakePacket {
    pub alvr_name: String,
    pub version: Version,
    pub device_name: String,
    pub hostname: String,
    pub certificate_pem: String,

    // reserved field is used to add features between major releases: the schema of the packet
    // should never change anymore (required only for this packet).
    pub reserved: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HeadsetInfoPacket {
    pub recommended_eye_width: u32,
    pub recommended_eye_height: u32,
    pub available_refresh_rates: Vec<f32>,
    pub preferred_refresh_rate: f32,

    // reserved field is used to add features in a minor release that otherwise would break the
    // packets schema
    pub reserved: String,
}

#[derive(Serialize, Deserialize)]
pub struct ClientConfigPacket {
    pub session_desc: String, // transfer session as string to allow for extrapolation
    pub eye_resolution_width: u32,
    pub eye_resolution_height: u32,
    pub fps: f32,
    pub web_gui_url: String,
    pub reserved: String,
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    Restarting,
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    RequestIDR,
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}
