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
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as smpsc, Arc,
    },
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

pub async fn record_audio_loop(sample_rate: u32, sender: StreamSender<()>) -> StrResult {
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

// Render a fade-out. It is aware of a in-progress fade-in
// frame_buffer must have enough frames
fn add_fade_out(
    fade_outs: &mut VecDeque<(f32, f32)>,
    frame_buffer: &VecDeque<(f32, f32)>,
    fade_in_progress: &mut Option<usize>,
    fade_frames_count: usize,
) {
    // fade_in_progress is set to None regardless
    let progress_start = if let Some(progress) = fade_in_progress.take() {
        fade_frames_count - progress
    } else {
        0
    };

    for idx in 0..fade_frames_count - progress_start {
        let volume =
            (PI * (idx + progress_start) as f32 / fade_frames_count as f32).cos() / 2. + 0.5;

        if idx < fade_outs.len() {
            fade_outs[idx].0 += frame_buffer[idx].0 * volume;
            fade_outs[idx].1 += frame_buffer[idx].1 * volume;
        } else {
            fade_outs.push_back((frame_buffer[idx].0 * volume, frame_buffer[idx].1 * volume));
        }
    }
}

struct PlayerCallback {
    frame_buffer: Arc<Mutex<VecDeque<(f32, f32)>>>,

    // length of fade-in/out in frames
    fade_frames_count: usize,

    // Prerendered fade-outs. In case of intermittent packet loss, multiple fade-outs could overlap.
    // A separate buffer is used because the samples we need in frame_buffer could get removed
    // due to buffer overgrowth.
    fade_outs: Arc<Mutex<VecDeque<(f32, f32)>>>,

    // Fade-in progress
    fade_in_progress: Arc<Mutex<Option<usize>>>,

    // Interrupted state starts from the beginning of a fade-out and ends at the start of a fade-in
    interrupted: Arc<AtomicBool>,
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
        let mut fade_in_progress_ref = self.fade_in_progress.lock();

        if frame_buffer_ref.len() >= out_frames.len() + self.fade_frames_count {
            if self.interrupted.load(Ordering::SeqCst) {
                self.interrupted.store(false, Ordering::SeqCst);

                *fade_in_progress_ref = Some(0);
            }

            let frames = frame_buffer_ref
                .drain(0..out_frames.len())
                .collect::<Vec<_>>();
            out_frames.copy_from_slice(&frames);

            if let Some(progress) = &mut *fade_in_progress_ref {
                for out_frame in out_frames.iter_mut() {
                    let volume =
                        (PI * *progress as f32 / self.fade_frames_count as f32).cos() / -2. + 0.5;
                    out_frame.0 *= volume;
                    out_frame.1 *= volume;

                    if *progress < self.fade_frames_count {
                        *progress += 1;
                    } else {
                        *fade_in_progress_ref = None;
                        break;
                    }
                }
            }
        } else {
            error!("audio buffer too small! size: {}", frame_buffer_ref.len());

            // Clear buffer. Previous audio samples don't get cleared automatically and they cause
            // buzzing
            out_frames.fill((0., 0.));

            if !self.interrupted.load(Ordering::SeqCst) {
                self.interrupted.store(true, Ordering::SeqCst);

                add_fade_out(
                    &mut *fade_outs_ref,
                    &*frame_buffer_ref,
                    &mut fade_in_progress_ref,
                    self.fade_frames_count,
                );
            }
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

    let frame_buffer = Arc::new(Mutex::new(VecDeque::from(vec![
        (0., 0.);
        fade_frames_count
    ])));
    let fade_outs = Arc::new(Mutex::new(VecDeque::new()));
    let fade_in_progress = Arc::new(Mutex::new(None));
    let interrupted = Arc::new(AtomicBool::new(false));
    thread::spawn({
        let frame_buffer = frame_buffer.clone();
        let fade_outs = fade_outs.clone();
        let fade_in_progress = fade_in_progress.clone();
        let interrupted = interrupted.clone();
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
                    fade_in_progress,
                    interrupted,
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
        let (_, data) = receiver.recv_buffer().await?;
        let frames = data
            .chunks_exact(4)
            .map(|c| {
                (
                    i16::from_ne_bytes([c[0], c[1]]).to_f32(),
                    i16::from_ne_bytes([c[2], c[3]]).to_f32(),
                )
            })
            .collect::<Vec<_>>();

        let mut frame_buffer_ref = frame_buffer.lock();
        frame_buffer_ref.extend(&frames);

        // todo: use smarter policy with EventTiming
        let buffer_size = frame_buffer_ref.len();
        if buffer_size > 2 * min_buffer_frames_count + fade_frames_count {
            error!("draining audio buffer. size: {}", buffer_size);

            // Add a fade-out before draining frame_buffer
            add_fade_out(
                &mut *fade_outs.lock(),
                &*frame_buffer_ref,
                &mut *fade_in_progress.lock(),
                fade_frames_count,
            );

            // Trigger a fade-in
            interrupted.store(true, Ordering::SeqCst);

            // Drain frame_buffer
            frame_buffer_ref.drain(0..(buffer_size - min_buffer_frames_count - fade_frames_count));
        }
    }
}
