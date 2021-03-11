use nalgebra::{Point2, Point3, UnitQuaternion};
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientHandshakePacket {
    pub alvr_name: String,
    pub version: Version,
    pub device_name: String,
    pub hostname: String,

    // reserved field is used to add features between major releases: the schema of the packet
    // should never change anymore (required only for this packet).
    pub reserved1: String,
    pub reserved2: String,
}

// Since this packet is not essential, any change to it will not be a braking change
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerHandshakePacket {
    ClientUntrusted,
    IncompatibleVersions,
}

#[derive(Serialize, Deserialize)]
pub enum HandshakePacket {
    Client(ClientHandshakePacket),
    Server(ServerHandshakePacket),
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
    pub dashboard_url: String,
    pub eye_resolution_width: u32,
    pub eye_resolution_height: u32,
    pub fps: f32,
    pub game_audio_sample_rate: u32,
    pub reserved: String,
}

#[derive(Serialize, Deserialize)]
pub enum ServerControlPacket {
    StartStream,
    Restarting,
    KeepAlive,
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}

#[derive(Serialize, Deserialize)]
pub struct PlayspaceSyncPacket {
    pub position: Point3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub area_width: f32,
    pub area_height: f32,
    pub perimeter_points: Option<Vec<Point2<f32>>>,
}

#[derive(Serialize, Deserialize)]
pub enum ClientControlPacket {
    PlayspaceSync(PlayspaceSyncPacket),
    RequestIDR,
    KeepAlive,
    Reserved(String),
    ReservedBuffer(Vec<u8>),
}
