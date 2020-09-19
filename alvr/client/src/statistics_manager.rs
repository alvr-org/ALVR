use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use alvr_common::data::ClientStatistics;

pub struct StatisticsManager {
    tracking_frame_times: VecDeque<(u64, Instant)>,
    last_decode_begin_time: Instant,
    decode_latency_sum: Duration,
    total_latency_sum: Duration,
    frame_count_per_sum: u32,
}

impl StatisticsManager {
    pub fn new() -> Self {
        Self {
            last_decode_begin_time: Instant::now(),
            tracking_frame_times: VecDeque::new(),
            decode_latency_sum: Duration::from_secs(0),
            total_latency_sum: Duration::from_secs(0),
            frame_count_per_sum: 0,
        }
    }

    pub fn report_tracking_frame(&mut self, frame_index: u64) {
        self.tracking_frame_times
            .push_back((frame_index, Instant::now()));
    }

    pub fn report_frame_to_be_decoded(&mut self) {
        self.last_decode_begin_time = Instant::now();
    }

    pub fn report_decoded_frame(&mut self) {
        self.decode_latency_sum += Instant::now() - self.last_decode_begin_time;
    }

    pub fn report_submitted_frame(&mut self, frame_index: u64) {
        while let Some(&(first_index, _)) = self.tracking_frame_times.front() {
            if first_index <= frame_index {
                if let Some((_, tracking_time)) = self.tracking_frame_times.pop_front() {
                    if first_index == frame_index {
                        self.total_latency_sum += Instant::now() - tracking_time;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        self.frame_count_per_sum += 1;
    }

    pub fn get_and_reset(&mut self) -> ClientStatistics {
        let statistics = ClientStatistics {
            average_total_latency: self.total_latency_sum / self.frame_count_per_sum,
            average_decode_latency: self.decode_latency_sum / self.frame_count_per_sum,
            fps: 1_f32 / self.frame_count_per_sum as f32,
        };

        self.total_latency_sum = Duration::from_secs(0);
        self.decode_latency_sum = Duration::from_secs(0);
        self.frame_count_per_sum = 0;

        statistics
    }
}
