#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(windows)]
pub use crate::windows::*;

use alvr_common::{
    ConnectionError, ToAny,
    anyhow::{self, Context, Result, bail},
    info,
    parking_lot::Mutex,
};
use alvr_session::{AudioBufferingConfig, CustomAudioDeviceConfig, MicrophoneDevicesConfig};
use alvr_sockets::{StreamReceiver, StreamSender};
use cpal::{
    BufferSize, Host, Sample, SampleFormat, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rodio::{OutputStreamBuilder, Source};
use std::{collections::VecDeque, sync::Arc, thread, time::Duration};

pub use cpal::Device;

fn device_from_custom_config(
    host: &Host,
    config: &CustomAudioDeviceConfig,
    is_output: bool,
) -> Result<Device> {
    let mut devices = if is_output {
        host.output_devices()?
    } else {
        host.input_devices()?
    };

    Ok(match config {
        CustomAudioDeviceConfig::NameSubstring(name_substring) => devices
            .find(|d| {
                d.name()
                    .map(|name| name.to_lowercase().contains(&name_substring.to_lowercase()))
                    .unwrap_or(false)
            })
            .with_context(|| {
                format!("Cannot find audio device which name contains \"{name_substring}\"")
            })?,
        CustomAudioDeviceConfig::Index(index) => devices
            .nth(*index)
            .with_context(|| format!("Cannot find audio device at index {index}"))?,
    })
}

pub fn new_output(config: Option<&CustomAudioDeviceConfig>) -> Result<Device> {
    let host = cpal::default_host();

    let device = match config {
        None => host
            .default_output_device()
            .context("No output audio device found")?,
        Some(config) => device_from_custom_config(&host, config, true)?,
    };

    Ok(device)
}

pub fn new_input(config: Option<CustomAudioDeviceConfig>) -> Result<Device> {
    let host = cpal::default_host();

    let device = match config {
        None => host
            .default_input_device()
            .context("No input audio device found")?,
        Some(config) => device_from_custom_config(&host, &config, false)?,
    };

    Ok(device)
}

// returns (sink, source)
pub fn new_virtual_microphone_pair(config: MicrophoneDevicesConfig) -> Result<(Device, Device)> {
    // No-op on Windows (this is windows specific code)
    let host = cpal::default_host();

    let (sink_name, source_name) = match config {
        MicrophoneDevicesConfig::Automatic => {
            // NOTE: This will iterate over all devices for every option it tries, if the audio
            // code is slow, change that first
            return [
                MicrophoneDevicesConfig::VAC,
                MicrophoneDevicesConfig::VBCable,
                MicrophoneDevicesConfig::VoiceMeeter,
                MicrophoneDevicesConfig::VoiceMeeterAux,
                MicrophoneDevicesConfig::VoiceMeeterVaio3,
            ]
            .into_iter()
            .find_map(|cable_type| new_virtual_microphone_pair(cable_type).ok())
            .context("No microphones found");
        }

        MicrophoneDevicesConfig::VAC => ("Line 1", "Line 1"),
        MicrophoneDevicesConfig::VBCable => ("CABLE Input", "CABLE Output"),
        MicrophoneDevicesConfig::VoiceMeeter => ("VoiceMeeter Input", "VoiceMeeter Output"),
        MicrophoneDevicesConfig::VoiceMeeterAux => {
            ("VoiceMeeter Aux Input", "VoiceMeeter Aux Output")
        }
        MicrophoneDevicesConfig::VoiceMeeterVaio3 => {
            ("VoiceMeeter VAIO3 Input", "VoiceMeeter VAIO3 Output")
        }

        MicrophoneDevicesConfig::Custom { sink, source } => {
            return Ok((
                device_from_custom_config(&host, &sink, true)?,
                device_from_custom_config(&host, &source, false)?,
            ));
        }
    };

    let sink = host
            .output_devices()?
            .find(|d| d.name().unwrap_or_default().contains(sink_name))
            .context("Virtual Audio Cable, VB-CABLE or VoiceMeeter not found. Please install or reinstall one")?;

    let source = host
        .input_devices()?
        .find(|d| d.name().unwrap_or_default().contains(source_name))
        .context("Matching output microphone not found. Did you rename it?")?;

    Ok((sink, source))
}

pub fn input_sample_rate(dev: &Device) -> Result<u32> {
    let config = dev
        .default_input_config()
        // On Windows, loopback devices are not recognized as input devices. Use output config.
        .or_else(|_| dev.default_output_config())?;

    Ok(config.sample_rate().0)
}

pub enum AudioRecordState {
    Recording,
    ShouldStop,
    Err(Option<anyhow::Error>),
}

pub enum AudioChannel {
    FrontLeft,
    FrontRight,
    Center,
    SurroundLeft,
    SurroundRight,
    BackLeft,
    BackRight,
    Top,
    HighFrontLeft,
    HighFrontRight,
    HighFrontCenter,
    HighBackLeft,
    HighBackRight,
    LowFrequency,
}

fn downmix_channels(channels: &[AudioChannel], data: &[u8], out_channels: u16) -> Vec<u8> {
    let mut left = 0.0;
    let mut right = 0.0;

    for i in 0..channels.len() {
        let [l, r] = match &channels[i] {
            AudioChannel::FrontLeft => [1.0, 0.0],
            AudioChannel::FrontRight => [0.0, 1.0],
            AudioChannel::Center => [0.707, 0.707],
            AudioChannel::SurroundLeft => [0.707, 0.0],
            AudioChannel::SurroundRight => [0.0, 0.707],
            AudioChannel::BackLeft => [0.707, 0.0],
            AudioChannel::BackRight => [0.0, 0.707],
            AudioChannel::Top => [0.577, 0.577],
            AudioChannel::HighFrontLeft => [0.707, 0.0],
            AudioChannel::HighFrontRight => [0.0, 0.707],
            AudioChannel::HighFrontCenter => [0.5, 0.5],
            AudioChannel::HighBackLeft => [0.5, 0.0],
            AudioChannel::HighBackRight => [0.0, 0.5],
            _ => [0.0, 0.0],
        };
        let val = i16::from_ne_bytes([data[i * 2], data[i * 2 + 1]]).to_sample::<f32>();
        left += val * l;
        right += val * r;
    }

    if out_channels == 1 {
        let bytes = ((left + right) / 2.0).to_sample::<i16>().to_ne_bytes();
        vec![bytes[0], bytes[1]]
    } else {
        let left_bytes = left.to_sample::<i16>().to_ne_bytes();
        let right_bytes = right.to_sample::<i16>().to_ne_bytes();
        vec![left_bytes[0], left_bytes[1], right_bytes[0], right_bytes[1]]
    }
}

fn downmix_audio(data: Vec<u8>, in_channels: u16, out_channels: u16) -> Vec<u8> {
    if in_channels == out_channels {
        data
    } else if in_channels == 1 && out_channels == 2 {
        data.chunks_exact(2)
            .flat_map(|c| vec![c[0], c[1], c[0], c[1]])
            .collect()
    } else {
        let channels = match in_channels {
            2 => vec![AudioChannel::FrontLeft, AudioChannel::FrontRight],
            3 => vec![
                AudioChannel::FrontLeft,
                AudioChannel::FrontRight,
                AudioChannel::LowFrequency,
            ],
            4 => vec![
                AudioChannel::FrontLeft,
                AudioChannel::FrontRight,
                AudioChannel::BackLeft,
                AudioChannel::BackRight,
            ],
            6 => vec![
                AudioChannel::FrontRight,
                AudioChannel::Center,
                AudioChannel::LowFrequency,
                AudioChannel::SurroundLeft, // Sometimes actually BackLeft, has same level so it's okay
                AudioChannel::SurroundRight, // Sometimes actually BackRight, has same level so it's okay
            ],
            8 => vec![
                AudioChannel::FrontLeft,
                AudioChannel::FrontRight,
                AudioChannel::Center,
                AudioChannel::LowFrequency,
                AudioChannel::BackLeft,
                AudioChannel::BackRight,
                AudioChannel::SurroundLeft,
                AudioChannel::SurroundRight,
            ],
            _ => unreachable!("Invalid input channel count"),
        };

        data.chunks_exact(in_channels as usize * 2)
            .flat_map(|c| downmix_channels(&channels, c, out_channels))
            .collect()
    }
}

#[allow(unused_variables)]
pub fn record_audio_blocking(
    is_running: Arc<dyn Fn() -> bool + Send + Sync>,
    mut sender: StreamSender<()>,
    device: &Device,
    channels_count: u16,
    mute: bool,
) -> Result<()> {
    let config = device
        .default_input_config()
        // On Windows, loopback devices are not recognized as input devices. Use output config.
        .or_else(|_| device.default_output_config())?;

    if config.channels() > 8 {
        bail!(
            "Audio devices with more than 8 channels are not supported. {}",
            "Please turn off surround audio."
        );
    } else if config.channels() == 5 || config.channels() == 7 {
        bail!(
            "Audio devices with {} channels are not supported.",
            config.channels()
        );
    }

    let stream_config = StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: BufferSize::Default,
    };

    let state = Arc::new(Mutex::new(AudioRecordState::Recording));

    let stream = device.build_input_stream_raw(
        &stream_config,
        config.sample_format(),
        {
            let state = Arc::clone(&state);
            let is_running = Arc::clone(&is_running);
            move |data, _| {
                let data = if config.sample_format() == SampleFormat::F32 {
                    data.bytes()
                        .chunks_exact(4)
                        .flat_map(|b| {
                            f32::from_ne_bytes([b[0], b[1], b[2], b[3]])
                                .to_sample::<i16>()
                                .to_ne_bytes()
                                .to_vec()
                        })
                        .collect()
                } else {
                    data.bytes().to_vec()
                };

                let data = downmix_audio(data, config.channels(), channels_count);

                if is_running() {
                    sender.send_header_with_payload(&(), &data).ok();
                } else {
                    *state.lock() = AudioRecordState::ShouldStop;
                }
            }
        },
        {
            let state = Arc::clone(&state);
            move |e| *state.lock() = AudioRecordState::Err(Some(e.into()))
        },
        None,
    )?;

    #[cfg(windows)]
    if mute && device.supports_output() {
        crate::windows::set_mute_windows_device(device, true).ok();
    }

    let mut res = stream.play().to_any();

    if res.is_ok() {
        while matches!(*state.lock(), AudioRecordState::Recording) && is_running() {
            thread::sleep(Duration::from_millis(500))
        }

        if let AudioRecordState::Err(e) = &mut *state.lock() {
            res = Err(e.take().unwrap());
        }
    }

    #[cfg(windows)]
    if mute && device.supports_output() {
        set_mute_windows_device(device, false).ok();
    }

    res
}

