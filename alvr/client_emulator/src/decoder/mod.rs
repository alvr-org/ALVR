//! Video decoding.
//!
//! [`VideoDecoder`] is the seam between the ALVR stream and the renderer. Only a portable software
//! implementation exists today; the trait is what lets a zero-copy platform implementation be added
//! later without the call sites or the renderer knowing which one is in use.
//!
//! Frames arrive as [`DecodedFrame`], which is deliberately an enum rather than a plain buffer: a
//! GPU implementation would return an already-resident texture, and the renderer decides how to
//! consume each shape.

mod software;

use alvr_common::anyhow::Result;
use alvr_session::CodecType;
use std::time::Duration;

/// How the YUV samples are scaled.
///
/// Getting this wrong is not a crash but a visibly washed out or crushed image, so it is carried
/// with the frame rather than assumed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorRange {
    /// Luma 16-235, chroma 16-240. The usual broadcast convention.
    Limited,
    /// Luma and chroma both use the full 0-255.
    Full,
}

/// A decoded video frame, in whatever form its decoder produced.
pub enum DecodedFrame {
    /// Planar YUV in system memory. Converted to RGB by the renderer's shader.
    Yuv420 {
        timestamp: Duration,
        width: u32,
        height: u32,
        range: ColorRange,
        /// Luma plane, `y_stride` bytes per row.
        y: Vec<u8>,
        y_stride: u32,
        /// Chroma planes at half resolution in both axes.
        u: Vec<u8>,
        v: Vec<u8>,
        uv_stride: u32,
    },
    // A future GPU implementation adds a variant carrying a texture here, so the frame never
    // reaches system memory.
}

impl DecodedFrame {
    pub fn timestamp(&self) -> Duration {
        match self {
            DecodedFrame::Yuv420 { timestamp, .. } => *timestamp,
        }
    }
}

/// Decodes an ALVR video stream.
///
/// Implementations are expected to be usable from a single thread; the emulator drives one decoder
/// from the connection callback and drains it on the render thread.
pub trait VideoDecoder: Send {
    /// Submits one NAL unit.
    ///
    /// Returns `false` if the frame could not be accepted, which tells the client core to drop it
    /// rather than assume it was queued.
    fn push_nal(&mut self, timestamp: Duration, nal: &[u8]) -> bool;

    /// Takes the next decoded frame, if one is ready.
    fn poll_frame(&mut self) -> Option<DecodedFrame>;
}

/// Which decoder implementation to use.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DecoderKind {
    /// Portable, decodes on the CPU and uploads frames to the GPU.
    Software,
}

impl DecoderKind {
    /// The best implementation available for this platform.
    ///
    /// Only one exists today. When a zero-copy implementation is added it is selected here, gated on
    /// the platform and on the graphics backend actually in use — a DirectX video decoder is only
    /// useful to a DirectX renderer.
    pub fn preferred() -> Self {
        DecoderKind::Software
    }
}

/// Creates a decoder for a stream.
///
/// `config_nal` is the codec configuration (SPS/PPS or equivalent) that ALVR delivers separately
/// from the frame data, ahead of the first frame.
pub fn create(
    kind: DecoderKind,
    codec: CodecType,
    config_nal: &[u8],
) -> Result<Box<dyn VideoDecoder>> {
    match kind {
        DecoderKind::Software => Ok(Box::new(software::SoftwareDecoder::new(
            codec, config_nal,
        )?)),
    }
}
