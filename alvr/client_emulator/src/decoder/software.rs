//! Portable CPU decoder built on ffmpeg.
//!
//! Fast enough by a wide margin for this purpose: measured at 38-45x realtime for a 1920x1792 72 fps
//! H.264 stream, so several emulated headsets cost a fraction of one core. The cost is one
//! CPU-to-GPU upload per frame, about 5 MB of planar YUV, which the renderer converts to RGB in a
//! shader rather than on the CPU.

use super::{ColorRange, DecodedFrame, FrameDecodedCallback, VideoDecoder};
use alvr_common::{
    anyhow::{Result, anyhow},
    info, warn,
};
use alvr_session::CodecType;
use ffmpeg_next as ffmpeg;
use std::{collections::VecDeque, time::Duration};

/// Timestamps are carried through ffmpeg as microseconds.
///
/// ALVR identifies frames by an arbitrary `Duration`, and it has to come back out of the decoder
/// unchanged so the renderer can pair a frame with the pose it was rendered for. Microseconds are
/// small enough to be exact in an `i64` and coarse enough never to overflow.
#[cfg_attr(not(test), allow(dead_code))]
const TIMESTAMP_SCALE: u32 = 1_000_000;

/// Upper bound on outstanding submitted timestamps.
///
/// A decoder holds only a few frames, so anything beyond this means frames were dropped internally
/// and the oldest entries are stale.
const MAX_TRACKED_TIMESTAMPS: usize = 16;

pub struct SoftwareDecoder {
    decoder: ffmpeg::decoder::Video,
    /// Codec parameter sets, prepended to every keyframe. H.264 and HEVC accept in-band parameter
    /// sets, so the configuration ALVR sends separately is spliced in front of the NAL rather than
    /// poked into the codec's extradata.
    config_nal: Vec<u8>,
    /// Which codec is being decoded, needed to recognise a keyframe.
    codec: CodecType,
    /// Logged for the first few frames only. Timestamp scaling and parameter-set handling both fail
    /// silently when wrong, so a little evidence at startup is worth the noise.
    frames_logged: u32,
    /// Set once an unsupported pixel format has been reported, to avoid flooding the log.
    reported_bad_format: bool,
    /// Holds a frame drained to make room for new input, until the renderer collects it.
    pending_frame: Option<DecodedFrame>,
    /// True while a keyframe is needed to repair the picture.
    ///
    /// Inter-frames are differences against earlier pictures, so one decoded without its reference
    /// is wrong, and every later frame predicted from it inherits the damage. Frames are still
    /// decoded and displayed while this is set — a glitchy image beats a frozen one — but reporting
    /// it is what makes ALVR ask the server for a keyframe, and the decoder is reset when that
    /// keyframe arrives so no damaged reference survives it.
    awaiting_keyframe: bool,
    /// Timestamps of packets submitted but not yet returned as frames, oldest first.
    submitted_timestamps: VecDeque<Duration>,
    /// Reported as each frame finishes decoding. The statistics depend on this firing per decoded
    /// frame, at decode time, rather than once per frame displayed.
    on_frame_decoded: FrameDecodedCallback,
}

impl SoftwareDecoder {
    pub fn new(
        codec: CodecType,
        config_nal: &[u8],
        on_frame_decoded: FrameDecodedCallback,
    ) -> Result<Self> {
        // Idempotent, and cheap enough to call per decoder.
        ffmpeg::init().map_err(|e| anyhow!("Cannot initialise ffmpeg: {e}"))?;

        let decoder = Self::open_codec(codec)?;

        info!(
            "Software {codec:?} decoder ready ({} bytes of codec config)",
            config_nal.len()
        );

        Ok(Self {
            decoder,
            config_nal: config_nal.to_vec(),
            codec,
            frames_logged: 0,
            reported_bad_format: false,
            pending_frame: None,
            awaiting_keyframe: true,
            submitted_timestamps: VecDeque::new(),
            on_frame_decoded,
        })
    }