// Audio callback. This is designed to be as less complex as possible. Still, when needed, this
// callback can render a fade-out autonomously.
#[inline]
pub fn get_next_frame_batch(
    sample_buffer: &mut VecDeque<f32>,
    channels_count: usize,
    batch_frames_count: usize,
) -> Vec<f32> {
    if sample_buffer.len() / channels_count >= batch_frames_count {
        let mut batch = sample_buffer
            .drain(0..batch_frames_count * channels_count)
            .collect::<Vec<_>>();

        if sample_buffer.len() / channels_count < batch_frames_count {
            // Render fade-out. It is completely contained in the current batch
            for f in 0..batch_frames_count {
                let volume = 1. - f as f32 / batch_frames_count as f32;
                for c in 0..channels_count {
                    batch[f * channels_count + c] *= volume;
                }
            }
        }
        // fade-ins and cross-fades are rendered in the receive loop directly inside sample_buffer.

        batch
    } else {
        vec![0.; batch_frames_count * channels_count]
    }
}

// The receive loop is resposible for ensuring smooth transitions in case of disruptions (buffer
// underflow, overflow, packet loss). In case the computation takes too much time, the audio
// callback will gracefully handle an interruption, and the callback timing and sound wave
// continuity will not be affected.
pub fn receive_samples_loop(
    is_running: impl Fn() -> bool,
    receiver: &mut StreamReceiver<()>,
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    channels_count: usize,
    batch_frames_count: usize,
    average_buffer_frames_count: usize,
) -> Result<()> {
    let mut recovery_sample_buffer = vec![];
    while is_running() {
        let data = match receiver.recv(Duration::from_millis(500)) {
            Ok(data) => data,
            Err(ConnectionError::TryAgain(_)) => continue,
            Err(ConnectionError::Other(e)) => return Err(e),
        };
        let (_, packet) = data.get()?;

        let new_samples = packet
            .chunks_exact(2)
            .map(|c| i16::from_ne_bytes([c[0], c[1]]).to_sample::<f32>())
            .collect::<Vec<_>>();

        let mut sample_buffer_ref = sample_buffer.lock();

        if data.had_packet_loss() {
            info!("Audio packet loss!");

            if sample_buffer_ref.len() / channels_count < batch_frames_count {
                sample_buffer_ref.clear();
            } else {
                // clear remaining samples
                sample_buffer_ref.drain(batch_frames_count * channels_count..);
            }

            recovery_sample_buffer.clear();
        }

        if sample_buffer_ref.len() / channels_count < batch_frames_count {
            recovery_sample_buffer.extend(sample_buffer_ref.drain(..));
        }

        if sample_buffer_ref.is_empty() || data.had_packet_loss() {
            recovery_sample_buffer.extend(&new_samples);

            if recovery_sample_buffer.len() / channels_count
                > average_buffer_frames_count + batch_frames_count
            {
                // Fade-in
                for f in 0..batch_frames_count {
                    let volume = f as f32 / batch_frames_count as f32;
                    for c in 0..channels_count {
                        recovery_sample_buffer[f * channels_count + c] *= volume;
                    }
                }

                if data.had_packet_loss()
                    && sample_buffer_ref.len() / channels_count == batch_frames_count
                {
                    // Add a fade-out to make a cross-fade.
                    for f in 0..batch_frames_count {
                        let volume = 1. - f as f32 / batch_frames_count as f32;
                        for c in 0..channels_count {
                            recovery_sample_buffer[f * channels_count + c] +=
                                sample_buffer_ref[f * channels_count + c] * volume;
                        }
                    }

                    sample_buffer_ref.clear();
                }

                sample_buffer_ref.extend(recovery_sample_buffer.drain(..));
                info!("Audio recovered");
            }
        } else {
            sample_buffer_ref.extend(&new_samples);
        }

        // todo: use smarter policy with EventTiming
        let buffer_frames_size = sample_buffer_ref.len() / channels_count;
        if buffer_frames_size > 2 * average_buffer_frames_count + batch_frames_count {
            info!("Audio buffer overflow! size: {buffer_frames_size}");

            let drained_samples = sample_buffer_ref
                .drain(0..(buffer_frames_size - average_buffer_frames_count) * channels_count)
                .collect::<Vec<_>>();

            // Render a cross-fade.
            for f in 0..batch_frames_count {
                let volume = f as f32 / batch_frames_count as f32;
                for c in 0..channels_count {
                    let index = f * channels_count + c;
                    sample_buffer_ref[index] = sample_buffer_ref[index]
                        .mul_add(volume, drained_samples[index] * (1. - volume));
                }
            }
        }
    }

    Ok(())
}

