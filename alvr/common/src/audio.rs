use crate::{
    data::{AudioConfig, AudioDeviceId},
    sockets::{StreamReceiver, StreamSender},
    *,
};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, Sample, SampleFormat, StreamConfig,
};
use parking_lot::Mutex;
use rodio::{source::SineWave, OutputStream, Sink, Source};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as smpsc, Arc,
    },
    thread,
};
use tokio::sync::mpsc as tmpsc;

#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use winapi::{
    shared::winerror::*,
    um::{combaseapi::*, endpointvolume::IAudioEndpointVolume, mmdeviceapi::*, objbase::*},
    Class, Interface,
};
#[cfg(windows)]
use wio::com::ComPtr;

#[cfg(windows)]
fn set_mute_audio_device(device_id: AudioDeviceId, mute: bool) -> StrResult {
    let device_index = match device_id {
        AudioDeviceId::Default => None,
        AudioDeviceId::Name(name_substring) => trace_err!(cpal::default_host().output_devices())?
            .enumerate()
            .find_map(|(i, d)| {
                if d.name().ok()?.contains(&name_substring) {
                    Some(i)
                } else {
                    None
                }
            }),
        AudioDeviceId::Index(index) => Some(index as _),
    };

    unsafe {
        CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);

        let mut mm_device_enumerator_ptr: *mut IMMDeviceEnumerator = ptr::null_mut();
        let hr = CoCreateInstance(
            &MMDeviceEnumerator::uuidof(),
            ptr::null_mut(),
            CLSCTX_ALL,
            &IMMDeviceEnumerator::uuidof(),
            &mut mm_device_enumerator_ptr as *mut _ as _,
        );
        if FAILED(hr) {
            return fmt_e!(
                "CoCreateInstance(IMMDeviceEnumerator) failed: hr = 0x{:08x}",
                hr
            );
        }
        let mm_device_enumerator = ComPtr::from_raw(mm_device_enumerator_ptr);

        let mm_device = if let Some(index) = device_index {
            let mut mm_device_collection_ptr: *mut IMMDeviceCollection = ptr::null_mut();
            let hr = mm_device_enumerator.EnumAudioEndpoints(
                eRender,
                DEVICE_STATE_ACTIVE,
                &mut mm_device_collection_ptr as _,
            );
            if FAILED(hr) {
                return fmt_e!(
                    "IMMDeviceEnumerator::EnumAudioEndpoints failed: hr = 0x{:08x}",
                    hr
                );
            }
            let mm_device_collection = ComPtr::from_raw(mm_device_collection_ptr);

            let mut mm_device_ptr: *mut IMMDevice = ptr::null_mut();
            let hr = mm_device_collection.Item(index as _, &mut mm_device_ptr as _);
            if FAILED(hr) {
                return fmt_e!("IMMDeviceCollection::Item failed: hr = 0x{:08x}", hr);
            }

            ComPtr::from_raw(mm_device_ptr)
        } else {
            let mut mm_device_ptr: *mut IMMDevice = ptr::null_mut();
            let hr = mm_device_enumerator.GetDefaultAudioEndpoint(
                eRender,
                eConsole,
                &mut mm_device_ptr as *mut _,
            );
            if hr == HRESULT_FROM_WIN32(ERROR_NOT_FOUND) {
                return fmt_e!("No default audio endpoint found. No audio device?");
            }
            if FAILED(hr) {
                return fmt_e!(
                    "IMMDeviceEnumerator::GetDefaultAudioEndpoint failed: hr = 0x{:08x}",
                    hr
                );
            }

            ComPtr::from_raw(mm_device_ptr)
        };

        let mut endpoint_volume_ptr: *mut IAudioEndpointVolume = ptr::null_mut();
        let hr = mm_device.Activate(
            &IAudioEndpointVolume::uuidof(),
            CLSCTX_ALL,
            ptr::null_mut(),
            &mut endpoint_volume_ptr as *mut _ as _,
        );
        if FAILED(hr) {
            return fmt_e!(
                "IMMDevice::Activate() for IAudioEndpointVolume failed: hr = 0x{:08x}",
                hr,
            );
        }
        let endpoint_volume = ComPtr::from_raw(endpoint_volume_ptr);

        let hr = endpoint_volume.SetMute(mute as _, ptr::null_mut());
        if FAILED(hr) {
            return fmt_e!("Failed to mute audio device: hr = 0x{:08x}", hr,);
        }
    }

    Ok(())
}

pub enum AudioDeviceType {
    Output,
    Input,
    VirtualMicrophone,
}

