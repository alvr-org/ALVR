use alvr_common::{HEAD_ID, LEFT_HAND_ID, RIGHT_HAND_ID};
use alvr_events::{EventType, GraphStatistics, Statistics};
use alvr_sockets::ClientStatistics;
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

const FULL_REPORT_INTERVAL: Duration = Duration::from_millis(500);

pub struct HistoryFrame {
    target_timestamp: Duration,
    tracking_received: Instant,
    frame_present: Instant,
    frame_composed: Instant,
    frame_encoded: Instant,
    total_pipeline_latency: Duration,
}

impl Default for HistoryFrame {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            target_timestamp: Duration::ZERO,
            tracking_received: now,
            frame_present: now,
            frame_composed: now,
            frame_encoded: now,
            total_pipeline_latency: Duration::ZERO,
        }
    }
}

pub struct StatisticsManager {
    history_buffer: VecDeque<HistoryFrame>,
    max_history_size: usize,
    last_full_report_instant: Instant,
    video_packets_total: usize,
    video_packets_partial_sum: usize,
    video_bytes_total: usize,
    video_bytes_partial_sum: usize,
    fec_errors_total: usize,
    fec_failures_partial_sum: usize,
    fec_percentage: u32,
    battery_gauges: HashMap<u64, f32>,
}

impl StatisticsManager {
    // history size used to calculate average total pipeline latency
    pub fn new(history_size: usize) -> Self {
        Self {
            history_buffer: VecDeque::new(),
            max_history_size: history_size,
            last_full_report_instant: Instant::now(),
            video_packets_total: 0,
            video_packets_partial_sum: 0,
            video_bytes_total: 0,
            video_bytes_partial_sum: 0,
            fec_errors_total: 0,
            fec_failures_partial_sum: 0,
            fec_percentage: 0,
            battery_gauges: HashMap::new(),
        }
    }

    pub fn report_tracking_received(&mut self, target_timestamp: Duration) {
        if !self
            .history_buffer
            .iter()
            .any(|frame| frame.target_timestamp == target_timestamp)
        {
            self.history_buffer.push_front(HistoryFrame {
                target_timestamp,
                tracking_received: Instant::now(),
                ..Default::default()
            });
        }

        if self.history_buffer.len() > self.max_history_size {
            self.history_buffer.pop_back();
        }
    }

