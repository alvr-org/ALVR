use alvr_common::Fov;
use openxr::Posef;
use std::time::Duration;

pub struct VideoPacket {
    timestamp: Duration,
    buffer: Vec<u8>,
    pose: Posef,
    fov: Fov,
}