pub struct AudioDevice {
    inner: Device,
    id: AudioDeviceId,
    device_type: AudioDeviceType,
}

impl AudioDevice {
    pub fn new(id: AudioDeviceId, device_type: AudioDeviceType) -> StrResult<Self> {
        let host = cpal::default_host();

        let device = match &id {
            AudioDeviceId::Default => match device_type {
                AudioDeviceType::Output => host
                    .default_output_device()
                    .ok_or_else(|| "No output audio device found".to_owned())?,
                AudioDeviceType::Input => host
                    .default_input_device()
                    .ok_or_else(|| "No input audio device found".to_owned())?,
                AudioDeviceType::VirtualMicrophone => trace_err!(host.output_devices())?
                    .find(|d| {
                        if let Ok(name) = d.name() {
                            name.contains("CABLE Input")
                        } else {
                            false
                        }
                    })
                    .ok_or_else(|| {
                        format!(
                            "CABLE Input device not found. {}",
                            "Did you install VB-CABLE Virtual Audio Device?"
                        )
                    })?,
            },
            AudioDeviceId::Name(name_substring) => {
                let mut devices = trace_err!(if matches!(device_type, AudioDeviceType::Input) {
                    host.input_devices()
                } else {
                    host.output_devices()
                })?;

                devices
                    .find(|d| {
                        if let Ok(name) = d.name() {
                            name.to_lowercase().contains(&name_substring.to_lowercase())
                        } else {
                            false
                        }
                    })
                    .ok_or_else(|| {
                        format!(
                            "Cannot find audio device which name contains \"{}\"",
                            name_substring
                        )
                    })?
            }
            AudioDeviceId::Index(index) => {
                let mut devices = trace_err!(if matches!(device_type, AudioDeviceType::Input) {
                    host.input_devices()
                } else {
                    host.output_devices()
                })?;

                devices
                    .nth(*index as usize - 1)
                    .ok_or_else(|| format!("Cannot find audio device at index {}", index))?
            }
        };

        Ok(Self {
            inner: device,
            id,
            device_type,
        })
    }
}

pub fn get_sample_rate(device: &AudioDevice) -> StrResult<u32> {
    let mut configs = trace_err!(device.inner.supported_output_configs())?;

    // Assumption: device is in shared mode: this means that there is one and fixed sample rate,
    // format and channel count
    Ok(trace_none!(configs.next())?.min_sample_rate().0)
}

// samples_bytes must be of format I16
fn convert_channels_count(
    samples_bytes: &[u8],
    source_channels_count: u16,
    dest_channel_count: u16,
) -> Vec<u8> {
    if source_channels_count == 1 && dest_channel_count == 2 {
        samples_bytes
            .chunks_exact(2)
            .flat_map(|c| vec![c[0], c[1], c[0], c[1]])
            .collect()
    } else if source_channels_count == 2 && dest_channel_count == 1 {
        samples_bytes
            .chunks_exact(4)
            .flat_map(|c| vec![c[0], c[1]])
            .collect()
    } else {
        // I assume the other case is source_channels_count == dest_channel_count. Otherwise the
        // buffer will be mishandled but no error will occur.
        samples_bytes.to_vec()
    }
}

pub async fn record_audio_loop(
    device: AudioDevice,
    channels_count: u16,
    sample_rate: u32,
    mute: bool,
    mut sender: StreamSender<()>,
) -> StrResult {
    let config = trace_none!(trace_err!(device.inner.supported_output_configs())?.next())?;

    if sample_rate != config.min_sample_rate().0 {
        return fmt_e!("Sample rate not supported");
    }

    let stream_config = StreamConfig {
        channels: config.channels(),
        sample_rate: config.min_sample_rate(),
        buffer_size: BufferSize::Default,
    };

    // data_sender/receiver is the bridge between tokio and std thread
    let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();

    // use a std thread to store the stream object. The stream object must be destroyed on the same
    // thread of creation.
    thread::spawn(move || -> StrResult {
        #[cfg(windows)]
        if mute && matches!(device.device_type, AudioDeviceType::Output) {
            set_mute_audio_device(device.id.clone(), true).ok();
        }

        let stream = trace_err!(device.inner.build_input_stream_raw(
            &stream_config,
            config.sample_format(),
            move |data, _| {
                let data = if config.sample_format() == SampleFormat::F32 {
                    data.bytes()
                        .chunks_exact(4)
                        .flat_map(|c| {
                            f32::from_ne_bytes([c[0], c[1], c[2], c[3]])
                                .to_i16()
                                .to_ne_bytes()
                                .to_vec()
                        })
                        .collect()
                } else {
                    data.bytes().to_vec()
                };
                let data = convert_channels_count(&data, config.channels(), channels_count);
                data_sender.send(data).ok();
            },
            |e| warn!("Error while recording audio: {}", e),
        ))?;

        trace_err!(stream.play())?;

        shutdown_receiver.recv().ok();

        #[cfg(windows)]
        if mute && matches!(device.device_type, AudioDeviceType::Output) {
            set_mute_audio_device(device.id, false).ok();
        }

        Ok(())
    });

    while let Some(data) = data_receiver.recv().await {
        let mut buffer = sender.new_buffer(&(), data.len())?;
        buffer.get_mut().extend(data);
        sender.send_buffer(buffer).await.ok();
    }

    Ok(())
}

