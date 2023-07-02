use alvr_common::{once_cell::sync::Lazy, parking_lot::Mutex, prelude::*};
use alvr_session::{
    AudioBufferingConfig, CustomAudioDeviceConfig, LinuxAudioBackend, MicrophoneDevicesConfig,
};
use alvr_sockets::{ReceiverBuffer, StreamReceiver, StreamSender};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, Host, Sample, SampleFormat, StreamConfig,
};
use rodio::{OutputStream, Source};
use std::{
    collections::{HashMap, VecDeque},
    sync::{mpsc as smpsc, Arc},
    thread,
};
use tokio::sync::mpsc as tmpsc;

static VIRTUAL_MICROPHONE_PAIRS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    [
        ("CABLE Input", "CABLE Output"),
        ("VoiceMeeter Input", "VoiceMeeter Output"),
        ("VoiceMeeter Aux Input", "VoiceMeeter Aux Output"),
        ("VoiceMeeter VAIO3 Input", "VoiceMeeter VAIO3 Output"),
    ]
    .into_iter()
    .collect()
});

fn device_from_custom_config(host: &Host, config: &CustomAudioDeviceConfig) -> StrResult<Device> {
    Ok(match config {
        CustomAudioDeviceConfig::NameSubstring(name_substring) => host
            .devices()
            .map_err(err!())?
            .find(|d| {
                d.name()
                    .map(|name| name.to_lowercase().contains(&name_substring.to_lowercase()))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                format!("Cannot find audio device which name contains \"{name_substring}\"")
            })?,
        CustomAudioDeviceConfig::Index(index) => host
            .devices()
            .map_err(err!())?
            .nth(*index)
            .ok_or_else(|| format!("Cannot find audio device at index {index}"))?,
    })
}

fn microphone_pair_from_sink_name(host: &Host, sink_name: &str) -> StrResult<(Device, Device)> {
    let sink = host
        .output_devices()
        .map_err(err!())?
        .find(|d| d.name().unwrap_or_default().contains(sink_name))
        .ok_or_else(|| {
            "VB-CABLE or Voice Meeter not found. Please install or reinstall either one".to_owned()
        })?;

    if let Some(source_name) = VIRTUAL_MICROPHONE_PAIRS.get(sink_name) {
        Ok((
            sink,
            host.input_devices()
                .map_err(err!())?
                .find(|d| {
                    d.name()
                        .map(|name| name.contains(source_name))
                        .unwrap_or(false)
                })
                .ok_or_else(|| {
                    "Matching output microphone not found. Did you rename it?".to_owned()
                })?,
        ))
    } else {
        unreachable!("Invalid argument")
    }
}

#[allow(dead_code)]
pub struct AudioDevice {
    inner: Device,
    is_output: bool,
}

#[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
impl AudioDevice {
    pub fn new_output(
        linux_backend: Option<LinuxAudioBackend>,
        config: Option<&CustomAudioDeviceConfig>,
    ) -> StrResult<Self> {
        #[cfg(target_os = "linux")]
        let host = match linux_backend {
            Some(LinuxAudioBackend::Alsa) => cpal::host_from_id(cpal::HostId::Alsa).unwrap(),
            Some(LinuxAudioBackend::Jack) => cpal::host_from_id(cpal::HostId::Jack).unwrap(),
            None => cpal::default_host(),
        };
        #[cfg(not(target_os = "linux"))]
        let host = cpal::default_host();

        let device = match config {
            None => host
                .default_output_device()
                .ok_or_else(|| "No output audio device found".to_owned())?,
            Some(config) => device_from_custom_config(&host, config)?,
        };

        Ok(Self {
            inner: device,
            is_output: true,
        })
    }

    pub fn new_input(config: Option<CustomAudioDeviceConfig>) -> StrResult<Self> {
        let host = cpal::default_host();

        let device = match config {
            None => host
                .default_input_device()
                .ok_or_else(|| "No input audio device found".to_owned())?,
            Some(config) => device_from_custom_config(&host, &config)?,
        };

        Ok(Self {
            inner: device,
            is_output: false,
        })
    }

