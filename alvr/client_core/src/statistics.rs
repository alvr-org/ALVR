use alvr_common::SlidingWindowAverage;
use alvr_packets::{ClientStatistics, VideoFramePresentedPacket};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

struct HistoryFrame {
    target_timestamp: Duration,
    input_acquired: Instant,
    video_packet_received: Instant,
    video_frame_decoded: Instant,
    video_frame_dequeued: Instant,
}

pub struct StatisticsManager {
    history_buffer: VecDeque<HistoryFrame>,
    max_history_size: usize,
    total_pipeline_latency_average: SlidingWindowAverage<Duration>,
    steamvr_pipeline_latency: Duration,
}

impl StatisticsManager {
    pub fn new(
        max_history_size: usize,
        nominal_server_frame_interval: Duration,
        steamvr_pipeline_frames: f32,
    ) -> Self {
        Self {
            max_history_size,
            history_buffer: VecDeque::new(),
            total_pipeline_latency_average: SlidingWindowAverage::new(
                Duration::ZERO,
                max_history_size,
            ),
            steamvr_pipeline_latency: Duration::from_secs_f32(
                steamvr_pipeline_frames * nominal_server_frame_interval.as_secs_f32(),
            ),
        }
    }

    pub fn report_input_acquired(&mut self, target_timestamp: Duration) {
        let now = Instant::now();

        if !self
            .history_buffer
            .iter()
            .any(|frame| frame.target_timestamp == target_timestamp)
        {
            self.history_buffer.push_front(HistoryFrame {
                target_timestamp,
                input_acquired: now,
                video_packet_received: now,
                video_frame_decoded: now,
                video_frame_dequeued: now,
            });
        }

        if self.history_buffer.len() > self.max_history_size {
            self.history_buffer.pop_back();
        }
    }

    pub fn report_video_packet_received(&mut self, target_timestamp: Duration) {
        let now = Instant::now();
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == target_timestamp)
        {
            frame.video_packet_received = now;
        }
    }

    // returns: decoding duration
    pub fn report_frame_decoded(&mut self, target_timestamp: Duration) -> Option<Duration> {
        let now = Instant::now();
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == target_timestamp)
        {
            frame.video_frame_decoded = now;

            Some(frame.video_frame_decoded - frame.video_packet_received)
        } else {
            None
        }
    }

    pub fn report_compositor_start(&mut self, target_timestamp: Duration) {
        let now = Instant::now();
        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == target_timestamp)
        {
            frame.video_frame_dequeued = now;
        }
    }

    // vsync_queue is the latency between this call and the vsync. it cannot be measured by ALVR and
    // should be reported by the VR runtime
    pub fn report_submit(&mut self, target_timestamp: Duration, vsync_queue: Duration, frame_interval: Duration) -> Option<VideoFramePresentedPacket>{
        let now = Instant::now();

        if let Some(frame) = self
            .history_buffer
            .iter_mut()
            .find(|frame| frame.target_timestamp == target_timestamp)
        {
            let mut packet = VideoFramePresentedPacket::default();
            packet.target_timestamp = target_timestamp;
            packet.decoder_queue = frame.video_frame_dequeued.saturating_duration_since(frame.video_frame_decoded);
            packet.rendering = now.saturating_duration_since(frame.video_frame_dequeued);
            packet.vsync_queue = vsync_queue;
            packet.total_pipeline_latency = now.saturating_duration_since(frame.input_acquired) + vsync_queue;

            Some(packet)
        } else {
            None
        }
    }

    // latency used for head prediction
    pub fn average_total_pipeline_latency(&self) -> Duration {
        self.total_pipeline_latency_average.get_average()
    }

    // latency used for controllers/trackers prediction
    pub fn tracker_prediction_offset(&self) -> Duration {
        self.total_pipeline_latency_average
            .get_average()
            .saturating_sub(self.steamvr_pipeline_latency)
    }
}
