use alvr_common::{
    data::AudioConfig,
    sockets::{StreamReceiver, StreamSender},
    *,
};
use bytes::BytesMut;
use cpal::Sample;
use oboe::*;
use parking_lot::Mutex;
use std::{
    cmp,
    collections::VecDeque,
    f32::consts::PI,
    mem,
    sync::{mpsc as smpsc, Arc},
    thread,
};
use tokio::sync::mpsc as tmpsc;

struct RecorderCallback {
    sender: tmpsc::UnboundedSender<Vec<u8>>,
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

        self.sender.send(sample_buffer.clone()).ok();

        DataCallbackResult::Continue
    }
}

pub async fn record_audio_loop(sample_rate: u32, mut sender: StreamSender<()>) -> StrResult {
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();

    thread::spawn(move || -> StrResult {
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
                sender: data_sender
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        shutdown_receiver.recv().ok();

        // This call gets stuck if the headset goes to sleep, but finishes when the headset wakes up
        stream.stop_with_timeout(0).ok();

        Ok(())
    });

    while let Some(data) = data_receiver.recv().await {
        let mut buffer = sender.new_buffer(&(), data.len())?;
        buffer.get_mut().extend(data);
        sender.send_buffer(buffer).await.ok();
    }

    Ok(())
}

// Render a fade-out. It is aware of a in-progress fade-in. This is done only if play_state is not
// already in FadeOutOrPaused and if frame_buffer has enough frames.
fn maybe_add_fade_out(
    fade_outs: &mut VecDeque<(f32, f32)>,
    frame_buffer: &VecDeque<(f32, f32)>,
    play_state: &mut PlayState,
    fade_frames_count: usize,
) {
    if frame_buffer.len() >= fade_frames_count {
        if let PlayState::FadeInOrResumed { fade_in_progress } = *play_state {
            let fade_out_start = fade_frames_count - fade_in_progress;

            for idx in 0..fade_frames_count - fade_out_start {
                let volume = (PI * (idx + fade_out_start) as f32 / fade_frames_count as f32).cos()
                    / 2.
                    + 0.5;

                if idx < fade_outs.len() {
                    fade_outs[idx].0 += frame_buffer[idx].0 * volume;
                    fade_outs[idx].1 += frame_buffer[idx].1 * volume;
                } else {
                    fade_outs
                        .push_back((frame_buffer[idx].0 * volume, frame_buffer[idx].1 * volume));
                }
            }
        }

        *play_state = PlayState::FadeOutOrPaused;
    }
}

enum PlayState {
    FadeOutOrPaused,
    FadeInOrResumed { fade_in_progress: usize },
}

struct PlayerCallback {
    frame_buffer: Arc<Mutex<VecDeque<(f32, f32)>>>,

    // length of fade-in/out in frames
    fade_frames_count: usize,

    // Prerendered fade-outs. In case of intermittent packet loss, multiple fade-outs could overlap.
    // A separate buffer is used because the samples we need in frame_buffer could get removed
    // due to buffer overgrowth.
    fade_outs: Arc<Mutex<VecDeque<(f32, f32)>>>,

    play_state: Arc<Mutex<PlayState>>,
}

