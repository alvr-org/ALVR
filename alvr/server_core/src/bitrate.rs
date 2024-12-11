use alvr_common::SlidingWindowAverage;
use alvr_events::BitrateDirectives;
use alvr_session::{
    settings_schema::Switch, BitrateAdaptiveFramerateConfig, BitrateConfig, BitrateMode,
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

pub struct DynamicEncoderParams {
    pub bitrate_bps: f32,
    pub framerate: f32,
}

pub struct BitrateManager {
    nominal_frame_interval: Duration,
    frame_interval_average: SlidingWindowAverage<Duration>,
    // note: why packet_sizes_bits_history is a queue and not a sliding average? Because some
    // network samples will be dropped but not any packet size sample
    packet_bytes_history: VecDeque<(Duration, usize)>,
    packet_bytes_average: SlidingWindowAverage<f32>,
    network_latency_average: SlidingWindowAverage<Duration>,
    encoder_latency_average: SlidingWindowAverage<Duration>,
    decoder_latency_overstep_count: usize,
    last_frame_instant: Instant,
    last_update_instant: Instant,
    dynamic_decoder_max_bytes_per_frame: f32,
    previous_config: Option<BitrateConfig>,
    update_needed: bool,
}

impl BitrateManager {
    pub fn new(max_history_size: usize, initial_framerate: f32) -> Self {
        Self {
            nominal_frame_interval: Duration::from_secs_f32(1. / initial_framerate),
            frame_interval_average: SlidingWindowAverage::new(
                Duration::from_millis(16),
                max_history_size,
            ),
            packet_bytes_history: VecDeque::new(),
            packet_bytes_average: SlidingWindowAverage::new(50000.0, max_history_size),
            network_latency_average: SlidingWindowAverage::new(
                Duration::from_millis(5),
                max_history_size,
            ),
            encoder_latency_average: SlidingWindowAverage::new(
                Duration::from_millis(5),
                max_history_size,
            ),
            decoder_latency_overstep_count: 0,
            last_frame_instant: Instant::now(),
            last_update_instant: Instant::now(),
            dynamic_decoder_max_bytes_per_frame: f32::MAX,
            previous_config: None,
            update_needed: true,
        }
    }

    // Note: This is used to calculate the framerate/frame interval. The frame present is the most
    // accurate event for this use.
    pub fn report_frame_present(&mut self, config: &Switch<BitrateAdaptiveFramerateConfig>) {
        let now = Instant::now();

        let interval = now - self.last_frame_instant;
        self.last_frame_instant = now;

        if let Some(config) = config.as_option() {
            let interval_ratio =
                interval.as_secs_f32() / self.frame_interval_average.get_average().as_secs_f32();

            self.frame_interval_average.submit_sample(interval);

            if interval_ratio > config.framerate_reset_threshold_multiplier
                || interval_ratio < 1.0 / config.framerate_reset_threshold_multiplier
            {
                // Clear most of the samples, keep some for stability
                self.frame_interval_average.retain(5);
                self.update_needed = true;
            }
        }
    }

    pub fn report_frame_encoded(
        &mut self,
        timestamp: Duration,
        encoder_latency: Duration,
        size_bytes: usize,
    ) {
        self.encoder_latency_average.submit_sample(encoder_latency);

        self.packet_bytes_history.push_back((timestamp, size_bytes));
    }

    // decoder_latency is used to learn a suitable maximum bitrate bound to avoid decoder runaway
    // latency
    pub fn report_frame_latencies(
        &mut self,
        config: &BitrateMode,
        timestamp: Duration,
        network_latency: Duration,
        decoder_latency: Duration,
    ) {
        if network_latency.is_zero() {
            return;
        }

        while let Some(&(history_timestamp, size_bytes)) = self.packet_bytes_history.front() {
            if history_timestamp == timestamp {
                self.packet_bytes_average.submit_sample(size_bytes as f32);
                self.network_latency_average.submit_sample(network_latency);

                self.packet_bytes_history.pop_front();

                break;
            } else {
                self.packet_bytes_history.pop_front();
            }
        }

        if let BitrateMode::Adaptive {
            decoder_latency_limiter: Switch::Enabled(config),
            ..
        } = &config
        {
            if decoder_latency > Duration::from_millis(config.max_decoder_latency_ms) {
                self.decoder_latency_overstep_count += 1;

                if self.decoder_latency_overstep_count == config.latency_overstep_frames {
                    self.dynamic_decoder_max_bytes_per_frame = f32::min(
                        self.packet_bytes_average.get_average(),
                        self.dynamic_decoder_max_bytes_per_frame,
                    ) * config
                        .latency_overstep_multiplier;

                    self.update_needed = true;

                    self.decoder_latency_overstep_count = 0;
                }
            } else {
                self.decoder_latency_overstep_count = 0;
            }
        }
    }

    pub fn get_encoder_params(
        &mut self,
        config: &BitrateConfig,
    ) -> Option<(DynamicEncoderParams, BitrateDirectives)> {
        let now = Instant::now();

        if self
            .previous_config
            .as_ref()
            .map(|prev| config != prev)
            .unwrap_or(true)
        {
            self.previous_config = Some(config.clone());
            // Continue method. Always update bitrate in this case
        } else if !self.update_needed
            && (now < self.last_update_instant + UPDATE_INTERVAL
                || matches!(config.mode, BitrateMode::ConstantMbps(_)))
        {
            return None;
        }

        self.last_update_instant = now;
        self.update_needed = false;

        let frame_interval = if config.adapt_to_framerate.enabled() {
            self.frame_interval_average.get_average()
        } else {
            self.nominal_frame_interval
        };

        let mut bitrate_directives = BitrateDirectives::default();

        let bitrate_bps = match &config.mode {
            BitrateMode::ConstantMbps(bitrate_mbps) => *bitrate_mbps as f32 * 1e6,
            BitrateMode::Adaptive {
                saturation_multiplier,
                max_throughput_mbps,
                min_throughput_mbps,
                max_network_latency_ms,
                encoder_latency_limiter,
                decoder_latency_limiter,
            } => {
                let packet_bytes_average = self.packet_bytes_average.get_average();
                let network_latency_average_s =
                    self.network_latency_average.get_average().as_secs_f32();

                let mut throughput_bps =
                    packet_bytes_average * 8.0 * saturation_multiplier / network_latency_average_s;
                bitrate_directives.scaled_calculated_throughput_bps = Some(throughput_bps);

                if decoder_latency_limiter.enabled() {
                    throughput_bps =
                        f32::min(throughput_bps, self.dynamic_decoder_max_bytes_per_frame);
                    bitrate_directives.decoder_latency_limiter_bps =
                        Some(self.dynamic_decoder_max_bytes_per_frame);
                }

                if let Switch::Enabled(max_ms) = max_network_latency_ms {
                    let max_bps =
                        throughput_bps * (*max_ms as f32 / 1000.0) / network_latency_average_s;
                    throughput_bps = f32::min(throughput_bps, max_bps);

                    bitrate_directives.network_latency_limiter_bps = Some(max_bps);
                }

                if let Switch::Enabled(config) = encoder_latency_limiter {
                    // Note: this assumes linear relationship between bitrate and encoder latency
                    // but this may not be the case
                    let saturation = self.encoder_latency_average.get_average().as_secs_f32()
                        / self.nominal_frame_interval.as_secs_f32();
                    let max_bps = throughput_bps * config.max_saturation_multiplier / saturation;
                    bitrate_directives.encoder_latency_limiter_bps = Some(max_bps);

                    if saturation > config.max_saturation_multiplier {
                        throughput_bps = f32::min(throughput_bps, max_bps);
                    }
                }

                if let Switch::Enabled(max) = max_throughput_mbps {
                    let max_bps = *max as f32 * 1e6;
                    throughput_bps = f32::min(throughput_bps, max_bps);

                    bitrate_directives.manual_max_throughput_bps = Some(max_bps);
                }
                if let Switch::Enabled(min) = min_throughput_mbps {
                    let min_bps = *min as f32 * 1e6;
                    throughput_bps = f32::max(throughput_bps, min_bps);

                    bitrate_directives.manual_min_throughput_bps = Some(min_bps);
                }

                // NB: Here we assign the calculated throughput to the requested bitrate. This is
                // crucial for the working of the adaptive bitrate algorithm. The goal is to
                // optimally occupy the available bandwidth, which is when the bitrate corresponds
                // to the throughput.
                throughput_bps
            }
        };

        bitrate_directives.requested_bitrate_bps = bitrate_bps;

        Some((
            DynamicEncoderParams {
                bitrate_bps,
                framerate: 1.0 / f32::min(frame_interval.as_secs_f32(), 1.0),
            },
            bitrate_directives,
        ))
    }
}
