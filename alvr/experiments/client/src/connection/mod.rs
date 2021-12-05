use crate::ViewConfig;
use std::time::Duration;

pub struct VideoSlicePacket {
    pub timestamp: Duration,
    pub buffer: Vec<u8>,
}

pub struct VideoFrameMetadataPacket {
    pub timestamp: Duration,
    pub view_configs: Vec<ViewConfig>,
}
