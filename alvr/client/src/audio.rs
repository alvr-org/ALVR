use alvr_common::{data::AudioConfig, *};
use oboe::*;
use std::{collections::VecDeque, sync::mpsc::Receiver, thread, time::Duration};
use tokio::sync::mpsc::UnboundedSender;

struct RecorderCallback {
    sender: UnboundedSender<Vec<u8>>,
    sample_buffer: Vec<u8>,
}

impl AudioInputCallback for RecorderCallback {
    type FrameType = (i16, Mono);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioInputStreamSafe,
        audio_data: &[i16],
    ) -> DataCallbackResult {
        self.sample_buffer.clear();

        // this code is inefficient but Oboe will be replaced by CPAL at one point
        for frame in audio_data {
            let pair = frame.to_ne_bytes();
            self.sample_buffer.extend(&pair);
        }

        self.sender.send(self.sample_buffer.clone());

        DataCallbackResult::Continue
    }
}

pub struct AudioRecorder {
    stream: AudioStreamAsync<Input, RecorderCallback>,
}

unsafe impl Send for AudioRecorder {}

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
            .set_callback(RecorderCallback {
                sender,
                sample_buffer: vec![],
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        Ok(Self { stream })
    }
}

const OUTPUT_FRAME_SIZE: usize = 2 * 2;

struct PlayerCallback {
    receiver: Receiver<Vec<u8>>,
    sample_buffer: VecDeque<u8>,
}

impl AudioOutputCallback for PlayerCallback {
    type FrameType = (i16, Stereo);

    fn on_audio_ready(
        &mut self,
        audio_stream: &mut dyn AudioOutputStreamSafe,
        frames: &mut [(i16, i16)],
    ) -> DataCallbackResult {
        for frame in frames {
            frame.0 = rand::random::<i16>();
            frame.1 = rand::random::<i16>();
        }

        // while let Ok(packet) = self.receiver.try_recv() {
        //     self.sample_buffer.extend(packet);
        // }

        // thread::sleep(Duration::from_millis(1));
        // let data_ref = data.bytes_mut();

        // if sample_buffer.len() >= data_ref.len() {
        //     data_ref.copy_from_slice(&sample_buffer.drain(0..data_ref.len()).collect::<Vec<_>>())
        // }

        // // trickle drain overgrown buffer. todo: use smarter policy with EventTiming
        // if sample_buffer.len()
        //     >= data_ref.len() * config.max_buffer_count_extra as usize + frame_size
        // {
        //     sample_buffer.drain(0..frame_size);
        // }

        DataCallbackResult::Continue
    }
}

pub struct AudioPlayer {
    stream: AudioStreamAsync<Output, PlayerCallback>,
}

unsafe impl Send for AudioPlayer {}

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
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        Ok(Self { stream })
    }
}
