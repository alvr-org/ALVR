use glam::Vec2;
use std::{
    collections::VecDeque,
    f32::consts::PI,
    time::{Duration, Instant},
};

pub struct SlidingWindowAverage<T> {
    history_buffer: VecDeque<T>,
    max_history_size: usize,
}

impl<T> SlidingWindowAverage<T> {
    pub fn new(initial_value: T, max_history_size: usize) -> Self {
        Self {
            history_buffer: [initial_value].into_iter().collect(),
            max_history_size,
        }
    }

    pub fn submit_sample(&mut self, sample: T) {
        if self.history_buffer.len() >= self.max_history_size {
            self.history_buffer.pop_front();
        }

        self.history_buffer.push_back(sample);
    }
}

impl SlidingWindowAverage<f32> {
    pub fn get_average(&self) -> f32 {
        self.history_buffer.iter().sum::<f32>() / self.history_buffer.len() as f32
    }
}

impl SlidingWindowAverage<Duration> {
    pub fn get_average(&self) -> Duration {
        self.history_buffer.iter().sum::<Duration>() / self.history_buffer.len() as u32
    }
}

pub enum DurationOffset {
    Positive(Duration),
    Negative(Duration),
}

// Calculate average time phase. The average is calulated in the complex domain and returned back as
// the phase time offset. Only the phase samples are stored (as a complex number), not the frame
// interval, since it's useless for these calculations.
pub struct SlidingTimePhaseAverage {
    last_time_sample: Instant,
    last_sample: Vec2,
    history_buffer: VecDeque<Vec2>,
    max_history_size: usize,
}

impl SlidingTimePhaseAverage {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            last_time_sample: Instant::now(),
            last_sample: Vec2::new(1.0, 0.0),
            history_buffer: [].into_iter().collect(),
            max_history_size,
        }
    }

    // The sample is actually the time of this call.
    pub fn submit_sample(&mut self, frame_interval: Duration) {
        let time_sample = Instant::now();

        let phase_sample = ((time_sample - self.last_time_sample).as_secs_f32()
            / frame_interval.as_secs_f32())
        .fract()
            * 2.0
            * PI;

        let complex_sample = Vec2::new(f32::cos(phase_sample), f32::sin(phase_sample));

        if self.history_buffer.len() >= self.max_history_size {
            self.history_buffer.pop_front();
        }

        self.history_buffer.push_back(complex_sample);

        self.last_time_sample = time_sample;
        self.last_sample = complex_sample
    }

    // The reference frame of the phase average is an implementation detail. This method returns the
    // phase offset wrt the time of this call.
    pub fn get_average_diff(&self, frame_interval: Duration) -> DurationOffset {
        let now = Instant::now();

        let current_phase =
            ((now - self.last_time_sample).as_secs_f32() / frame_interval.as_secs_f32()).fract()
                * 2.0
                * PI;

        // let current_complex_phase = Vec2::new(f32::cos(current_phase), f32::sin(current_phase));

        // Note: no need to normalize
        let average_complex = self.history_buffer.iter().sum::<Vec2>();
        let average_phase = f32::atan2(average_complex.y, average_complex.x);

        let phase_diff = current_phase - average_phase;

        // Nomalized between -PI and +PI
        let normalized_phase_diff = (phase_diff + PI).rem_euclid(2.0 * PI) - PI;

        if normalized_phase_diff.is_sign_positive() {
            DurationOffset::Positive(Duration::from_secs_f32(
                normalized_phase_diff * frame_interval.as_secs_f32(),
            ))
        } else {
            DurationOffset::Negative(Duration::from_secs_f32(
                -normalized_phase_diff * frame_interval.as_secs_f32(),
            ))
        }
    }
}
