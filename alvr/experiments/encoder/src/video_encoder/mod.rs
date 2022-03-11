mod vulkan;
mod x264;

use self::x264::X264Encoder;
use alvr_common::glam::UVec2;
use alvr_graphics::{ash::vk, wgpu::Texture, GraphicsContext};
use alvr_session::{AccelerationType, CodecType, VideoEncoderConfig};
use std::sync::Arc;
use vulkan::VulkanEncoder;

enum EncoderInner {
    Vulkan(VulkanEncoder),
    X264(X264Encoder),
}

pub struct VideoEncoder {
    inner: EncoderInner,
    semaphore: vk::Semaphore,
}

// Note: in case more slices than video queues are requested, distribute slices equally across
// available video queues. In case no video queue is available, fall back to software encoding.
impl VideoEncoder {
    pub fn new(
        graphics_context: Arc<GraphicsContext>,
        codec_type: CodecType,
        acceleration_type: AccelerationType,
        config: VideoEncoderConfig,
        slice_size: UVec2,
        // for vulkan this is for the video queue, for x264 this is the transfer queue
        queue_index: usize,
    ) {
        // let device = &graphics_context
    }

    // Timeline semaphores. Odd values are set by the compositor, even values by the encoder.
    pub fn semaphore(&self) -> vk::Semaphore {
        self.semaphore
    }

    pub fn encode(&mut self, texture: Texture) -> &[u8] {
        match &mut self.inner {
            EncoderInner::Vulkan(encoder) => unsafe { encoder.encode(texture, self.semaphore) },
            EncoderInner::X264(encoder) => encoder.encode(&texture, self.semaphore),
        }
    }
}
