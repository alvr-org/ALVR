use alvr_sockets::ClientStatistics;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

struct HistoryFrame {
    input_acquired: Instant,
    video_packet_received: Instant,
    intervals: ClientStatistics,
}

pub struct StatisticsManager {
    history_buffer: VecDeque<HistoryFrame>,
    max_history_size: usize,
    prev_vsync: Instant,
    last_average_total_pipeline_latency: Duration,
}

impl StatisticsManager {
    pub fn new(history_size: usize) -> Self {
        Self {
            max_history_size: history_size,
            history_buffer: VecDeque::new(),
            prev_vsync: Instant::now(),
            last_average_total_pipeline_latency: Duration::ZERO,
        }
    }

    pub fn report_input_acquired(&mut self, target_timestamp: Duration) {
        if !self
            .history_buffer
            .iter()
            .any(|frame| frame.intervals.target_timestamp == target_timestamp)
        {
            self.history_buffer.push_front(HistoryFrame {
                input_acquired: Instant::now(),
                // this is just a placeholder because Instant does not have a default value
                video_packet_received: Instant::now(),
                intervals: ClientStatistics {
                    target_timestamp,
                    ..Default::default()
                },
            });
        }

        if self.history_buffer.len() > self.max_history_size {
            self.history_buffer.pop_back();
        }
    }

    pub fn report_video_packet_received(&mut self, target_timestamp: Duration) {
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.intervals.target_timestamp == target_timestamp)
        {
            frame.video_packet_received = Instant::now();
        }
    }

    pub fn report_frame_decoded(&mut self, target_timestamp: Duration) {
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.intervals.target_timestamp == target_timestamp)
        {
            frame.intervals.video_decode =
                Instant::now().saturating_duration_since(frame.video_packet_received);
        }
    }

    pub fn report_compositor_start(&mut self, target_timestamp: Duration) {
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.intervals.target_timestamp == target_timestamp)
        {
            frame.intervals.video_decoder_queue = Instant::now().saturating_duration_since(
                frame.video_packet_received + frame.intervals.video_decode,
            );
        }
    }

    // vsync_queue is the latency between this call and the vsync. it cannot be measured by ALVR and
    // should be reported by the VR runtime
    pub fn report_submit(&mut self, target_timestamp: Duration, vsync_queue: Duration) {
        let now = Instant::now();

        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.intervals.target_timestamp == target_timestamp)
        {
            frame.intervals.rendering = now.saturating_duration_since(
                frame.video_packet_received
                    + frame.intervals.video_decode
                    + frame.intervals.video_decoder_queue,
            );
            frame.intervals.vsync_queue = vsync_queue;
            frame.intervals.total_pipeline_latency =
                now.saturating_duration_since(frame.input_acquired) + vsync_queue;

            let vsync = now + vsync_queue;
            frame.intervals.frame_interval = vsync.saturating_duration_since(self.prev_vsync);
            self.prev_vsync = vsync;
        }

        let mut frames_count = 0;
        let mut sum = Duration::ZERO;
        for frame in &self.history_buffer {
            if frame.intervals.total_pipeline_latency != Duration::ZERO {
                sum += frame.intervals.total_pipeline_latency;
                frames_count += 1;
            }
        }
        self.last_average_total_pipeline_latency = if frames_count > 0 {
            sum / frames_count
        } else {
            Duration::ZERO
        };

        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.intervals.target_timestamp == target_timestamp)
        {
            frame.intervals.average_total_pipeline_latency =
                self.last_average_total_pipeline_latency;
        }
    }

    pub fn summary(&self, target_timestamp: Duration) -> Option<ClientStatistics> {
        self.history_buffer
            .iter()
            .find(|frame| frame.intervals.target_timestamp == target_timestamp)
            .map(|frame| frame.intervals.clone())
    }

    // latency used for prediction
    pub fn average_total_pipeline_latency(&self) -> Duration {
        self.last_average_total_pipeline_latency
    }
}
