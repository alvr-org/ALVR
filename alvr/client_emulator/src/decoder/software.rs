//! Portable CPU decoder built on ffmpeg.
//!
//! Fast enough by a wide margin for this purpose: measured at 38-45x realtime for a 1920x1792 72 fps
//! H.264 stream, so several emulated headsets cost a fraction of one core. The cost is one
//! CPU-to-GPU upload per frame, about 5 MB of planar YUV, which the renderer converts to RGB in a
//! shader rather than on the CPU.

use super::{ColorRange, DecodedFrame, VideoDecoder};
use alvr_common::{
    anyhow::{Result, anyhow},
    info, warn,
};
use alvr_session::CodecType;
use ffmpeg_next as ffmpeg;
use std::time::Duration;

/// Timestamps are carried through ffmpeg as microseconds.
///
/// ALVR identifies frames by an arbitrary `Duration`, and it has to come back out of the decoder
/// unchanged so the renderer can pair a frame with the pose it was rendered for. Microseconds are
/// small enough to be exact in an `i64` and coarse enough never to overflow.
#[cfg_attr(not(test), allow(dead_code))]
const TIMESTAMP_SCALE: u32 = 1_000_000;

pub struct SoftwareDecoder {
    decoder: ffmpeg::decoder::Video,
    /// Prepended to the first frame. H.264 and HEVC accept in-band parameter sets, so the
    /// configuration ALVR sends separately is simply spliced in front of the first NAL rather than
    /// poked into the codec's extradata.
    pending_config: Option<Vec<u8>>,
    /// Logged for the first few frames only. Timestamp scaling and parameter-set handling both fail
    /// silently when wrong, so a little evidence at startup is worth the noise.
    frames_logged: u32,
    /// Set once an unsupported pixel format has been reported, to avoid flooding the log.
    reported_bad_format: bool,
    /// Holds a frame drained to make room for new input, until the renderer collects it.
    pending_frame: Option<DecodedFrame>,
}

impl SoftwareDecoder {
    pub fn new(codec: CodecType, config_nal: &[u8]) -> Result<Self> {
        // Idempotent, and cheap enough to call per decoder.
        ffmpeg::init().map_err(|e| anyhow!("Cannot initialise ffmpeg: {e}"))?;

        let codec_id = match codec {
            CodecType::H264 => ffmpeg::codec::Id::H264,
            CodecType::Hevc => ffmpeg::codec::Id::HEVC,
            CodecType::AV1 => ffmpeg::codec::Id::AV1,
        };

        let ffmpeg_codec = ffmpeg::decoder::find(codec_id)
            .ok_or_else(|| anyhow!("No {codec:?} decoder available in this ffmpeg build"))?;

        let context = ffmpeg::codec::Context::new_with_codec(ffmpeg_codec);
        let decoder = context
            .decoder()
            .video()
            .map_err(|e| anyhow!("Cannot open {codec:?} decoder: {e}"))?;

        info!(
            "Software {codec:?} decoder ready ({} bytes of codec config)",
            config_nal.len()
        );

        Ok(Self {
            decoder,
            pending_config: (!config_nal.is_empty()).then(|| config_nal.to_vec()),
            frames_logged: 0,
            reported_bad_format: false,
            pending_frame: None,
        })
    }

    fn decode_packet(&mut self, timestamp: Duration, nal: &[u8]) -> Result<()> {
        // The parameter sets have to precede the first frame, or the decoder rejects it for having
        // no sequence header.
        let payload = match self.pending_config.take() {
            Some(config) => {
                let mut combined = config;
                combined.extend_from_slice(nal);
                combined
            }
            None => nal.to_vec(),
        };

        let mut packet = ffmpeg::codec::packet::Packet::copy(&payload);
        packet.set_pts(Some(duration_to_pts(timestamp)));
        packet.set_dts(Some(duration_to_pts(timestamp)));

        match self.decoder.send_packet(&packet) {
            Ok(()) => Ok(()),
            // EAGAIN means the decoder is holding finished frames and will not take more input
            // until they are collected. Frames are normally drained by the render thread, but it
            // runs at display rate while packets arrive at stream rate, so it can fall behind.
            // Draining here and retrying keeps the pipeline moving; without it the decoder wedges
            // and every subsequent packet is dropped.
            Err(ffmpeg::Error::Other {
                errno: ffmpeg::error::EAGAIN,
            }) => {
                self.drain_into_pending();

                self.decoder
                    .send_packet(&packet)
                    .map_err(|e| anyhow!("Decoder rejected packet after draining: {e}"))
            }
            Err(e) => Err(anyhow!("Decoder rejected packet: {e}")),
        }
    }

