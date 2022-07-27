use crate::{
    IS_RESUMED, AUDIO_INPUT_DEVICE, AUDIO_INPUT_UPDATED, AUDIO_OUTPUT_DEVICE, AUDIO_OUTPUT_UPDATED,
};
use alvr_audio::AudioDevice;
use alvr_common::{parking_lot::Mutex, prelude::*};
use alvr_session::AudioBufferingConfig;
use alvr_sockets::{StreamReceiver, StreamSender};
use oboe::{
    AudioInputCallback, AudioInputStreamSafe, AudioOutputCallback, AudioOutputStreamSafe,
    AudioStream, AudioStreamBuilder, DataCallbackResult, InputPreset, Mono, PerformanceMode,
    SampleRateConversionQuality, Stereo, Usage, Error,
};
use std::{
    collections::VecDeque,
    mem,
    sync::{mpsc as smpsc, Arc},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};
use tokio::sync::mpsc as tmpsc;

struct RecorderCallback {
    sender: tmpsc::UnboundedSender<Vec<u8>>,
    shutdown_notifier: smpsc::Sender<()>,
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

        self.sender.send(sample_buffer).ok();

        if !AUDIO_INPUT_UPDATED.load(Ordering::Relaxed) {
            DataCallbackResult::Continue
        }else{
            self.shutdown_notifier.send(()).unwrap();
            DataCallbackResult::Stop
        }
    }
    
    fn on_error_after_close(
        &mut self,
        _audio_stream: &mut dyn AudioInputStreamSafe,
        error: Error
    ) {
        if error != Error::Disconnected {
            info!("AudioInputCallback::on_error_after_close with error {}", error);
        }
        self.shutdown_notifier.send(()).unwrap();
    }
}

#[allow(unused_variables)]
pub async fn record_audio_loop(
    device: AudioDevice,
    channels_count: u16,
    mute: bool,
    mut sender: StreamSender<()>,
) -> StrResult {
    let sample_rate = device.input_sample_rate()?;

    let (shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();

    // make sure the following loop will be stopped once this function is finished.
    let is_finished = Arc::new(AtomicBool::new(false));

    thread::spawn({
        let is_finished = Arc::clone(&is_finished);
        move || loop {
            let device_id = AUDIO_INPUT_DEVICE.load(Ordering::Relaxed);
            // Reset the AUDIO_INTPUT_UPDATED flag, as it has been handled.
            AUDIO_INPUT_UPDATED.store(false, Ordering::Relaxed);
            info!("record_audio_loop stream with device_id {device_id}");

            let mut audio_stream_builder = AudioStreamBuilder::default()
                .set_shared()
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sample_rate(sample_rate as _)
                .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
                .set_mono()
                .set_i16()
                .set_input()
                .set_usage(Usage::Game)
                .set_input_preset(InputPreset::VoicePerformance);
            if device_id > 0 {
                audio_stream_builder = audio_stream_builder.set_device_id(device_id);
            }
            let stream = audio_stream_builder
                .set_callback(RecorderCallback {
                    sender: data_sender.clone(),
                    shutdown_notifier: shutdown_notifier.clone(),
                })
                .open_stream()
                .map_err(err!());

            match stream {
                Err(e) => info!("record_audio_loop audio_stream_builder.open_stream() failed {e}"),
                Ok(mut stream) => match stream.start().map_err(err!()) {
                    Err(e) => info!("record_audio_loop stream.start() failed {e}"),
                    Ok(_) => {
                        shutdown_receiver.recv().ok();

                        // This call gets stuck if the headset goes to sleep, but finishes when the headset wakes up
                        stream.stop_with_timeout(0).ok();
                    },
                },
            }
            
            if is_finished.load(Ordering::Relaxed) || !IS_RESUMED.value() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    while let Some(data) = data_receiver.recv().await {
        let mut buffer = sender.new_buffer(&(), data.len())?;
        buffer.get_mut().extend(data);
        sender.send_buffer(buffer).await.ok();
    }
    is_finished.store(true, Ordering::Relaxed);
    Ok(())
}

struct PlayerCallback {
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    batch_frames_count: usize,
    shutdown_notifier: smpsc::Sender<()>,
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

        if !AUDIO_OUTPUT_UPDATED.load(Ordering::Relaxed) {
            DataCallbackResult::Continue
        }else{
            self.shutdown_notifier.send(()).unwrap();
            DataCallbackResult::Stop
        }
    }

    fn on_error_after_close(
        &mut self,
        _audio_stream: &mut dyn AudioOutputStreamSafe,
        error: Error
    ) {
        if error != Error::Disconnected {
            info!("AudioOutputCallback::on_error_after_close with error {}", error);
        }
        self.shutdown_notifier.send(()).unwrap();
    }
}

#[allow(unused_variables)]
pub async fn play_audio_loop(
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

    // make sure the following loop will be stopped once this function is finished.
    let is_finished = Arc::new(AtomicBool::new(false));

    // store the stream in a thread (because !Send) and extract the playback handle
    let (shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    thread::spawn({
        let sample_buffer = Arc::clone(&sample_buffer);
        let is_finished = Arc::clone(&is_finished);
        move || loop {
            // If there is a newly plugged headphone, use it.
            let device_id = AUDIO_OUTPUT_DEVICE.load(Ordering::Relaxed);
            // Reset the AUDIO_OUTPUT_UPDATED flag, as it is handled now.
            AUDIO_OUTPUT_UPDATED.store(false, Ordering::Relaxed);
            info!("play_audio_loop stream with device_id {device_id}");

            let mut audio_stream_builder = AudioStreamBuilder::default()
                .set_shared()
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sample_rate(sample_rate as _)
                .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
                .set_stereo()
                .set_f32()
                .set_frames_per_callback(batch_frames_count as _)
                .set_output()
                .set_usage(Usage::Game);
            if device_id > 0 {
                audio_stream_builder = audio_stream_builder.set_device_id(device_id);
            }
            let stream = audio_stream_builder
                .set_callback(PlayerCallback {
                    sample_buffer: sample_buffer.clone(),
                    batch_frames_count,
                    shutdown_notifier: shutdown_notifier.clone(),
                })
                .open_stream()
                .map_err(err!());

            match stream {
                Err(e) => info!("play_audio_loop audio_stream_builder.open_stream() failed {e}"),
                Ok(mut stream) => match stream.start().map_err(err!()) {
                    Err(e) => info!("play_audio_loop stream.start() failed {e}"),
                    Ok(_) => {
                        shutdown_receiver.recv().ok();

                        // Note: Oboe crahes if stream.stop() is NOT called on AudioPlayer
                        stream.stop_with_timeout(0).ok();
                    },
                }
            }

            // keep reopening audio stream if ALVR is still running.
            if is_finished.load(Ordering::Relaxed) || !IS_RESUMED.value() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    let res = alvr_audio::receive_samples_loop(
        receiver,
        sample_buffer,
        2,
        batch_frames_count,
        average_buffer_frames_count,
    )
    .await;
    is_finished.store(true, Ordering::Relaxed);
    res
}