impl AudioOutputCallback for PlayerCallback {
    type FrameType = (f32, Stereo);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioOutputStreamSafe,
        out_frames: &mut [(f32, f32)],
    ) -> DataCallbackResult {
        let mut frame_buffer_ref = self.frame_buffer.lock();
        let mut fade_outs_ref = self.fade_outs.lock();
        let mut play_state_ref = self.play_state.lock();

        if frame_buffer_ref.len() >= out_frames.len() + self.fade_frames_count {
            let mut fade_in_progress = match *play_state_ref {
                PlayState::FadeInOrResumed { fade_in_progress } => fade_in_progress,
                PlayState::FadeOutOrPaused => 0,
            };

            let frames = frame_buffer_ref
                .drain(0..out_frames.len())
                .collect::<Vec<_>>();
            out_frames.copy_from_slice(&frames);

            if fade_in_progress < self.fade_frames_count {
                for out_frame in out_frames.iter_mut() {
                    let volume =
                        (PI * fade_in_progress as f32 / self.fade_frames_count as f32).cos() / -2.
                            + 0.5;
                    out_frame.0 *= volume;
                    out_frame.1 *= volume;

                    fade_in_progress += 1;
                    if fade_in_progress == self.fade_frames_count {
                        break;
                    }
                }

                *play_state_ref = PlayState::FadeInOrResumed { fade_in_progress };
            }
        } else {
            error!("Audio buffer underflow! size: {}", frame_buffer_ref.len());

            // Clear buffer. Previous audio samples don't get cleared automatically and they cause
            // buzzing
            out_frames.fill((0., 0.));

            maybe_add_fade_out(
                &mut *fade_outs_ref,
                &*frame_buffer_ref,
                &mut play_state_ref,
                self.fade_frames_count,
            );
        }

        // drain fade-outs into the output frame buffer
        let drained_frames_count = cmp::min(out_frames.len(), fade_outs_ref.len());
        let mut drained_frames = fade_outs_ref.drain(0..drained_frames_count);
        for out_frame in out_frames.iter_mut() {
            if let Some(frame) = drained_frames.next() {
                out_frame.0 += frame.0;
                out_frame.1 += frame.1;
            }
        }

        DataCallbackResult::Continue
    }
}
pub async fn play_audio_loop(
    sample_rate: u32,
    config: AudioConfig,
    mut receiver: StreamReceiver<()>,
) -> StrResult {
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<BytesMut>();

    let fade_frames_count = sample_rate as usize * config.fade_ms as usize / 1000;
    let min_buffer_frames_count = sample_rate as usize * config.min_buffering_ms as usize / 1000;

    let frame_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let fade_outs = Arc::new(Mutex::new(VecDeque::new()));
    let play_state = Arc::new(Mutex::new(PlayState::FadeOutOrPaused));
    thread::spawn({
        let frame_buffer = frame_buffer.clone();
        let fade_outs = fade_outs.clone();
        let play_state = play_state.clone();
        move || -> StrResult {
            let mut stream = trace_err!(AudioStreamBuilder::default()
                .set_shared()
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sample_rate(sample_rate as _)
                .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
                .set_stereo()
                .set_f32()
                .set_output()
                .set_usage(Usage::Game)
                .set_callback(PlayerCallback {
                    frame_buffer,
                    fade_frames_count,
                    fade_outs,
                    play_state,
                })
                .open_stream())?;

            trace_err!(stream.start())?;

            shutdown_receiver.recv().ok();

            // Note: Oboe crahes if stream.stop() is NOT called on AudioPlayer
            stream.stop_with_timeout(0).ok();

            Ok(())
        }
    });

    loop {
        let packet = receiver.recv().await?;
        let frames = packet
            .buffer
            .chunks_exact(4)
            .map(|c| {
                (
                    i16::from_ne_bytes([c[0], c[1]]).to_f32(),
                    i16::from_ne_bytes([c[2], c[3]]).to_f32(),
                )
            })
            .collect::<Vec<_>>();

        // This is the first object that should be locked. The same is done on the audio callback.
        // This ensures there are no deadlocks between multiple mutexes.
        let mut frame_buffer_ref = frame_buffer.lock();

        if packet.had_packet_loss {
            error!("Audio packet loss detected! Clearing audio buffer");

            // Add a fade-out *before* draining frame_buffer
            maybe_add_fade_out(
                &mut *fade_outs.lock(),
                &*frame_buffer_ref,
                &mut *play_state.lock(),
                fade_frames_count,
            );

            // frame_buffer must be drained completely. There is no way of reusing the old frames
            // without discontinuity.
            frame_buffer_ref.clear();
        }

        frame_buffer_ref.extend(&frames);

        // todo: use smarter policy with EventTiming
        let buffer_size = frame_buffer_ref.len();
        if buffer_size > 2 * min_buffer_frames_count + fade_frames_count {
            error!("Audio buffer overflow! size: {}", buffer_size);

            // Add a fade-out *before* draining frame_buffer
            maybe_add_fade_out(
                &mut *fade_outs.lock(),
                &*frame_buffer_ref,
                &mut *play_state.lock(),
                fade_frames_count,
            );

            // Drain frame_buffer partially. A discontinuity is formed but the playback can resume
            // immediately with a fade-in
            frame_buffer_ref.drain(0..(buffer_size - min_buffer_frames_count - fade_frames_count));
        }
    }
}