    /// Builds a fresh ffmpeg decoder for a codec.
    fn open_codec(codec: CodecType) -> Result<ffmpeg::decoder::Video> {
        let codec_id = match codec {
            CodecType::H264 => ffmpeg::codec::Id::H264,
            CodecType::Hevc => ffmpeg::codec::Id::HEVC,
            CodecType::AV1 => ffmpeg::codec::Id::AV1,
        };

        let ffmpeg_codec = ffmpeg::decoder::find(codec_id)
            .ok_or_else(|| anyhow!("No {codec:?} decoder available in this ffmpeg build"))?;

        ffmpeg::codec::Context::new_with_codec(ffmpeg_codec)
            .decoder()
            .video()
            .map_err(|e| anyhow!("Cannot open {codec:?} decoder: {e}"))
    }

    /// Throws away all decoder state, including every reference picture it is holding.
    ///
    /// Flushing alone is not always enough after concealed errors, so the codec is reopened. Any
    /// frames still queued inside are lost, which is fine: they are the damaged ones.
    fn reset_codec(&mut self) -> Result<()> {
        self.decoder = Self::open_codec(self.codec)?;
        self.pending_frame = None;
        self.submitted_timestamps.clear();

        Ok(())
    }

    /// Whether this access unit contains a keyframe, and can therefore be decoded on its own.
    ///
    /// ALVR delivers Annex-B, so NAL units are separated by `00 00 01` start codes and the type is
    /// in the byte that follows. Only the coded-slice types matter here; parameter sets and other
    /// non-VCL units are skipped over.
    fn contains_keyframe(codec: CodecType, nal: &[u8]) -> bool {
        nal.windows(3)
            .enumerate()
            .filter(|(_, window)| *window == [0, 0, 1])
            .filter_map(|(index, _)| nal.get(index + 3))
            .any(|&header| match codec {
                // H.264: type is the low 5 bits; 5 is an IDR slice.
                CodecType::H264 => header & 0x1f == 5,
                // HEVC: type is bits 1-6; 16-23 are the IRAP range, which includes IDR and CRA.
                CodecType::Hevc => matches!((header >> 1) & 0x3f, 16..=23),
                // AV1 is not Annex-B framed, so this parse does not apply. Treated as always
                // decodable rather than discarding everything.
                CodecType::AV1 => true,
            })
    }

