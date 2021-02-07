use alvr_common::{
    sockets::{StreamReceiver, StreamSender},
    *,
};
use bytes::BytesMut;
use oboe::*;
use std::{collections::VecDeque, mem, sync::mpsc as smpsc, thread};
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
        sender.send_buffer(buffer).await?;
    }

    Ok(())
}

const OUTPUT_FRAME_SIZE: usize = 2 * mem::size_of::<i16>();

struct PlayerCallback {
    receiver: smpsc::Receiver<BytesMut>,
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
pub async fn play_audio_loop(
    sample_rate: u32,
    buffer_range_multiplier: u64,
    mut receiver: StreamReceiver<()>,
) -> StrResult {
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<BytesMut>();
    let (data_sender, data_receiver) = smpsc::channel();

    thread::spawn(move || -> StrResult {
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
                receiver: data_receiver,
                sample_buffer: VecDeque::new(),
                buffer_range_multiplier: buffer_range_multiplier as _,
                last_input_buffer_size: 0,
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        shutdown_receiver.recv().ok();

        // Note: Oboe crahes if stream.stop() is NOT called on AudioPlayer
        stream.stop_with_timeout(0).ok();

        Ok(())
    });

    loop {
        let (_, data) = receiver.recv_buffer().await?;
        trace_err!(data_sender.send(data))?;
    }
}