enum PlayState {
    Playing,
    Underflow,
    Overflow,
    PacketLoss,
    Recovering,
}

struct StreamingSource {
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    channels: u16,
    sample_rate: u32,
    play_state: Arc<Mutex<PlayState>>,
}

impl Source for StreamingSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
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

    fn next(&mut self) -> Option<f32> {
        let mut play_state_ref = self.play_state.lock();

        let maybe_sample = self.sample_buffer.lock().pop_front();
        if maybe_sample.is_none() {
            *play_state_ref = PlayState::Underflow;
        }

        maybe_sample
    }
}

pub async fn play_audio_loop(
    device: AudioDevice,
    channels_count: u16,
    sample_rate: u32,
    config: AudioConfig,
    mut receiver: StreamReceiver<()>,
) -> StrResult {
    assert!(!matches!(device.device_type, AudioDeviceType::Input));

    // let fade_frames_count = sample_rate as usize * config.fade_ms as usize / 1000;
    let min_buffer_samples_count =
        sample_rate as usize * channels_count as usize * config.min_buffering_ms as usize / 1000;

    // store the stream in a thread (because !Send) and extract the playback handle
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    let (stream_handle_sender, stream_handler_retriever) = oneshot::channel();
    thread::spawn(move || -> StrResult {
        let (_stream, handle) = trace_err!(OutputStream::try_from_device(&device.inner))?;
        stream_handle_sender.send(handle).ok();

        shutdown_receiver.recv().ok();
        Ok(())
    });

    let stream_handle = trace_err!(stream_handler_retriever.await)?;
    let playback_sink = trace_err!(Sink::try_new(&stream_handle))?;

    let mut old_sample_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let mut sample_buffer = Arc::new(Mutex::new(VecDeque::new()));

    let mut play_state = Arc::new(Mutex::new(PlayState::Underflow));

    playback_sink.play();

    loop {
        let packet = receiver.recv().await?;

        error!(
            "mic received. playing: {}, queue len:{}",
            !playback_sink.is_paused(),
            playback_sink.len()
        );

        let frames = packet
            .buffer
            .chunks_exact(2)
            .map(|c| i16::from_ne_bytes([c[0], c[1]]).to_f32())
            .collect::<VecDeque<_>>();

        // assign a new object to play_state to avoid the previous source to report underflow
        if packet.had_packet_loss {
            error!("packet loss");
            play_state = Arc::new(Mutex::new(PlayState::PacketLoss));
        } else if sample_buffer.lock().len() + frames.len() > 2 * min_buffer_samples_count {
            error!("overflow");
            play_state = Arc::new(Mutex::new(PlayState::Overflow));
        }

        if matches!(*play_state.lock(), PlayState::Underflow) {
            error!("underflow");
        }

        let mut play_state_ref = play_state.lock();

        if matches!(
            *play_state_ref,
            PlayState::Underflow | PlayState::Overflow | PlayState::PacketLoss
        ) {
            error!("swap buffers");
            old_sample_buffer.lock().clear();
            old_sample_buffer = sample_buffer;
            sample_buffer = Arc::new(Mutex::new(VecDeque::new()));
            *play_state_ref = PlayState::Recovering;
        }

        let mut sample_buffer_ref = sample_buffer.lock();
        sample_buffer_ref.extend(frames);

        if matches!(*play_state_ref, PlayState::Recovering)
            && sample_buffer_ref.len() > min_buffer_samples_count
        {
            error!("recovering");
            old_sample_buffer.lock().clear();

            let streaming_source = StreamingSource {
                sample_buffer: sample_buffer.clone(),
                channels: channels_count,
                sample_rate,
                play_state: play_state.clone(),
            };
            playback_sink.append(streaming_source);

            *play_state_ref = PlayState::Playing;
            error!("recovered!");
        }
    }
}