    fn decode_packet(&mut self, timestamp: Duration, nal: &[u8], is_keyframe: bool) -> Result<()> {
        // The parameter sets are prepended to every keyframe, not just the first.
        //
        // ALVR sends them out of band and repeats them each time it is asked for a recovery
        // keyframe, precisely so the decoder can be re-primed. Consuming them once would leave every
        // later keyframe without a sequence header: it then fails to decode, the picture never
        // repairs itself, and the corruption looks permanent.
        //
        // Repeating them on a keyframe that already carries its own is harmless, because a decoder
        // is required to accept a redundant parameter set.
        let payload = if is_keyframe && !self.config_nal.is_empty() {
            let mut combined = self.config_nal.clone();
            combined.extend_from_slice(nal);
            combined
        } else {
            nal.to_vec()
        };

        // The timestamp is remembered here rather than recovered from the decoded frame.
        //
        // ALVR identifies a frame by the tracking timestamp it was rendered from, and its statistics
        // are keyed by exactly that value: report a different one and the lookup silently finds
        // nothing, no statistics reach the server, and a server pacing itself on them stops sending
        // video. Round-tripping through ffmpeg does not preserve it — the value that came back was
        // microseconds where the input was seconds — so the queue below keeps the original.
        self.submitted_timestamps.push_back(timestamp);
        while self.submitted_timestamps.len() > MAX_TRACKED_TIMESTAMPS {
            self.submitted_timestamps.pop_front();
        }

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
        // Concealed frames are still displayed. They look wrong, but a glitchy picture is more
        // usable than a frozen one, and flagging here is what asks the server for the keyframe that
        // clears the artefacts.
        if frame.is_corrupt() || frame.has_decode_errors() {
            if !self.awaiting_keyframe {
                warn!("Concealed frame from decoder, requesting a keyframe");
                self.awaiting_keyframe = true;
            }
        } else if self.awaiting_keyframe && frame.is_key() {
            // A clean keyframe is the point at which the picture is trustworthy again.
            self.awaiting_keyframe = false;
        }

        // The timestamp must come back out exactly as ALVR passed it in, to the nanosecond. The PTS
        // identifies which submitted frame this is; the value handed on is the original, because a
        // PTS only carries microseconds. See [`take_submitted_timestamp`]. A frame that arrives
        // without a usable PTS falls back to submission order.
        let timestamp = frame
            .pts()
            .and_then(|pts| take_submitted_timestamp(&mut self.submitted_timestamps, pts))
            .or_else(|| self.submitted_timestamps.pop_front())
            .unwrap_or_default();

        // Both layouts are planar 8-bit YUV 4:2:0 and differ only in sample range. YUVJ is
        // deprecated in ffmpeg but still what its H.264 decoder reports for full-range streams,
        // which is what ALVR's encoders produce.
        let range = match frame.format() {
            ffmpeg::format::Pixel::YUV420P => Some(ColorRange::Limited),
            ffmpeg::format::Pixel::YUVJ420P => Some(ColorRange::Full),
            _ => None,
        };

        // Reported here, as the frame finishes decoding, matching where the real client reports it
        // from its decoder callback. The statistics measure decode time from packet arrival to this
        // point, so reporting later — once per displayed frame — leaves them incomplete.
        if range.is_some() {
            (self.on_frame_decoded)(timestamp);
        }

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
        let is_keyframe = Self::contains_keyframe(self.codec, nal);

        if self.frames_logged < 3 {
            info!(
                "Video frame {}: timestamp {:?}, {} bytes, keyframe {is_keyframe}",
                self.frames_logged,
                timestamp,
                nal.len()
            );
            self.frames_logged += 1;
        }

        // Frames are decoded even while a keyframe is outstanding. They may be predicted from a
        // picture that was never received and so look wrong, but a briefly glitchy image is easier
        // to work with than a frozen one, and the artefacts clear as soon as the keyframe lands.
        //
        // The decoder is reset on the keyframe itself, below, so nothing damaged survives it.
        let was_awaiting = self.awaiting_keyframe;

        if is_keyframe && was_awaiting {
            // Starting the codec from scratch discards every reference picture the concealed frames
            // left behind. Without this the keyframe decodes correctly but later frames can still
            // predict from stale, damaged references, which is what makes corruption look permanent.
            if let Err(e) = self.reset_codec() {
                warn!("Could not reset decoder before keyframe: {e}");
            }
        }

        match self.decode_packet(timestamp, nal, is_keyframe) {
            Ok(()) => {
                // Collected here rather than left for the render thread so that a concealed frame
                // is noticed while this call can still report it. `convert` inspects each frame and
                // re-arms the keyframe wait if the decoder had to conceal errors.
                self.drain_into_pending();

                if was_awaiting && !self.awaiting_keyframe {
                    info!("Keyframe decoded, video recovered");
                }

                // Reporting failure is what makes ALVR request the keyframe needed to recover.
                !self.awaiting_keyframe
            }
            Err(e) => {
                warn!("{e}");

                // Whatever the decoder had is now unreliable, so wait for a keyframe before showing
                // anything again. Returning false makes ALVR request one.
                self.awaiting_keyframe = true;
                false
            }
        }
    }

