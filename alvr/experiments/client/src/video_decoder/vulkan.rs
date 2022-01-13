use alvr_common::{glam::UVec2, prelude::*};
use alvr_graphics::GraphicsContext;
use alvr_session::{CodecType, MediacodecDataType};
use std::{sync::Arc, time::Duration};
use wgpu::Texture;

pub struct VideoDecoder {}

impl VideoDecoder {
    pub fn new(
        context: Arc<GraphicsContext>,
        codec_type: CodecType,
        video_size: UVec2,
        extra_options: &[(String, MediacodecDataType)],
    ) -> StrResult<Self> {
        todo!()
    }

    // Block until the buffer has been written or timeout is reached. Returns false if timeout.
    pub fn push_frame_nals(
        &self,
        frame_timestamp: Duration,
        data: &[u8],
        timeout: Duration,
    ) -> StrResult<bool> {
        todo!()
    }

    // Block until one frame is available or timeout is reached. Returns the frame timestamp (as
    // specified in push_frame_nals()). Returns None if timeout.
    pub fn get_output_frame(
        &self,
        output: &Texture,
        slice_index: u32,
        timeout: Duration,
    ) -> StrResult<Option<Duration>> {
        todo!()
    }
}