struct StreamingSource {
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    current_batch: Vec<f32>,
    current_batch_cursor: usize,
    channels_count: usize,
    sample_rate: u32,
    batch_frames_count: usize,
}

impl Source for StreamingSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels_count as _
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Iterator for StreamingSource {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        if self.current_batch_cursor == 0 {
            self.current_batch = get_next_frame_batch(
                &mut self.sample_buffer.lock(),
                self.channels_count,
                self.batch_frames_count,
            );
        }

        let sample = self.current_batch[self.current_batch_cursor];

        self.current_batch_cursor =
            (self.current_batch_cursor + 1) % (self.batch_frames_count * self.channels_count);

        Some(sample)
    }
}

pub fn play_audio_loop(
    is_running: impl Fn() -> bool,
    device: &Device,
    channels_count: u16,
    sample_rate: u32,
    config: AudioBufferingConfig,
    receiver: &mut StreamReceiver<()>,
) -> Result<()> {
    // Size of a chunk of frames. It corresponds to the duration if a fade-in/out in frames.
    let batch_frames_count = sample_rate as usize * config.batch_ms as usize / 1000;

    // Average buffer size in frames
    let average_buffer_frames_count =
        sample_rate as usize * config.average_buffering_ms as usize / 1000;

    let sample_buffer = Arc::new(Mutex::new(VecDeque::new()));

    let stream = OutputStreamBuilder::from_device(device.clone())?.open_stream()?;

    stream.mixer().add(StreamingSource {
        sample_buffer: Arc::clone(&sample_buffer),
        current_batch: vec![],
        current_batch_cursor: 0,
        channels_count: channels_count as _,
        sample_rate,
        batch_frames_count,
    });

    receive_samples_loop(
        is_running,
        receiver,
        sample_buffer,
        channels_count as _,
        batch_frames_count,
        average_buffer_frames_count,
    )
    .ok();

    Ok(())
}
