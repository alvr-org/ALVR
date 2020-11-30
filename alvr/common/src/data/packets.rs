use super::settings::*;
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HandshakePacket {
    pub alvr_name: String,
    pub version: Version,
    pub device_name: String,
    pub hostname: String,
    pub certificate_pem: String,

    // reserved field is used to add features between major releases: the packets schema should
    // never change anymore.
    pub reserved: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HeadsetInfoPacket {
    pub recommended_eye_width: u32,
    pub recommended_eye_height: u32,
    pub recommended_left_eye_fov: Fov,
    pub available_refresh_rates: Vec<f32>,

    // reserved field is used to add features in a minor release that otherwise would break the
    // packets schema
    pub reserved: String,
}

#[derive(Serialize, Deserialize)]
pub struct ClientConfigPacket {
    pub settings: String,
    pub eye_resolution_width: u32,
    pub eye_resolution_height: u32,
    pub left_eye_fov: Fov,
    pub fps: u32,
    pub web_gui_url: String,
    pub reserved: String,
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    Restarting,
    Shutdown,
    Reserved(String),
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    Disconnect,
    Reserved(String),
}
