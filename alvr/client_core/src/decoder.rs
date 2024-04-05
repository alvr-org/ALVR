use alvr_common::anyhow::{bail, Result};
use alvr_session::{CodecType, MediacodecDataType};
use std::time::Duration;

#[derive(Clone, Default)]
pub struct DecoderConfig {
    pub codec: CodecType,
    pub force_software_decoder: bool,
    pub max_buffering_frames: f32,
    pub buffering_history_weight: f32,
    pub options: Vec<(String, MediacodecDataType)>,
    pub config_buffer: Vec<u8>,
}

pub struct DecoderSink {
    #[cfg(target_os = "android")]
    inner: crate::platform::VideoDecoderSink,
}

impl DecoderSink {
    // returns true if frame has been successfully enqueued
    #[allow(unused_variables)]
    pub fn push_nal(&mut self, timestamp: Duration, nal: &[u8]) -> bool {
        #[cfg(target_os = "android")]
        {
            alvr_common::show_err(self.inner.push_frame_nal(timestamp, nal)).unwrap_or(false)
        }
        #[cfg(not(target_os = "android"))]
        false
    }
}

pub struct DecoderSource {
    #[cfg(target_os = "android")]
    inner: crate::platform::VideoDecoderSource,
}

impl DecoderSource {
    /// If a frame is available, return the timestamp and the AHardwareBuffer.
    pub fn get_frame(&mut self) -> Result<Option<(Duration, *mut std::ffi::c_void)>> {
        #[cfg(target_os = "android")]
        {
            self.inner.dequeue_frame()
        }
        #[cfg(not(target_os = "android"))]
        bail!("Not implemented");
    }
}

// report_frame_decoded: (target_timestamp: Duration) -> ()
#[allow(unused_variables)]
pub fn create_decoder(
    config: DecoderConfig,
    report_frame_decoded: impl Fn(Duration) + Send + 'static,
) -> (DecoderSink, DecoderSource) {
    #[cfg(target_os = "android")]
    {
        let (sink, source) = crate::platform::video_decoder_split(
            config.clone(),
            config.config_buffer,
            report_frame_decoded,
        )
        .unwrap();

        (DecoderSink { inner: sink }, DecoderSource { inner: source })
    }
    #[cfg(not(target_os = "android"))]
    (DecoderSink {}, DecoderSource {})
}
