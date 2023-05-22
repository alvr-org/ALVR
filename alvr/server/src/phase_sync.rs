//! Server phase sync
//! Phase sync on the server is concerned with vsync cycle handling and tracking timing.

use alvr_common::SlidingWindowAverage;
use std::time::{Duration, Instant};

pub struct PhaseSyncManager {
    predicted_frame_interval_average: SlidingWindowAverage<Duration>,
    last_vsync_time: Instant,
}

impl PhaseSyncManager {
    pub fn new(max_history_size: usize, initial_frame_interval: Duration) -> Self {
        Self {
            predicted_frame_interval_average: SlidingWindowAverage::new(
                initial_frame_interval,
                max_history_size,
            ),
            last_vsync_time: Instant::now(),
        }
    }

    pub fn report_predicted_frame_interval(&mut self, interval: Duration) {
        self.predicted_frame_interval_average
            .submit_sample(interval);
    }

    pub fn frame_interval_average(&self) -> Duration {
        self.predicted_frame_interval_average.get_average()
    }

    // NB: this call is non-blocking, waiting should be done externally
    pub fn duration_until_next_vsync(&mut self) -> Duration {
        let now = Instant::now();

        let frame_interval = self.predicted_frame_interval_average.get_average();

        // update the last vsync if it's too old
        while self.last_vsync_time + frame_interval < now {
            self.last_vsync_time += frame_interval;
        }

        (self.last_vsync_time + frame_interval).saturating_duration_since(now)
    }
}