    fn set_config_nal(&mut self, config_nal: &[u8]) {
        if !config_nal.is_empty() {
            self.config_nal = config_nal.to_vec();
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

/// Recovers the exact `Duration` that produced `pts`, and drops everything ahead of it.
///
/// A PTS carries microseconds, and ALVR's frame timestamps do not fit in microseconds: they are
/// `Instant` elapsed times, which on Windows come from the performance counter at 100 ns
/// resolution. Converting the PTS back therefore lands within a microsecond of the original rather
/// than on it, and every lookup keyed by the timestamp then misses — the statistics chain finds
/// nothing, and `report_compositor_start` hands back the *previous* frame's view parameters
/// instead of this frame's, so anything drawn from them lags the video by however long the last
/// exact match was ago. Measured before this: 91% of displayed frames came back with a stale pose.
///
/// Entries in front of the match are dropped rather than kept. The decoder emits frames in
/// submission order, so anything still queued ahead of the match went in and never came out.
fn take_submitted_timestamp(queue: &mut VecDeque<Duration>, pts: i64) -> Option<Duration> {
    let index = queue
        .iter()
        .position(|timestamp| duration_to_pts(*timestamp) == pts)?;

    queue.drain(..index);
    queue.pop_front()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_full_precision_timestamps() {
        // Sub-microsecond digits, as a real frame timestamp has. Truncating the PTS and converting
        // it back would lose them, and every lookup keyed by the timestamp would miss.
        let submitted = [
            Duration::new(3, 16_666_400),
            Duration::new(3, 30_555_100),
            Duration::new(3, 44_443_900),
        ];
        let mut queue = VecDeque::from(submitted.to_vec());

        assert_eq!(
            take_submitted_timestamp(&mut queue, duration_to_pts(submitted[1])),
            Some(submitted[1])
        );
        // The skipped frame is gone, so the next match cannot resolve backwards onto it.
        assert_eq!(queue.len(), 1);
        assert_eq!(
            take_submitted_timestamp(&mut queue, duration_to_pts(submitted[2])),
            Some(submitted[2])
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn unknown_pts_leaves_the_queue_alone() {
        let mut queue = VecDeque::from(vec![Duration::new(1, 500)]);

        assert_eq!(take_submitted_timestamp(&mut queue, 999_999_999), None);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn detects_h264_keyframes() {
        // nal_unit_type 5 is an IDR slice, 1 a non-IDR slice.
        let idr = [0, 0, 1, 0x65, 0x88];
        let non_idr = [0, 0, 1, 0x41, 0x9a];

        assert!(SoftwareDecoder::contains_keyframe(CodecType::H264, &idr));
        assert!(!SoftwareDecoder::contains_keyframe(
            CodecType::H264,
            &non_idr
        ));

        // A keyframe preceded by its parameter sets, which is how ALVR delivers one: SPS (7) and
        // PPS (8) are not keyframes themselves, so the scan has to continue past them.
        let with_parameter_sets = [
            0, 0, 1, 0x67, 0x42, // SPS
            0, 0, 1, 0x68, 0xce, // PPS
            0, 0, 1, 0x65, 0x88, // IDR
        ];
        assert!(SoftwareDecoder::contains_keyframe(
            CodecType::H264,
            &with_parameter_sets
        ));

        // Parameter sets alone must not count, or the stream would be declared recovered before any
        // picture had actually been decoded.
        let parameter_sets_only = [0, 0, 1, 0x67, 0x42, 0, 0, 1, 0x68, 0xce];
        assert!(!SoftwareDecoder::contains_keyframe(
            CodecType::H264,
            &parameter_sets_only
        ));
    }

    #[test]
    fn detects_hevc_keyframes() {
        // HEVC puts the type in bits 1-6; 19 (IDR_W_RADL) is in the IRAP range, 1 (TRAIL_R) is not.
        let idr = [0, 0, 1, 19 << 1, 0x01];
        let trail = [0, 0, 1, 1 << 1, 0x01];

        assert!(SoftwareDecoder::contains_keyframe(CodecType::Hevc, &idr));
        assert!(!SoftwareDecoder::contains_keyframe(CodecType::Hevc, &trail));
    }

    #[test]
    fn scale_matches_conversion() {
        // A frame at exactly one second must land on the scale, catching a stray factor of 1000.
        assert_eq!(duration_to_pts(Duration::from_secs(1)), TIMESTAMP_SCALE as i64);
    }
}
