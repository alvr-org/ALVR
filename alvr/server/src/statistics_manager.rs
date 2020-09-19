use alvr_common::{data::ClientStatistics, *};
use logging::LogId;
use std::time::{Duration, Instant};

pub struct StatisticsManager {
    last_update_instant: Instant,
    encode_latency_sum: Duration,
    server_frames_count: usize,
}

impl StatisticsManager {
    pub fn new() -> Self {
        Self {
            last_update_instant: Instant::now(),
            encode_latency_sum: Duration::from_secs(0),
            server_frames_count: 0,
        }
    }

    pub fn update(&mut self, data: ClientStatistics) {
        let now = Instant::now();

        let total_latency_ms = data.average_total_latency.as_millis() as _;
        let encode_latency_ms =
            (self.encode_latency_sum.as_millis() as f32 / self.server_frames_count as f32) as _;
        let decode_latency_ms = data.average_decode_latency.as_millis() as _;

        info!(id: LogId::Statistics {
            total_latency_ms,
            encode_latency_ms,
            decode_latency_ms,
            other_latency_ms: total_latency_ms - encode_latency_ms - decode_latency_ms,
            client_fps: data.fps,
            server_fps:
                self.server_frames_count as f32 / (now - self.last_update_instant).as_secs_f32(),
        });

        self.last_update_instant = now;
        self.encode_latency_sum = Duration::from_secs(0);
        self.server_frames_count = 0;
    }

    pub fn report_encode_latency(&mut self, latency: Duration) {
        self.encode_latency_sum += latency;
    }
}