    // returns (sink, source)
    pub fn new_virtual_microphone_pair(
        linux_backend: Option<LinuxAudioBackend>,
        config: MicrophoneDevicesConfig,
    ) -> StrResult<(Self, Self)> {
        #[cfg(target_os = "linux")]
        let host = match linux_backend {
            Some(LinuxAudioBackend::Alsa) => cpal::host_from_id(cpal::HostId::Alsa).unwrap(),
            Some(LinuxAudioBackend::Jack) => cpal::host_from_id(cpal::HostId::Jack).unwrap(),
            None => cpal::default_host(),
        };
        #[cfg(not(target_os = "linux"))]
        let host = cpal::default_host();

        let (sink, source) = match config {
            MicrophoneDevicesConfig::Automatic => {
                let mut pair = Err(String::new());
                for sink_name in VIRTUAL_MICROPHONE_PAIRS.keys() {
                    pair = microphone_pair_from_sink_name(&host, sink_name);
                    if pair.is_ok() {
                        break;
                    }
                }

                pair?
            }
            MicrophoneDevicesConfig::VBCable => {
                microphone_pair_from_sink_name(&host, "CABLE Input")?
            }
            MicrophoneDevicesConfig::VoiceMeeter => {
                microphone_pair_from_sink_name(&host, "VoiceMeeter Input")?
            }
            MicrophoneDevicesConfig::VoiceMeeterAux => {
                microphone_pair_from_sink_name(&host, "VoiceMeeter Aux Input")?
            }
            MicrophoneDevicesConfig::VoiceMeeterVaio3 => {
                microphone_pair_from_sink_name(&host, "VoiceMeeter VAIO3 Input")?
            }
            MicrophoneDevicesConfig::Custom { sink, source } => (
                device_from_custom_config(&host, &sink)?,
                device_from_custom_config(&host, &source)?,
            ),
        };

        Ok((
            Self {
                inner: sink,
                is_output: true,
            },
            Self {
                inner: source,
                is_output: false,
            },
        ))
    }

    pub fn input_sample_rate(&self) -> StrResult<u32> {
        let config = self
            .inner
            .default_input_config()
            // On Windows, loopback devices are not recognized as input devices. Use output config.
            .or_else(|_| self.inner.default_output_config())
            .map_err(err!())?;

        Ok(config.sample_rate().0)
    }
}

pub fn is_same_device(device1: &AudioDevice, device2: &AudioDevice) -> bool {
    if let (Ok(name1), Ok(name2)) = (device1.inner.name(), device2.inner.name()) {
        name1 == name2
    } else {
        false
    }
}

#[cfg(windows)]
fn get_windows_device(device: &AudioDevice) -> StrResult<windows::Win32::Media::Audio::IMMDevice> {
    use widestring::U16CStr;
    use windows::Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Media::Audio::{eAll, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE},
        System::Com::{self, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ},
    };

    let device_name = device.inner.name().map_err(err!())?;

    unsafe {
        // This will fail the second time is called, ignore the error
        Com::CoInitializeEx(None, COINIT_MULTITHREADED).ok();

        let imm_device_enumerator: IMMDeviceEnumerator =
            Com::CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(err!())?;

        let imm_device_collection = imm_device_enumerator
            .EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE)
            .map_err(err!())?;

        let count = imm_device_collection.GetCount().map_err(err!())?;

        for i in 0..count {
            let imm_device = imm_device_collection.Item(i).map_err(err!())?;

            let property_store = imm_device.OpenPropertyStore(STGM_READ).map_err(err!())?;

            let mut prop_variant = property_store
                .GetValue(&PKEY_Device_FriendlyName)
                .map_err(err!())?;
            let utf16_name =
                U16CStr::from_ptr_str(prop_variant.Anonymous.Anonymous.Anonymous.pwszVal.0);
            Com::StructuredStorage::PropVariantClear(&mut prop_variant).map_err(err!())?;

            let imm_device_name = utf16_name.to_string().map_err(err!())?;
            if imm_device_name == device_name {
                return Ok(imm_device);
            }
        }

        fmt_e!("No device found with specified name")
    }
}

#[cfg(windows)]
pub fn get_windows_device_id(device: &AudioDevice) -> StrResult<String> {
    use widestring::U16CStr;
    use windows::Win32::System::Com;

    unsafe {
        let imm_device = get_windows_device(device)?;

        let id_str_ptr = imm_device.GetId().map_err(err!())?;
        let id_str = U16CStr::from_ptr_str(id_str_ptr.0)
            .to_string()
            .map_err(err!())?;
        Com::CoTaskMemFree(Some(id_str_ptr.0 as _));

        Ok(id_str)
    }
}

