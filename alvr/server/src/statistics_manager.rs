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

        info!(id: LogId::Statistics {
            packets_lost_total: data.packets_lost_total,
            packets_lost_per_second: data.packets_lost_per_second,
            total_latency_ms: data.average_total_latency.as_millis() as _,
            encode_latency_ms:
                (self.encode_latency_sum.as_millis() as f32 / self.server_frames_count as f32) as _,
            transport_latency_ms: data.average_transport_latency.as_millis() as _,
            decode_latency_ms: data.average_decode_latency.as_millis() as _,
            client_fps: data.fps,
            server_fps: (
                self.server_frames_count as f32 / (now - self.last_update_instant).as_secs_f32()
            ) as _,
        });

        self.last_update_instant = now;
        self.encode_latency_sum = Duration::from_secs(0);
        self.server_frames_count = 0;
    }

    pub fn report_encode_latency(&mut self, latency: Duration) {
        self.encode_latency_sum += latency;
    }
}
