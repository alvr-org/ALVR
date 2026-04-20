use glam::Vec2;
use std::{collections::VecDeque, num::NonZeroUsize, time::Duration};

pub struct SlidingWindowAverage<T> {
    history_buffer: VecDeque<T>,
    max_history_size: usize,
}

impl<T> SlidingWindowAverage<T> {
    pub fn new(initial_value: T, max_history_size: usize) -> Self {
        let mut history_buffer = VecDeque::with_capacity(max_history_size);
        history_buffer.push_back(initial_value);

        Self {
            history_buffer,
            max_history_size,
        }
    }

    pub fn submit_sample(&mut self, sample: T) {
        if self.history_buffer.len() >= self.max_history_size {
            self.history_buffer.pop_front();
        }

        self.history_buffer.push_back(sample);
    }

    pub fn retain(&mut self, count: NonZeroUsize) {
        self.history_buffer
            .drain(0..self.history_buffer.len().saturating_sub(count.get()));
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

impl SlidingWindowAverage<Vec2> {
    pub fn get_average(&self) -> Vec2 {
        self.history_buffer.iter().sum::<Vec2>() / self.history_buffer.len() as f32
    }
}

pub struct AngleSlidingWindowAverage {
    inner: SlidingWindowAverage<Vec2>,
}

impl AngleSlidingWindowAverage {
    pub fn new(initial_value: f32, max_history_size: usize) -> Self {
        Self {
            inner: SlidingWindowAverage::new(Vec2::from_angle(initial_value), max_history_size),
        }
    }

    pub fn submit_sample(&mut self, sample: f32) {
        self.inner.submit_sample(Vec2::from_angle(sample));
    }

    pub fn get_average(&self) -> f32 {
        self.inner.get_average().to_angle()
    }
}
