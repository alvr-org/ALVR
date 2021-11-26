use std::time::Duration;

use crate::ViewConfig;

pub struct VideoSlicePacket {
    timestamp: Duration,
    buffer: Vec<u8>,
}

pub struct FrameMetadataPacket {
    timestamp: Duration,
    views: Vec<ViewConfig>,
}
