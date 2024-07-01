use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use crate::{device_from_custom_config, receive_samples_loop, AudioDevice, StreamingSource};
use alvr_common::{
    anyhow::{anyhow, bail, Result},
    once_cell::sync::Lazy,
};
use alvr_session::{AudioBufferingConfig, CustomAudioDeviceConfig, MicrophoneDevicesConfig};
use alvr_sockets::StreamReceiver;
use cpal::{Device, Host};
use rodio::{DeviceTrait, OutputStream};
use windows::{
    core::GUID,
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Media::Audio::{
            eAll, Endpoints::IAudioEndpointVolume, IMMDevice, IMMDeviceEnumerator,
            MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
        },
        System::Com::{self, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ},
    },
};

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

impl AudioDevice {
    pub fn new_output(config: Option<&CustomAudioDeviceConfig>) -> Result<Self> {
        let host = cpal::default_host();

        let device = match config {
            None => host
                .default_output_device()
                .context("No output audio device found")?,
            Some(config) => device_from_custom_config(&host, config)?,
        };

        Ok(Self {
            inner: device,
            is_output: true,
        })
    }

    pub fn new_input(config: Option<CustomAudioDeviceConfig>) -> Result<Self> {
        let host = cpal::default_host();

        let device = match config {
            None => host
                .default_input_device()
                .context("No input audio device found")?,
            Some(config) => device_from_custom_config(&host, &config)?,
        };

        Ok(Self {
            inner: device,
            is_output: false,
        })
    }

    // returns (sink, source)
    pub fn new_virtual_microphone_pair(config: MicrophoneDevicesConfig) -> Result<(Self, Self)> {
        #[cfg(target_os = "linux")]
        let host = cpal::default_host();

        let (sink, source) = match config {
            MicrophoneDevicesConfig::Automatic => {
                let mut pair = Err(anyhow!("No microphones found"));
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

    pub fn input_sample_rate(&self) -> Result<u32> {
        let config = self
            .inner
            .default_input_config()
            // On Windows, loopback devices are not recognized as input devices. Use output config.
            .or_else(|_| self.inner.default_output_config())?;

        Ok(config.sample_rate().0)
    }
}

fn microphone_pair_from_sink_name(host: &Host, sink_name: &str) -> Result<(Device, Device)> {
    let sink = host
        .output_devices()?
        .find(|d| d.name().unwrap_or_default().contains(sink_name))
        .context("VB-CABLE or Voice Meeter not found. Please install or reinstall either one")?;

    if let Some(source_name) = VIRTUAL_MICROPHONE_PAIRS.get(sink_name) {
        Ok((
            sink,
            host.input_devices()?
                .find(|d| {
                    d.name()
                        .map(|name| name.contains(source_name))
                        .unwrap_or(false)
                })
                .context("Matching output microphone not found. Did you rename it?")?,
        ))
    } else {
        unreachable!("Invalid argument")
    }
}

fn get_windows_device(device: &AudioDevice) -> Result<IMMDevice> {
    let device_name = device.inner.name()?;

    unsafe {
        // This will fail the second time is called, ignore the error
        Com::CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok();

        let imm_device_enumerator: IMMDeviceEnumerator =
            Com::CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        let imm_device_collection =
            imm_device_enumerator.EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE)?;

        for i in 0..imm_device_collection.GetCount()? {
            let imm_device = imm_device_collection.Item(i)?;

            let imm_device_name = imm_device
                .OpenPropertyStore(STGM_READ)?
                .GetValue(&PKEY_Device_FriendlyName)?
                .to_string();

            if imm_device_name == device_name {
                return Ok(imm_device);
            }
        }

        bail!("No device found with specified name")
    }
}

pub fn get_windows_device_id(device: &AudioDevice) -> Result<String> {
    unsafe {
        let imm_device = get_windows_device(device)?;

        let id_str_ptr = imm_device.GetId()?;
        let id_str = id_str_ptr.to_string()?;
        Com::CoTaskMemFree(Some(id_str_ptr.0 as _));

        Ok(id_str)
    }
}

// device must be an output device
pub fn set_mute_windows_device(device: &AudioDevice, mute: bool) -> Result<()> {
    unsafe {
        let imm_device = get_windows_device(device)?;

        let endpoint_volume = imm_device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)?;

        endpoint_volume.SetMute(mute, &GUID::zeroed())?;
    }

    Ok(())
}

pub fn play_audio_loop_cpal(
    is_running: impl Fn() -> bool,
    device: &AudioDevice,
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

    let (_stream, handle) = OutputStream::try_from_device(&device.inner)?;

    handle.play_raw(StreamingSource {
        sample_buffer: Arc::clone(&sample_buffer),
        current_batch: vec![],
        current_batch_cursor: 0,
        channels_count: channels_count as _,
        sample_rate,
        batch_frames_count,
    })?;

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