// device must be an output device
#[cfg(windows)]
fn set_mute_windows_device(device: &AudioDevice, mute: bool) -> StrResult {
    use windows::{
        core::GUID,
        Win32::{Media::Audio::Endpoints::IAudioEndpointVolume, System::Com::CLSCTX_ALL},
    };

    unsafe {
        let imm_device = get_windows_device(device)?;

        let endpoint_volume = imm_device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(err!())?;

        endpoint_volume
            .SetMute(mute, &GUID::zeroed())
            .map_err(err!())?;
    }

    Ok(())
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub async fn record_audio_loop(
    device: AudioDevice,
    channels_count: u16,
    mute: bool,
    mut sender: StreamSender<()>,
) -> StrResult {
    let config = device
        .inner
        .default_input_config()
        // On Windows, loopback devices are not recognized as input devices. Use output config.
        .or_else(|_| device.inner.default_output_config())
        .map_err(err!())?;

    if config.channels() > 2 {
        // todo: handle more than 2 channels
        return fmt_e!(
            "Audio devices with more than 2 channels are not supported. {}",
            "Please turn off surround audio."
        );
    }

    let stream_config = StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: BufferSize::Default,
    };

    // data_sender/receiver is the bridge between tokio and std thread
    let (data_sender, mut data_receiver) = tmpsc::unbounded_channel::<StrResult<Vec<_>>>();
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();

    let thread_callback = {
        let data_sender = data_sender.clone();
        move || {
            #[cfg(windows)]
            if mute && device.is_output {
                set_mute_windows_device(&device, true).ok();
            }

            let stream = device
                .inner
                .build_input_stream_raw(
                    &stream_config,
                    config.sample_format(),
                    {
                        let data_sender = data_sender.clone();
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

                            let data = if config.channels() == 1 && channels_count == 2 {
                                data.chunks_exact(2)
                                    .flat_map(|c| vec![c[0], c[1], c[0], c[1]])
                                    .collect()
                            } else if config.channels() == 2 && channels_count == 1 {
                                data.chunks_exact(4)
                                    .flat_map(|c| vec![c[0], c[1]])
                                    .collect()
                            } else {
                                data
                            };

                            data_sender.send(Ok(data)).ok();
                        }
                    },
                    {
                        let data_sender = data_sender.clone();
                        move |e| {
                            data_sender
                                .send(fmt_e!("Error while recording audio: {e}"))
                                .ok();
                        }
                    },
                    None,
                )
                .map_err(err!())?;

            stream.play().map_err(err!())?;

            shutdown_receiver.recv().ok();

            #[cfg(windows)]
            if mute && device.is_output {
                set_mute_windows_device(&device, false).ok();
            }

            Ok(vec![])
        }
    };

    // use a std thread to store the stream object. The stream object must be destroyed on the same
    // thread of creation.
    thread::spawn(move || {
        let res = thread_callback();
        if res.is_err() {
            data_sender.send(res).ok();
        }
    });

    // todo: reuse buffers also in the audio callback
    while let Some(maybe_data) = data_receiver.recv().await {
        sender.send(&(), maybe_data?).await.ok();
    }

    Ok(())
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
pub async fn receive_samples_loop(
    mut receiver: StreamReceiver<()>,
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    channels_count: usize,
    batch_frames_count: usize,
    average_buffer_frames_count: usize,
) -> StrResult {
    let mut receiver_buffer = ReceiverBuffer::new();
    let mut recovery_sample_buffer = vec![];
    loop {
        receiver.recv_buffer(&mut receiver_buffer).await?;
        let (_, packet) = receiver_buffer.get()?;

        let new_samples = packet
            .chunks_exact(2)
            .map(|c| i16::from_ne_bytes([c[0], c[1]]).to_sample::<f32>())
            .collect::<Vec<_>>();

        let mut sample_buffer_ref = sample_buffer.lock();

        if receiver_buffer.had_packet_loss() {
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

        if sample_buffer_ref.len() == 0 || receiver_buffer.had_packet_loss() {
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

                if receiver_buffer.had_packet_loss()
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
                    sample_buffer_ref[index] =
                        sample_buffer_ref[index] * volume + drained_samples[index] * (1. - volume);
                }
            }
        }
    }
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
    fn current_frame_len(&self) -> Option<usize> {
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

pub async fn play_audio_loop(
    device: AudioDevice,
    channels_count: u16,
    sample_rate: u32,
    config: AudioBufferingConfig,
    receiver: StreamReceiver<()>,
) -> StrResult {
    // Size of a chunk of frames. It corresponds to the duration if a fade-in/out in frames.
    let batch_frames_count = sample_rate as usize * config.batch_ms as usize / 1000;

    // Average buffer size in frames
    let average_buffer_frames_count =
        sample_rate as usize * config.average_buffering_ms as usize / 1000;

    let sample_buffer = Arc::new(Mutex::new(VecDeque::new()));

    // Store the stream in a thread (because !Send)
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    thread::spawn({
        let sample_buffer = Arc::clone(&sample_buffer);
        move || -> StrResult {
            let (_stream, handle) = OutputStream::try_from_device(&device.inner).map_err(err!())?;

            let source = StreamingSource {
                sample_buffer,
                current_batch: vec![],
                current_batch_cursor: 0,
                channels_count: channels_count as _,
                sample_rate,
                batch_frames_count,
            };
            handle.play_raw(source).map_err(err!())?;

            shutdown_receiver.recv().ok();
            Ok(())
        }
    });

    receive_samples_loop(
        receiver,
        sample_buffer,
        channels_count as _,
        batch_frames_count,
        average_buffer_frames_count,
    )
    .await
}
