use std::time::Duration;

use alvr_common::SlidingWindowAverage;
use alvr_session::BitrateDesc;
use settings_schema::Switch;

pub struct BitrateManager {
    desc: BitrateDesc,
    frame_size_bits_average: SlidingWindowAverage<usize>,
    network_latency_average: SlidingWindowAverage<Duration>,
    last_timestamp: Duration,
}

impl BitrateManager {
    pub fn new(desc: BitrateDesc, max_history_size: usize) -> Self {
        Self {
            desc,
            frame_size_bits_average: SlidingWindowAverage::new(80_000, max_history_size),
            network_latency_average: SlidingWindowAverage::new(
                Duration::from_millis(16),
                max_history_size,
            ),
            last_timestamp: Duration::ZERO,
        }
    }

    // Note: this will be reported with the frequency of the server framerate
    pub fn report_encoded_frame(&mut self, size_bytes: usize) {
        self.frame_size_bits_average.submit_sample(size_bytes * 8);
    }

    // Note: this will be reported with the frequency of the client framerate
    pub fn report_frame_network_latency(&mut self, timestamp: Duration, latency: Duration) {
        if timestamp != self.last_timestamp {
            self.network_latency_average.submit_sample(latency);
        }
    }

    pub fn get_bitrate_bps(&self) -> u64 {
        match &self.desc {
            BitrateDesc::ConstantMbs(bitrate_mbs) => *bitrate_mbs * 1_000_000,
            BitrateDesc::Adaptive {
                saturation_multiplier,
                max_bitrate_mbs,
                min_bitrate_mbs,
            } => {
                let bits_average = self.frame_size_bits_average.get_average();
                let latency_average = self.network_latency_average.get_average();

                let mut bitrate_bits = (bits_average as f32 / latency_average.as_secs_f32()
                    * saturation_multiplier) as u64;

                if let Switch::Enabled(max) = max_bitrate_mbs {
                    bitrate_bits = u64::min(bitrate_bits, max * 1_000_000);
                }
                if let Switch::Enabled(min) = min_bitrate_mbs {
                    bitrate_bits = u64::max(bitrate_bits, min * 1_000_000);
                }

                bitrate_bits
            }
        }
    }
}
