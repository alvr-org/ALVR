use alvr_common::{
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

struct PlayerCallback {
    // bucket where frames are normally read from
    frame_buffer: Arc<Mutex<VecDeque<(f32, f32)>>>,

    // frames read in case frame_buffer has drained
    last_packet: Arc<Mutex<Option<Vec<(f32, f32)>>>>,

    // last played sample, used to restart the exponential if needed
    last_frame: (f32, f32),

    // no audio samples to play or recovering
    disrupted: Arc<AtomicBool>,
}

impl AudioOutputCallback for PlayerCallback {
    type FrameType = (f32, Stereo);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioOutputStreamSafe,
        out_frames: &mut [(f32, f32)],
    ) -> DataCallbackResult {
        let mut frame_buffer_ref = self.frame_buffer.lock();

        let got_samples = frame_buffer_ref.len() >= out_frames.len();
        if got_samples {
            let frames = frame_buffer_ref
                .drain(0..out_frames.len())
                .collect::<Vec<_>>();
            out_frames.copy_from_slice(&frames);
        } else {
            error!("audio buffer too small! size: {}", frame_buffer_ref.len());

            // Clear buffer. Previous audio samples don't get cleared automatically and they cause
            // buzzing
            out_frames.fill((0., 0.));

            self.disrupted.store(true, Ordering::Relaxed);
        }

        if self.disrupted.load(Ordering::Relaxed) {
            error!(
                "distupted! got_samples {}, last_frame {:?}",
                got_samples, self.last_frame
            );

            // steadily increasing volume
            if got_samples {
                error!("fade in");
                let frames_count = out_frames.len() as f32;
                for (i, (left, right)) in out_frames.iter_mut().enumerate() {
                    let multiplier = i as f32 / frames_count;
                    *left *= multiplier;
                    *right *= multiplier;
                }

                self.disrupted.store(false, Ordering::Relaxed);
            }

            if !got_samples || self.last_frame != (0., 0.) {
                if let Some(packet) = &*self.last_packet.lock() {
                    if packet.len() > out_frames.len() {
                        // find sample in the old packet that most closely matches the last played
                        // sample, making sure there will be enough samples to play after that
                        let maybe_index = packet[0..packet.len() - out_frames.len()]
                            .iter()
                            .enumerate()
                            .map(|(i, (l, r))| {
                                (
                                    i,
                                    (self.last_frame.0 - l).abs() + (self.last_frame.1 - r).abs(),
                                )
                            })
                            .min_by(|(_, dist1), (_, dist2)| {
                                // f32 does not implement Ord, so no cmp method
                                if dist1 < dist2 {
                                    cmp::Ordering::Less
                                } else {
                                    cmp::Ordering::Greater
                                }
                            })
                            .map(|(i, _)| i);

                        if let Some(idx) = maybe_index {
                            // Calculate volume scaling to perfectly match the choosen frame. Very
                            // often the resulting values will be close to 1.
                            // Note: volume scaling is preferred to a bias. DC bias is removed by
                            // the audio controller and will still cause a pop. This is why a
                            // decreasing exponential bias will not work.
                            let mut starting_frame = packet[idx];
                            if starting_frame.0.abs() < f32::EPSILON {
                                starting_frame.0 = f32::EPSILON;
                            }
                            if starting_frame.1.abs() < f32::EPSILON {
                                starting_frame.1 = f32::EPSILON;
                            }
                            let left_scale =
                                (self.last_frame.0 / starting_frame.0).clamp(-1.2, 1.2);
                            let right_scale = self.last_frame.1 / starting_frame.1.clamp(-1.2, 1.2);
                            // error!("starting_frame {:?}", starting_frame);
                            // error!("last_frame {:?}", self.last_frame);
                            error!("scale {} {}", left_scale, right_scale);

                            let frames_count = out_frames.len() as f32;
                            for (idx, (left, right)) in out_frames.iter_mut().enumerate() {
                                let multiplier = 1. - (idx + 1) as f32 / frames_count;
                                *left += packet[idx + 1].0 * left_scale * multiplier;
                                *right += packet[idx + 1].1 * right_scale * multiplier;

                                // The last iteration will leave "left" and "right" immutated. So
                                // the last frame will be (0., 0.) if !got_samples.
                            }
                        }
                    }
                }
            }
        }

        self.last_frame = *out_frames.last().unwrap();

        DataCallbackResult::Continue
    }
}
pub async fn play_audio_loop(
    sample_rate: u32,
    buffer_range_multiplier: u64,
    mut receiver: StreamReceiver<()>,
) -> StrResult {
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<BytesMut>();

    let frame_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let last_packet = Arc::new(Mutex::new(None));
    let disrupted = Arc::new(AtomicBool::new(false));
    thread::spawn({
        let frame_buffer = frame_buffer.clone();
        let last_packet = last_packet.clone();
        let disrupted = disrupted.clone();
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
                    last_packet,
                    last_frame: (0., 0.),
                    disrupted,
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
        *last_packet.lock() = Some(frames);

        // todo: use smarter policy with EventTiming
        if frame_buffer_ref.len() > 2 * buffer_range_multiplier as usize * data.len() {
            error!("draining audio buffer. size: {}", frame_buffer_ref.len());

            let buffer_size = frame_buffer_ref.len();
            frame_buffer_ref
                .drain(0..(buffer_size - buffer_range_multiplier as usize * data.len()));

            disrupted.store(true, Ordering::Relaxed);
        }
    }
}