    pub fn report_frame_present(&mut self, target_timestamp: Duration) {
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == target_timestamp)
        {
            frame.frame_present = Instant::now();
        }
    }

    pub fn report_frame_composed(&mut self, target_timestamp: Duration) {
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == target_timestamp)
        {
            frame.frame_composed = Instant::now();
        }
    }

    pub fn report_frame_encoded(&mut self, target_timestamp: Duration) {
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == target_timestamp)
        {
            frame.frame_encoded = Instant::now();
        }
    }

    pub fn report_video_packet(&mut self, bytes_count: usize) {
        self.video_packets_total += 1;
        self.video_packets_partial_sum += 1;
        self.video_bytes_total += bytes_count;
        self.video_bytes_partial_sum += bytes_count;
    }

    pub fn report_fec_failure(&mut self, fec_percentage: u32) {
        self.fec_percentage = fec_percentage;
        self.fec_errors_total += 1;
        self.fec_failures_partial_sum += 1;
    }

    pub fn report_battery(&mut self, device_id: u64, gauge_value: f32) {
        *self.battery_gauges.entry(device_id).or_default() = gauge_value;
    }

    // Called every frame. Some statistics are reported once every frame
    // Returns network latency
    pub fn report_statistics(
        &mut self,
        client_stats: ClientStatistics,
        game_frame_interval: Duration,
    ) -> Duration {
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == client_stats.target_timestamp)
        {
            frame.total_pipeline_latency = client_stats.total_pipeline_latency;

            let game_time_latency = frame
                .frame_present
                .saturating_duration_since(frame.tracking_received);

            let server_compositor_latency = frame
                .frame_composed
                .saturating_duration_since(frame.frame_present);

            let encoder_latency = frame
                .frame_encoded
                .saturating_duration_since(frame.frame_composed);

            // The network latency cannot be estiamed directly. It is what's left of the total
            // latency after subtracting all other latency intervals. In particular it contains the
            // transport latency of the tracking packet and the interval between the first video
            // packet is sent and the last video packet is received for a specific frame.
            // For safety, use saturating_sub to avoid a crash if for some reason the network
            // latency is miscalculated as negative.
            let network_latency = frame.total_pipeline_latency.saturating_sub(
                game_time_latency
                    + server_compositor_latency
                    + encoder_latency
                    + client_stats.video_decode
                    + client_stats.rendering
                    + client_stats.vsync_queue,
            );

            if self.last_full_report_instant + FULL_REPORT_INTERVAL < Instant::now() {
                self.last_full_report_instant += FULL_REPORT_INTERVAL;

                let interval_secs = FULL_REPORT_INTERVAL.as_secs_f32();

                alvr_events::send_event(EventType::Statistics(Statistics {
                    video_packets_total: self.video_packets_total,
                    video_packets_per_sec: (self.video_packets_partial_sum as f32 / interval_secs)
                        as _,
                    video_mbytes_total: (self.video_bytes_total as f32 / 1e6) as usize,
                    video_mbits_per_sec: self.video_bytes_partial_sum as f32 / interval_secs * 8.
                        / 1e6,
                    total_latency_ms: client_stats.total_pipeline_latency.as_secs_f32() * 1000.,
                    network_latency_ms: network_latency.as_secs_f32() * 1000.,
                    encode_latency_ms: encoder_latency.as_secs_f32() * 1000.,
                    decode_latency_ms: client_stats.video_decode.as_secs_f32() * 1000.,
                    fec_percentage: self.fec_percentage,
                    fec_errors_total: self.fec_errors_total,
                    fec_errors_per_sec: (self.fec_failures_partial_sum as f32 / interval_secs) as _,
                    client_fps: (1. / client_stats.frame_interval.as_secs_f32()) as _,
                    server_fps: (1. / game_frame_interval.as_secs_f32()) as _,
                    battery_hmd: (self
                        .battery_gauges
                        .get(&HEAD_ID)
                        .cloned()
                        .unwrap_or_default()
                        * 100.) as _,
                    battery_left: (self
                        .battery_gauges
                        .get(&LEFT_HAND_ID)
                        .cloned()
                        .unwrap_or_default()
                        * 100.) as _,
                    battery_right: (self
                        .battery_gauges
                        .get(&RIGHT_HAND_ID)
                        .cloned()
                        .unwrap_or_default()
                        * 100.) as _,
                }));

                self.video_packets_partial_sum = 0;
                self.video_bytes_partial_sum = 0;
                self.fec_failures_partial_sum = 0;
            }

            // todo: use target timestamp in nanoseconds. the dashboard needs to use the first
            // timestamp as the graph time origin.
            alvr_events::send_event(EventType::GraphStatistics(GraphStatistics {
                total_pipeline_latency_s: client_stats.total_pipeline_latency.as_secs_f32(),
                game_time_s: game_time_latency.as_secs_f32(),
                server_compositor_s: server_compositor_latency.as_secs_f32(),
                encoder_s: encoder_latency.as_secs_f32(),
                network_s: network_latency.as_secs_f32(),
                decoder_s: client_stats.video_decode.as_secs_f32(),
                client_compositor_s: client_stats.rendering.as_secs_f32(),
                vsync_queue_s: client_stats.vsync_queue.as_secs_f32(),
                client_fps: 1. / client_stats.frame_interval.as_secs_f32(),
                server_fps: 1. / game_frame_interval.as_secs_f32(),
            }));

            network_latency
        } else {
            Duration::ZERO
        }
    }
}
