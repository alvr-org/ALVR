use alvr_common::*;
use oboe::*;
use std::{
    collections::VecDeque,
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
};
use tokio::sync::mpsc::UnboundedSender;

struct RecorderCallback {
    sender: UnboundedSender<Vec<u8>>,
    running: Arc<AtomicBool>,
}

impl AudioInputCallback for RecorderCallback {
    type FrameType = (i16, Mono);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioInputStreamSafe,
        frames: &[i16],
    ) -> DataCallbackResult {
        let mut sample_buffer = Vec::with_capacity(frames.len() * size_of::<i16>());

        for frame in frames {
            sample_buffer.extend(&frame.to_ne_bytes());
        }

        self.sender.send(sample_buffer.clone()).ok();

        if self.running.load(Ordering::Relaxed) {
            DataCallbackResult::Continue
        } else {
            DataCallbackResult::Stop
        }
    }
}

pub struct AudioRecorder {
    stream: AudioStreamAsync<Input, RecorderCallback>,
    running: Arc<AtomicBool>,
}

impl AudioRecorder {
    pub fn start(sample_rate: u32, sender: UnboundedSender<Vec<u8>>) -> StrResult<Self> {
        let running = Arc::new(AtomicBool::new(true));

        let mut stream = trace_err!(AudioStreamBuilder::default()
            .set_shared()
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sample_rate(sample_rate as _)
            .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
            .set_mono()
            .set_i16()
            .set_input()
            .set_usage(Usage::VoiceCommunication)
            .set_input_preset(InputPreset::VoiceCommunication)
            .set_callback(RecorderCallback {
                sender,
                running: Arc::new(AtomicBool::new(true)),
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        Ok(Self { stream, running })
    }
}
impl Drop for AudioRecorder {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);

        // This call gets stuck if the headset goes to sleep, but finishes when the headset wakes up
        self.stream.stop_with_timeout(0).ok();
    }
}

const OUTPUT_FRAME_SIZE: usize = 2 * size_of::<i16>();

struct PlayerCallback {
    receiver: Receiver<Vec<u8>>,
    sample_buffer: VecDeque<u8>,
    buffer_range_multiplier: usize,
    last_input_buffer_size: usize,
}

impl AudioOutputCallback for PlayerCallback {
    type FrameType = (i16, Stereo);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioOutputStreamSafe,
        frames: &mut [(i16, i16)],
    ) -> DataCallbackResult {
        while let Ok(packet) = self.receiver.try_recv() {
            self.last_input_buffer_size = packet.len();
            self.sample_buffer.extend(packet);
        }

        let frames_bytes_count = frames.len() * OUTPUT_FRAME_SIZE;
        if self.sample_buffer.len() >= frames_bytes_count {
            let buffer = self
                .sample_buffer
                .drain(0..frames_bytes_count)
                .collect::<Vec<_>>();

            for (idx, (left, right)) in frames.iter_mut().enumerate() {
                *left = i16::from_ne_bytes([
                    buffer[idx * OUTPUT_FRAME_SIZE],
                    buffer[idx * OUTPUT_FRAME_SIZE + 1],
                ]);
                *right = i16::from_ne_bytes([
                    buffer[idx * OUTPUT_FRAME_SIZE + 2],
                    buffer[idx * OUTPUT_FRAME_SIZE + 3],
                ]);
            }
        } else {
            error!("audio buffer too small! size: {}", self.sample_buffer.len());
        }

        // todo: use smarter policy with EventTiming
        if self.sample_buffer.len() > 2 * self.buffer_range_multiplier * self.last_input_buffer_size
        {
            error!("draining audio buffer. size: {}", self.sample_buffer.len());

            self.sample_buffer.drain(
                0..(self.sample_buffer.len()
                    - self.buffer_range_multiplier * self.last_input_buffer_size),
            );
        }

        DataCallbackResult::Continue
    }
}

pub struct AudioPlayer {
    stream: AudioStreamAsync<Output, PlayerCallback>,
}

impl AudioPlayer {
    pub fn start(
        sample_rate: u32,
        buffer_range_multiplier: u64,
        receiver: Receiver<Vec<u8>>,
    ) -> StrResult<Self> {
        let mut stream = trace_err!(AudioStreamBuilder::default()
            .set_shared()
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sample_rate(sample_rate as _)
            .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
            .set_stereo()
            .set_i16()
            .set_output()
            .set_usage(Usage::Game)
            .set_callback(PlayerCallback {
                receiver,
                sample_buffer: VecDeque::new(),
                buffer_range_multiplier: buffer_range_multiplier as _,
                last_input_buffer_size: 0,
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        Ok(Self { stream })
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        // Note: Oboe crahes if stream.stop() is NOT called on AudioPlayer
        self.stream.stop_with_timeout(0).ok();
    }
}