    /// Moves everything the decoder has finished into [`Self::pending_frames`].
    ///
    /// Only the newest is kept: showing the most recent frame matters more here than showing every
    /// frame, and holding a backlog would only add latency.
    fn drain_into_pending(&mut self) {
        let mut frame = ffmpeg::frame::Video::empty();

        while self.decoder.receive_frame(&mut frame).is_ok() {
            if let Some(decoded) = self.convert(&frame) {
                self.pending_frame = Some(decoded);
            }
        }
    }

    /// Copies an ffmpeg frame into the representation the renderer consumes.
    fn convert(&mut self, frame: &ffmpeg::frame::Video) -> Option<DecodedFrame> {
        let timestamp = frame
            .pts()
            .or_else(|| frame.timestamp())
            .map(pts_to_duration)
            .unwrap_or_default();

        // Both layouts are planar 8-bit YUV 4:2:0 and differ only in sample range. YUVJ is
        // deprecated in ffmpeg but still what its H.264 decoder reports for full-range streams,
        // which is what ALVR's encoders produce.
        let range = match frame.format() {
            ffmpeg::format::Pixel::YUV420P => Some(ColorRange::Limited),
            ffmpeg::format::Pixel::YUVJ420P => Some(ColorRange::Full),
            _ => None,
        };

        match range {
            Some(range) => Some(DecodedFrame::Yuv420 {
                timestamp,
                width: frame.width(),
                height: frame.height(),
                range,
                // ffmpeg pads rows, so the stride is kept and the renderer uploads accordingly
                // rather than assuming stride == width.
                y: frame.data(0).to_vec(),
                y_stride: frame.stride(0) as u32,
                u: frame.data(1).to_vec(),
                v: frame.data(2).to_vec(),
                uv_stride: frame.stride(1) as u32,
            }),
            None => {
                // Nothing converts other layouts yet. Reported once rather than per frame, since it
                // would otherwise flood at the stream frame rate.
                if !self.reported_bad_format {
                    warn!(
                        "Unsupported decoded pixel format {:?}; frames dropped",
                        frame.format()
                    );
                    self.reported_bad_format = true;
                }

                None
            }
        }
    }
}

impl VideoDecoder for SoftwareDecoder {
    fn push_nal(&mut self, timestamp: Duration, nal: &[u8]) -> bool {
        if self.frames_logged < 3 {
            info!(
                "Video frame {}: timestamp {:?}, {} bytes",
                self.frames_logged,
                timestamp,
                nal.len()
            );
            self.frames_logged += 1;
        }

        match self.decode_packet(timestamp, nal) {
            Ok(()) => true,
            Err(e) => {
                warn!("{e}");
                false
            }
        }
    }

    fn poll_frame(&mut self) -> Option<DecodedFrame> {
        // Anything stashed while making room for new input is returned first.
        if let Some(frame) = self.pending_frame.take() {
            return Some(frame);
        }

        let mut frame = ffmpeg::frame::Video::empty();

        // An error here is usually EAGAIN, meaning no frame is ready yet.
        if self.decoder.receive_frame(&mut frame).is_err() {
            return None;
        }

        self.convert(&frame)
    }
}

fn duration_to_pts(timestamp: Duration) -> i64 {
    timestamp.as_micros() as i64
}

fn pts_to_duration(pts: i64) -> Duration {
    Duration::from_micros(pts.max(0) as u64)
}

/// Guards the assumption that microseconds round-trip exactly.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_round_trip() {
        for micros in [0u64, 1, 999, 1_000_000, 16_666, 72_000_000_000] {
            let original = Duration::from_micros(micros);
            assert_eq!(pts_to_duration(duration_to_pts(original)), original);
        }
    }

    #[test]
    fn scale_matches_conversion() {
        // A frame at exactly one second must land on the scale, catching a stray factor of 1000.
        assert_eq!(duration_to_pts(Duration::from_secs(1)), TIMESTAMP_SCALE as i64);
    }
}
