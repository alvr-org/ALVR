use alvr_common::{glam::UVec2, prelude::*};
use alvr_graphics::GraphicsContext;
use alvr_session::{CodecType, MediacodecDataType};
use std::{sync::Arc, time::Duration};
use wgpu::Texture;

pub struct VideoDecoderEnqueuer {}
impl VideoDecoderEnqueuer {
    // Block until the buffer has been written or timeout is reached. Returns false if timeout.
    pub fn push_frame_nals(
        &self,
        timestamp: Duration,
        data: &[u8],
        timeout: Duration,
    ) -> StrResult<bool> {
        Ok(false)
    }
}

pub struct VideoDecoderDequeuer {}
impl VideoDecoderDequeuer {
    pub fn poll(&self, timeout: Duration) -> StrResult {
        Ok(())
    }
}

pub struct VideoDecoderFrameGrabber {}
impl VideoDecoderFrameGrabber {
    // Block until one frame is available or timeout is reached. Returns the frame timestamp (as
    // specified in push_frame_nals())
    pub fn get_output_frame(&self, timeout: Duration) -> StrResult<Duration> {
        Ok(Duration::ZERO)
    }
}

pub fn split(
    graphics_context: Arc<GraphicsContext>,
    codec_type: CodecType,
    csd_0: &[u8],
    extra_options: &[(String, MediacodecDataType)],
    output_texture: Arc<Texture>,
    output_size: UVec2,
    slice_index: u32,
) -> StrResult<(
    VideoDecoderEnqueuer,
    VideoDecoderDequeuer,
    VideoDecoderFrameGrabber,
)> {
    todo!()
}
