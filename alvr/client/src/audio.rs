use alvr_common::{
    sockets::{StreamReceiver, StreamSender},
    *,
};
use bytes::BytesMut;
use cpal::Sample;
use oboe::*;
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
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

// A decreasing exponential used to reduce pop sound on packet loss. The pop sound is caused by a
// jump in the sample values (to zero). An exponential curve gradually lowers the value to zero. The
// exponential curve is the only curve (other than the trivial case y=k) that has no oscillatory
// part and it gets perceived as silence. Any other curve (including a steadily decreasing line) CAN
// be decomposed into sine waves, at least partially.
//
// Exponential multiplier: ratio between consecutive samples, used to construct the exponential.
const EXPONENIAL_MULTIPLIER: f32 = 0.9995;

struct PlayerCallback {
    sample_buffer: Arc<Mutex<VecDeque<(f32, f32)>>>,
    buffer_range_multiplier: usize,
    last_packet_samples_count: Arc<AtomicUsize>,

    // decreasing exponential multiplier OR 1 - sample volume multiplier
    multiplier: f32,

    // starting point to construct the decreasing exponential
    exponential_last_value: (f32, f32),

    // last played sample, used to restart the exponential if needed
    last_sample_value: (f32, f32),
}

impl AudioOutputCallback for PlayerCallback {
    type FrameType = (f32, Stereo);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioOutputStreamSafe,
        frames: &mut [(f32, f32)],
    ) -> DataCallbackResult {
        let mut sample_buffer_ref = self.sample_buffer.lock();

        // todo: use smarter policy with EventTiming
        let last_packet_samples_count = self.last_packet_samples_count.load(Ordering::Relaxed);
        if sample_buffer_ref.len() > 2 * self.buffer_range_multiplier * last_packet_samples_count {
            error!("draining audio buffer. size: {}", sample_buffer_ref.len());

            let buffer_size = sample_buffer_ref.len();
            sample_buffer_ref
                .drain(0..(buffer_size - self.buffer_range_multiplier * last_packet_samples_count));

            // Restart decreasing exponential
            self.multiplier = EXPONENIAL_MULTIPLIER;
            self.exponential_last_value = (
                self.last_sample_value.0 * EXPONENIAL_MULTIPLIER,
                self.last_sample_value.1 * EXPONENIAL_MULTIPLIER,
            );
        }

        let mut should_restart_exponential = false;
        if sample_buffer_ref.len() >= frames.len() {
            let buffer_it = sample_buffer_ref.drain(0..frames.len()).collect::<Vec<_>>();
            frames.copy_from_slice(&buffer_it);
        } else {
            error!("audio buffer too small! size: {}", sample_buffer_ref.len());

            // Restart decreasing exponential
            should_restart_exponential = true;
            self.exponential_last_value = (
                self.last_sample_value.0 * EXPONENIAL_MULTIPLIER,
                self.last_sample_value.1 * EXPONENIAL_MULTIPLIER,
            );

            // Clear buffer. Previous audio samples don't get cleared automatically and they cause
            // buzzing
            frames.fill((0., 0.));
        }

        // mix steadily decreasing exponential with samples with increasing volume (with a
        // saturating exponential)
        for (left, right) in frames.iter_mut() {
            *left = (self.exponential_last_value.0 + *left * (1. - self.multiplier)).clamp(-1., 1.);
            *right = self.exponential_last_value.1 + *right * (1. - self.multiplier).clamp(-1., 1.);

            self.multiplier *= EXPONENIAL_MULTIPLIER;
            self.exponential_last_value = (
                self.exponential_last_value.0 * EXPONENIAL_MULTIPLIER,
                self.exponential_last_value.1 * EXPONENIAL_MULTIPLIER,
            );
        }

        // if no samples were played, reset the multiplier
        if should_restart_exponential {
            self.multiplier = EXPONENIAL_MULTIPLIER;
        }

        self.last_sample_value = *frames.last().unwrap();

        DataCallbackResult::Continue
    }
}
pub async fn play_audio_loop(
    sample_rate: u32,
    buffer_range_multiplier: u64,
    mut receiver: StreamReceiver<()>,
) -> StrResult {
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<BytesMut>();

    let sample_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let last_packet_samples_count = Arc::new(AtomicUsize::new(0));
    thread::spawn({
        let sample_buffer = sample_buffer.clone();
        let last_packet_samples_count = last_packet_samples_count.clone();
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
                    sample_buffer,
                    buffer_range_multiplier: buffer_range_multiplier as _,
                    last_packet_samples_count,
                    multiplier: 1.,
                    exponential_last_value: (0., 0.),
                    last_sample_value: (0., 0.),
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
        let samples_it = data.chunks_exact(4).map(|c| {
            (
                i16::from_ne_bytes([c[0], c[1]]).to_f32(),
                i16::from_ne_bytes([c[2], c[3]]).to_f32(),
            )
        });

        last_packet_samples_count.store(samples_it.len(), Ordering::Relaxed);

        sample_buffer.lock().extend(samples_it);
    }
}
