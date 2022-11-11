use std::{collections::VecDeque, time::Duration};

pub struct SlidingWindowAverage<T> {
    history_buffer: VecDeque<T>,
    max_history_size: usize,
}

impl<T> SlidingWindowAverage<T> {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            history_buffer: VecDeque::new(),
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
        if !self.history_buffer.is_empty() {
            self.history_buffer.iter().sum::<f32>() / self.history_buffer.len() as f32
        } else {
            0.0
        }
    }
}

impl SlidingWindowAverage<Duration> {
    pub fn get_average(&self) -> Duration {
        if !self.history_buffer.is_empty() {
            self.history_buffer.iter().sum::<Duration>() / self.history_buffer.len() as u32
        } else {
            Duration::ZERO
        }
    }
}
