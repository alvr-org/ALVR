use alvr_common::{data::AudioConfig, *};
use oboe::*;
use std::{collections::VecDeque, mem::size_of, sync::mpsc::Receiver};
use tokio::sync::mpsc::UnboundedSender;

struct RecorderCallback {
    sender: UnboundedSender<Vec<u8>>,
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

        DataCallbackResult::Continue
    }
}

pub struct AudioRecorder {
    _stream: AudioStreamAsync<Input, RecorderCallback>,
}

impl AudioRecorder {
    pub fn start(config: AudioConfig, sender: UnboundedSender<Vec<u8>>) -> StrResult<Self> {
        // Oboe doesn't support untyped callbacks. For convenience, not all configs are respected
        let mut stream = trace_err!(AudioStreamBuilder::default()
            .set_shared()
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sample_rate(config.sample_rate as _)
            .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
            .set_mono()
            .set_i16()
            .set_input()
            .set_usage(Usage::VoiceCommunication)
            .set_input_preset(InputPreset::VoiceCommunication)
            .set_callback(RecorderCallback { sender })
            .open_stream())?;

        trace_err!(stream.start())?;

        Ok(Self { _stream: stream })
    }
}

const OUTPUT_FRAME_SIZE: usize = 2 * size_of::<i16>();

struct PlayerCallback {
    receiver: Receiver<Vec<u8>>,
    sample_buffer: VecDeque<u8>,
    max_buffer_count_extra: usize,
}

impl AudioOutputCallback for PlayerCallback {
    type FrameType = (i16, Stereo);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioOutputStreamSafe,
        frames: &mut [(i16, i16)],
    ) -> DataCallbackResult {
        while let Ok(packet) = self.receiver.try_recv() {
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
        }

        // trickle drain overgrown buffer. todo: use smarter policy with EventTiming
        if self.sample_buffer.len()
            >= frames_bytes_count * self.max_buffer_count_extra + OUTPUT_FRAME_SIZE
        {
            self.sample_buffer.drain(0..OUTPUT_FRAME_SIZE);
        }

        DataCallbackResult::Continue
    }
}

pub struct AudioPlayer {
    _stream: AudioStreamAsync<Output, PlayerCallback>,
}

impl AudioPlayer {
    pub fn start(config: AudioConfig, receiver: Receiver<Vec<u8>>) -> StrResult<Self> {
        let mut stream = trace_err!(AudioStreamBuilder::default()
            .set_shared()
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sample_rate(config.sample_rate as _)
            .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
            .set_stereo()
            .set_i16()
            .set_output()
            .set_usage(Usage::Game)
            .set_callback(PlayerCallback {
                receiver,
                sample_buffer: VecDeque::new(),
                max_buffer_count_extra: config.max_buffer_count_extra as _,
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        Ok(Self { _stream: stream })
    }
}
