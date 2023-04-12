use crate::FfiDynamicEncoderParams;
use alvr_common::SlidingWindowAverage;
use alvr_session::{settings_schema::Switch, BitrateConfig, BitrateMode};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

pub struct BitrateManager {
    config: BitrateConfig,
    nominal_framerate: f32,
    max_history_size: usize,
    frame_interval_average: SlidingWindowAverage<Duration>,
    packet_sizes_bits_history: VecDeque<(Duration, usize)>,
    network_latency_average: SlidingWindowAverage<Duration>,
    bitrate_average: SlidingWindowAverage<f32>,
    decoder_latency_overstep_count: usize,
    last_frame_instant: Instant,
    last_update_instant: Instant,
    dynamic_max_bitrate: f32,
    update_needed: bool,
}

impl BitrateManager {
    pub fn new(config: BitrateConfig, max_history_size: usize, nominal_framerate: f32) -> Self {
        Self {
            config,
            nominal_framerate,
            max_history_size,
            frame_interval_average: SlidingWindowAverage::new(
                Duration::from_millis(16),
                max_history_size,
            ),
            packet_sizes_bits_history: VecDeque::new(),
            network_latency_average: SlidingWindowAverage::new(
                Duration::from_millis(5),
                max_history_size,
            ),
            bitrate_average: SlidingWindowAverage::new(30_000_000.0, max_history_size),
            decoder_latency_overstep_count: 0,
            last_frame_instant: Instant::now(),
            last_update_instant: Instant::now(),
            dynamic_max_bitrate: f32::MAX,
            update_needed: true,
        }
    }

    // Note: This is used to calculate the framerate/frame interval. The frame present is the most
    // accurate event for this use.
    pub fn report_frame_present(&mut self) {
        let now = Instant::now();

        let interval = now - self.last_frame_instant;
        self.last_frame_instant = now;

        if let Switch::Enabled(config) = &self.config.adapt_to_framerate {
            let interval_ratio =
                interval.as_secs_f32() / self.frame_interval_average.get_average().as_secs_f32();

            self.frame_interval_average.submit_sample(interval);

            if interval_ratio > config.framerate_reset_threshold_multiplier
                || interval_ratio < 1.0 / config.framerate_reset_threshold_multiplier
            {
                self.frame_interval_average =
                    SlidingWindowAverage::new(interval, self.max_history_size);
                self.update_needed = true;
            }
        }
    }

    pub fn report_encoded_frame_size(&mut self, timestamp: Duration, size_bytes: usize) {
        self.packet_sizes_bits_history
            .push_back((timestamp, size_bytes * 8));
    }

    // decoder_latency is used to learn a suitable maximum bitrate bound to avoid decoder runaway
    // latency
    pub fn report_frame_latencies(
        &mut self,
        timestamp: Duration,
        network_latency: Duration,
        decoder_latency: Duration,
    ) {
        if network_latency == Duration::ZERO {
            return;
        }

        while let Some(&(timestamp_, size_bits)) = self.packet_sizes_bits_history.front() {
            if timestamp_ == timestamp {
                self.bitrate_average
                    .submit_sample(size_bits as f32 / network_latency.as_secs_f32());

                self.packet_sizes_bits_history.pop_front();

                break;
            } else {
                self.packet_sizes_bits_history.pop_front();
            }
        }

        if let BitrateMode::Adaptive {
            decoder_latency_fixer: Switch::Enabled(config),
            ..
        } = &self.config.mode
        {
            if decoder_latency > Duration::from_millis(config.max_decoder_latency_ms) {
                self.decoder_latency_overstep_count += 1;

                if self.decoder_latency_overstep_count == config.latency_overstep_frames as usize {
                    self.dynamic_max_bitrate =
                        f32::min(self.bitrate_average.get_average(), self.dynamic_max_bitrate)
                            * config.latency_overstep_multiplier;

                    self.update_needed = true;

                    self.decoder_latency_overstep_count = 0;
                }
            } else {
                self.decoder_latency_overstep_count = 0;
            }
        }
    }

    pub fn get_encoder_params(&mut self) -> FfiDynamicEncoderParams {
        let now = Instant::now();
        if self.update_needed || now > self.last_update_instant + UPDATE_INTERVAL {
            self.last_update_instant = now;
        } else {
            return FfiDynamicEncoderParams {
                updated: 0,
                bitrate_bps: 0,
                framerate: 0.0,
            };
        }

        let bitrate_bps = match &self.config.mode {
            BitrateMode::ConstantMbps(bitrate_mbps) => *bitrate_mbps as f32 * 1e6,
            BitrateMode::Adaptive {
                saturation_multiplier,
                max_bitrate_mbps,
                min_bitrate_mbps,
                max_network_latency_ms,
                ..
            } => {
                let mut bitrate_bps = self.bitrate_average.get_average() * saturation_multiplier;

                if let Switch::Enabled(max) = max_bitrate_mbps {
                    bitrate_bps = f32::min(bitrate_bps, *max as f32 * 1e6);
                }
                if let Switch::Enabled(min) = min_bitrate_mbps {
                    bitrate_bps = f32::max(bitrate_bps, *min as f32 * 1e6);
                }

                if let Switch::Enabled(max_ms) = max_network_latency_ms {
                    let multiplier = *max_ms as f32
                        / 1000.0
                        / self.network_latency_average.get_average().as_secs_f32();
                    bitrate_bps = f32::min(bitrate_bps, bitrate_bps * multiplier);
                }

                bitrate_bps = f32::min(bitrate_bps, self.dynamic_max_bitrate);

                bitrate_bps
            }
        };

        let framerate = if self.config.adapt_to_framerate.enabled() {
            1.0 / self
                .frame_interval_average
                .get_average()
                .as_secs_f32()
                .min(1.0)
        } else {
            self.nominal_framerate
        };

        FfiDynamicEncoderParams {
            updated: 1,
            bitrate_bps: bitrate_bps as u64,
            framerate,
        }
    }
}
