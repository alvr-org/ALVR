use alvr_audio::{AudioDevice, AudioRecordState};
use alvr_common::{
    parking_lot::{Mutex, RwLock},
    prelude::*,
    RelaxedAtomic,
};
use alvr_session::AudioBufferingConfig;
use alvr_sockets::{StreamReceiver, StreamSender};
use oboe::{
    AudioInputCallback, AudioInputStreamSafe, AudioOutputCallback, AudioOutputStreamSafe,
    AudioStream, AudioStreamBuilder, DataCallbackResult, InputPreset, Mono, PerformanceMode,
    SampleRateConversionQuality, Stereo, Usage,
};
use std::{collections::VecDeque, mem, sync::Arc, thread, time::Duration};
use tokio::runtime::Runtime;

struct RecorderCallback {
    runtime: Arc<RwLock<Option<Runtime>>>,
    sender: StreamSender<()>,
    state: Arc<Mutex<AudioRecordState>>,
}

impl AudioInputCallback for RecorderCallback {
    type FrameType = (i16, Mono);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioInputStreamSafe,
        frames: &[i16],
    ) -> DataCallbackResult {
        let mut sample_buffer = Vec::with_capacity(frames.len() * mem::size_of::<i16>());

        for frame in frames {
            sample_buffer.extend(&frame.to_ne_bytes());
        }

        if let Some(runtime) = &*self.runtime.read() {
            self.sender.send(runtime, &(), sample_buffer).ok();

            DataCallbackResult::Continue
        } else {
            *self.state.lock() = AudioRecordState::ShouldStop;

            DataCallbackResult::Stop
        }
    }

    fn on_error_before_close(&mut self, _: &mut dyn AudioInputStreamSafe, error: oboe::Error) {
        *self.state.lock() = AudioRecordState::Err(error.to_string());
    }
}

#[allow(unused_variables)]
pub fn record_audio_blocking(
    runtime: Arc<RwLock<Option<Runtime>>>,
    sender: StreamSender<()>,
    device: &AudioDevice,
    channels_count: u16,
    mute: bool,
) -> StrResult {
    let sample_rate = device.input_sample_rate()?;

    let state = Arc::new(Mutex::new(AudioRecordState::Recording));

    let mut stream = AudioStreamBuilder::default()
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
            runtime: Arc::clone(&runtime),
            sender,
            state: Arc::clone(&state),
        })
        .open_stream()
        .map_err(err!())?;

    let mut res = stream.start().map_err(err!());

    if res.is_ok() {
        while matches!(*state.lock(), AudioRecordState::Recording) && runtime.read().is_some() {
            thread::sleep(Duration::from_millis(500))
        }

        if let AudioRecordState::Err(e) = state.lock().clone() {
            res = Err(e);
        }
    }

    res
}

struct PlayerCallback {
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    batch_frames_count: usize,
}

impl AudioOutputCallback for PlayerCallback {
    type FrameType = (f32, Stereo);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioOutputStreamSafe,
        out_frames: &mut [(f32, f32)],
    ) -> DataCallbackResult {
        assert!(self.batch_frames_count == out_frames.len());

        let samples = alvr_audio::get_next_frame_batch(
            &mut *self.sample_buffer.lock(),
            2,
            self.batch_frames_count,
        );

        for f in 0..out_frames.len() {
            out_frames[f] = (samples[f * 2], samples[f * 2 + 1]);
        }

        DataCallbackResult::Continue
    }
}

#[allow(unused_variables)]
pub fn play_audio_loop(
    running: Arc<RelaxedAtomic>,
    device: AudioDevice,
    channels_count: u16,
    sample_rate: u32,
    config: AudioBufferingConfig,
    receiver: StreamReceiver<()>,
) -> StrResult {
    // the client sends invalid sample rates sometimes, and we crash if we try and use one
    // (batch_frames_count ends up zero and the audio callback gets confused)
    if sample_rate < 8000 {
        return fmt_e!("Invalid audio sample rate");
    }

    let batch_frames_count = sample_rate as usize * config.batch_ms as usize / 1000;
    let average_buffer_frames_count =
        sample_rate as usize * config.average_buffering_ms as usize / 1000;

    let sample_buffer = Arc::new(Mutex::new(VecDeque::new()));

    let mut stream = AudioStreamBuilder::default()
        .set_shared()
        .set_performance_mode(PerformanceMode::LowLatency)
        .set_sample_rate(sample_rate as _)
        .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
        .set_stereo()
        .set_f32()
        .set_frames_per_callback(batch_frames_count as _)
        .set_output()
        .set_usage(Usage::Game)
        .set_callback(PlayerCallback {
            sample_buffer: Arc::clone(&sample_buffer),
            batch_frames_count,
        })
        .open_stream()
        .map_err(err!())?;

    stream.start().map_err(err!())?;

    alvr_audio::receive_samples_loop(
        running,
        receiver,
        sample_buffer,
        2,
        batch_frames_count,
        average_buffer_frames_count,
    )
    .ok();

    // Note: Oboe crahes if stream.stop() is NOT called on AudioPlayer
    stream.stop_with_timeout(0).ok();

    Ok(())
}
