use crate::FfiDynamicEncoderParams;
use alvr_common::SlidingWindowAverage;
use alvr_session::{BitrateDesc, CodecType};
use settings_schema::Switch;
use std::time::{Duration, Instant};

const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

pub struct BitrateManager {
    video_desc: BitrateDesc,
    codec_desc: CodecType,
    frame_interval_average: SlidingWindowAverage<Duration>,
    frame_size_bits_average: SlidingWindowAverage<usize>,
    network_latency_average: SlidingWindowAverage<Duration>,
    last_frame_instant: Instant,
    last_timestamp: Duration,
    last_update_instant: Instant,
}

impl BitrateManager {
    pub fn new(video_desc: BitrateDesc, codec_desc: CodecType, max_history_size: usize) -> Self {
        Self {
            video_desc,
            codec_desc,
            frame_interval_average: SlidingWindowAverage::new(
                Duration::from_millis(16),
                max_history_size,
            ),
            frame_size_bits_average: SlidingWindowAverage::new(80_000, max_history_size),
            network_latency_average: SlidingWindowAverage::new(
                Duration::from_millis(16),
                max_history_size,
            ),
            last_frame_instant: Instant::now(),
            last_timestamp: Duration::ZERO,
            last_update_instant: Instant::now(),
        }
    }

    // Note: this will be reported with the frequency of the server framerate
    pub fn report_encoded_frame(&mut self, size_bytes: usize) {
        let now = Instant::now();
        self.frame_interval_average
            .submit_sample(now - self.last_frame_instant);
        self.last_frame_instant = now;

        self.frame_size_bits_average.submit_sample(size_bytes * 8);
    }

    // Note: this will be reported with the frequency of the client framerate
    pub fn report_frame_network_latency(&mut self, timestamp: Duration, latency: Duration) {
        if timestamp != self.last_timestamp {
            self.network_latency_average.submit_sample(latency);
        }
    }

    pub fn get_encoder_params(&mut self) -> FfiDynamicEncoderParams {
        let now = Instant::now();
        if now > self.last_update_instant + UPDATE_INTERVAL {
            self.last_update_instant = now;
        } else {
            return FfiDynamicEncoderParams {
                updated: false,
                bitrate_bps: 0,
                framerate: 0.0,
            };
        }

        let limit_bitrate = match &self.codec_desc {
            CodecType::H264 => false,
            CodecType::HEVC => true,
        };
        let bitrate_bps = match &self.video_desc {
            BitrateDesc::ConstantMbs(bitrate_mbs) => *bitrate_mbs * 1_000_000,
            BitrateDesc::Adaptive {
                saturation_multiplier,
                max_bitrate_mbs,
                min_bitrate_mbs,
            } => {
                let bits_average = self.frame_size_bits_average.get_average();
                let latency_average = self.network_latency_average.get_average();

                let mut bitrate_bps = (bits_average as f32 / latency_average.as_secs_f32()
                    * saturation_multiplier) as u64;

                //Limit bitrate on HEVC because going above 100 causes issues with the decoder(XR2)
                if limit_bitrate {
                    bitrate_bps = u64::min(bitrate_bps, 100 * 1_000_000);
                }
                if let Switch::Enabled(max) = max_bitrate_mbs {
                    bitrate_bps = u64::min(bitrate_bps, max * 1_000_000);
                }
                if let Switch::Enabled(min) = min_bitrate_mbs {
                    bitrate_bps = u64::max(bitrate_bps, min * 1_000_000);
                }

                bitrate_bps
            }
        };

        let framerate = 1.0
            / self
                .frame_interval_average
                .get_average()
                .as_secs_f32()
                .min(1.0);

        FfiDynamicEncoderParams {
            updated: true,
            bitrate_bps,
            framerate,
        }
    }
}
