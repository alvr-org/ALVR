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
use rodio::{OutputStream, Source};
use std::{
    collections::VecDeque,
    f32::consts::PI,
    sync::{mpsc as smpsc, Arc},
    thread,
};
use tokio::sync::mpsc as tmpsc;

#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use widestring::U16CStr;
#[cfg(windows)]
use winapi::{
    shared::{winerror::*, wtypes::VT_LPWSTR},
    um::{
        combaseapi::*, coml2api::STGM_READ, endpointvolume::IAudioEndpointVolume,
        functiondiscoverykeys_devpkey::PKEY_Device_FriendlyName, mmdeviceapi::*, objbase::*,
        propidl::PROPVARIANT, propsys::IPropertyStore,
    },
    Class, Interface,
};
#[cfg(windows)]
use wio::com::ComPtr;

pub enum AudioDeviceType {
    Output,
    Input,

    // for the virtual microphone devices, input and output labels are swapped
    VirtualMicrophoneInput,
    VirtualMicrophoneOutput,
}

impl AudioDeviceType {
    fn is_output(&self) -> bool {
        matches!(self, Self::Output | Self::VirtualMicrophoneInput)
    }
}

pub struct AudioDevice {
    inner: Device,

    #[allow(dead_code)]
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
                AudioDeviceType::VirtualMicrophoneInput => trace_err!(host.output_devices())?
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
                AudioDeviceType::VirtualMicrophoneOutput => trace_err!(host.input_devices())?
                    .find(|d| {
                        if let Ok(name) = d.name() {
                            name.contains("CABLE Output")
                        } else {
                            false
                        }
                    })
                    .ok_or_else(|| {
                        format!(
                            "CABLE Output device not found. {}",
                            "Did you install VB-CABLE Virtual Audio Device?"
                        )
                    })?,
            },
            AudioDeviceId::Name(name_substring) => {
                let mut devices = trace_err!(if device_type.is_output() {
                    host.output_devices()
                } else {
                    host.input_devices()
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
                let mut devices = trace_err!(if device_type.is_output() {
                    host.output_devices()
                } else {
                    host.input_devices()
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

pub fn is_same_device(device1: &AudioDevice, device2: &AudioDevice) -> bool {
    if let (Ok(name1), Ok(name2)) = (device1.inner.name(), device2.inner.name()) {
        name1 == name2
    } else {
        false
    }
}

#[cfg(windows)]
fn get_windows_device(device: &AudioDevice) -> StrResult<ComPtr<IMMDevice>> {
    let device_name = trace_err!(device.inner.name())?;

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

        let mut mm_device_collection_ptr: *mut IMMDeviceCollection = ptr::null_mut();
        let hr = mm_device_enumerator.EnumAudioEndpoints(
            eAll,
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

        // GetCount has a wrong signature (*const parameter instead of *mut). Count needs to be a
        // mutable variable even if not enforced.
        #[allow(unused_mut)]
        let mut count = 0;
        let hr = mm_device_collection.GetCount(&count);
        if FAILED(hr) {
            return fmt_e!("IMMDeviceCollection::GetCount failed: hr = 0x{:08x}", hr);
        }

        for i in 0..count {
            let mut mm_device_ptr: *mut IMMDevice = ptr::null_mut();
            let hr = mm_device_collection.Item(i as _, &mut mm_device_ptr as _);
            if FAILED(hr) {
                return fmt_e!("IMMDeviceCollection::Item failed: hr = 0x{:08x}", hr);
            }
            let mm_device = ComPtr::from_raw(mm_device_ptr);

            let mut property_store_ptr: *mut IPropertyStore = ptr::null_mut();
            let hr = mm_device.OpenPropertyStore(STGM_READ, &mut property_store_ptr as _);
            if FAILED(hr) {
                return fmt_e!("IMMDevice::OpenPropertyStore failed: hr = 0x{:08x}", hr);
            }
            let property_store = ComPtr::from_raw(property_store_ptr);

            let mut prop_variant = PROPVARIANT::default();
            let hr = property_store.GetValue(&PKEY_Device_FriendlyName, &mut prop_variant);
            if FAILED(hr) {
                return fmt_e!("IPropertyStore::GetValue failed: hr = 0x{:08x}", hr);
            }
            if prop_variant.vt as u32 != VT_LPWSTR {
                return fmt_e!(
                    "PKEY_Device_FriendlyName variant type is {} - expected VT_LPWSTR",
                    prop_variant.vt
                );
            }
            let utf16_name = U16CStr::from_ptr_str(*prop_variant.data.pwszVal());
            let hr = PropVariantClear(&mut prop_variant);
            if FAILED(hr) {
                return fmt_e!("PropVariantClear failed: hr = 0x{:08x}", hr);
            }

            let mm_device_name = trace_err!(utf16_name.to_string())?;
            if mm_device_name == device_name {
                return Ok(mm_device);
            }
        }

        fmt_e!("No device found with specified name")
    }
}

#[cfg(windows)]
pub fn get_windows_device_id(device: &AudioDevice) -> StrResult<String> {
    unsafe {
        let mm_device = get_windows_device(device)?;

        let mut id_str_ptr = ptr::null_mut();
        mm_device.GetId(&mut id_str_ptr);
        let id_str = trace_err!(U16CStr::from_ptr_str(id_str_ptr).to_string())?;
        CoTaskMemFree(id_str_ptr as _);

        Ok(id_str)
    }
}

// device must be an output device
#[cfg(windows)]
fn set_mute_windows_device(device: &AudioDevice, mute: bool) -> StrResult {
    unsafe {
        let mm_device = get_windows_device(device)?;

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

pub fn get_sample_rate(device: &AudioDevice) -> StrResult<u32> {
    let mut configs = trace_err!(device.inner.supported_output_configs())?;

    // Assumption: device is in shared mode: this means that there is one and fixed sample rate,
    // format and channel count
    Ok(trace_none!(configs.next())?.min_sample_rate().0)
}

pub async fn record_audio_loop(
    device: AudioDevice,
    channels_count: u16,
    sample_rate: u32,
    #[allow(unused_variables)] mute: bool,
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
        if mute && device.device_type.is_output() {
            set_mute_windows_device(&device, true).ok();
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

                let data = if config.channels() == 1 && channels_count == 2 {
                    data.chunks_exact(2)
                        .flat_map(|c| vec![c[0], c[1], c[0], c[1]])
                        .collect()
                } else if config.channels() == 2 && channels_count == 1 {
                    data.chunks_exact(4)
                        .flat_map(|c| vec![c[0], c[1]])
                        .collect()
                } else {
                    // I assume the other case is config.channels() == channels_count. Otherwise the
                    // buffer will be mishandled but no error will occur.
                    data.to_vec()
                };

                data_sender.send(data).ok();
            },
            |e| warn!("Error while recording audio: {}", e),
        ))?;

        trace_err!(stream.play())?;

        shutdown_receiver.recv().ok();

        #[cfg(windows)]
        if mute && device.device_type.is_output() {
            set_mute_windows_device(&device, false).ok();
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
    FadeOutOrPaused,
    FadeInOrResumed { fade_in_progress_frames: usize },
}

impl Default for PlayState {
    fn default() -> Self {
        Self::FadeOutOrPaused
    }
}

// Between locks, sample_buffer and fade_outs must contain an integer number of frames and the first
// sample must correspond to channel 0.
#[derive(Default)]
pub struct AudioState {
    sample_buffer: VecDeque<f32>,

    // Prerendered fade-outs. In case of intermittent packet loss, multiple fade-outs could overlap.
    // A separate buffer is used because the samples we need in sample_buffer could get removed
    // due to buffer overgrowth.
    fade_outs: VecDeque<f32>,

    play_state: PlayState,
}

// Render a fade-out. It is aware of a in-progress fade-in. This is done only if play_state is not
// already in FadeOutOrPaused and if sample_buffer has enough samples.
#[inline]
fn maybe_add_fade_out(
    audio_state: &mut AudioState,
    channels_count: usize,
    fade_frames_count: usize,
) {
    if audio_state.sample_buffer.len() / channels_count >= fade_frames_count {
        if let PlayState::FadeInOrResumed {
            fade_in_progress_frames,
        } = audio_state.play_state
        {
            let fade_out_start = fade_frames_count - fade_in_progress_frames;

            for f in 0..fade_frames_count - fade_out_start {
                let volume =
                    (PI * (f + fade_out_start) as f32 / fade_frames_count as f32).cos() / 2. + 0.5;

                let sample_index_base = f * channels_count;
                if f < audio_state.fade_outs.len() / channels_count {
                    for c in 0..channels_count {
                        audio_state.fade_outs[sample_index_base + c] +=
                            audio_state.sample_buffer[sample_index_base + c] * volume;
                    }
                } else {
                    for c in 0..channels_count {
                        audio_state
                            .fade_outs
                            .push_back(audio_state.sample_buffer[sample_index_base + c] * volume);
                    }
                }
            }
        }

        audio_state.play_state = PlayState::FadeOutOrPaused;
    }
}

#[inline]
pub fn get_next_frame(
    audio_state_ref: &mut AudioState,
    channels_count: usize,
    fade_frames_count: usize,
) -> Vec<f32> {
    let mut frame = if audio_state_ref.sample_buffer.len() / channels_count > fade_frames_count {
        let mut fade_in_progress_frames = match audio_state_ref.play_state {
            PlayState::FadeInOrResumed {
                fade_in_progress_frames,
            } => fade_in_progress_frames,
            PlayState::FadeOutOrPaused => 0,
        };

        let mut frame = audio_state_ref
            .sample_buffer
            .drain(0..channels_count)
            .collect::<Vec<_>>();

        if fade_in_progress_frames < fade_frames_count {
            let volume =
                (PI * fade_in_progress_frames as f32 / fade_frames_count as f32).cos() / -2. + 0.5;

            for sample in &mut frame {
                *sample *= volume;
            }

            fade_in_progress_frames += 1;
            audio_state_ref.play_state = PlayState::FadeInOrResumed {
                fade_in_progress_frames,
            };
        }

        frame
    } else {
        if matches!(
            audio_state_ref.play_state,
            PlayState::FadeInOrResumed { .. }
        ) {
            info!(
                "Audio buffer underflow! size: {}",
                audio_state_ref.sample_buffer.len()
            );
        }

        maybe_add_fade_out(&mut *audio_state_ref, channels_count, fade_frames_count);

        vec![0.; channels_count]
    };

    if audio_state_ref.fade_outs.len() >= channels_count {
        for (idx, sample) in audio_state_ref
            .fade_outs
            .drain(0..channels_count)
            .enumerate()
        {
            frame[idx] += sample;
        }
    }

    frame
}

pub async fn receive_samples_loop(
    mut receiver: StreamReceiver<()>,
    audio_state: Arc<Mutex<AudioState>>,
    channels_count: usize,
    fade_frames_count: usize,
    min_buffer_frames_count: usize,
) -> StrResult {
    loop {
        let packet = receiver.recv().await?;
        let samples = packet
            .buffer
            .chunks_exact(2)
            .map(|c| i16::from_ne_bytes([c[0], c[1]]).to_f32())
            .collect::<Vec<_>>();

        let mut audio_state_ref = audio_state.lock();

        if packet.had_packet_loss {
            info!("Audio packet loss!");

            // Add a fade-out *before* draining sample_buffer
            maybe_add_fade_out(&mut *audio_state_ref, channels_count, fade_frames_count);

            // sample_buffer must be drained completely. There is no way of reusing the old frames
            // without discontinuity.
            audio_state_ref.sample_buffer.clear();
        }

        audio_state_ref.sample_buffer.extend(&samples);

        // todo: use smarter policy with EventTiming
        let buffer_size = audio_state_ref.sample_buffer.len();
        if buffer_size > 2 * min_buffer_frames_count + fade_frames_count {
            info!("Audio buffer overflow! size: {}", buffer_size);

            // Add a fade-out *before* draining sample_buffer
            maybe_add_fade_out(&mut *audio_state_ref, channels_count, fade_frames_count);

            // Drain sample_buffer partially. A discontinuity is formed but the playback can resume
            // immediately with a fade-in
            audio_state_ref.sample_buffer.drain(
                0..(buffer_size - min_buffer_frames_count - fade_frames_count) * channels_count,
            );
        }
    }
}

struct StreamingSource {
    audio_state: Arc<Mutex<AudioState>>,
    current_frame: Vec<f32>,
    channel_index: usize,
    channels_count: usize,
    sample_rate: u32,

    fade_frames_count: usize,
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
        let mut audio_state_ref = self.audio_state.lock();

        if self.channel_index == 0 {
            self.current_frame = get_next_frame(
                &mut audio_state_ref,
                self.channels_count,
                self.fade_frames_count,
            );
        }

        let sample = self.current_frame[self.channel_index];

        self.channel_index = (self.channel_index + 1) % self.channels_count;

        Some(sample)
    }
}

pub async fn play_audio_loop(
    device: AudioDevice,
    channels_count: u16,
    sample_rate: u32,
    config: AudioConfig,
    receiver: StreamReceiver<()>,
) -> StrResult {
    assert!(device.device_type.is_output());

    // length of fade-in/out in frames
    let fade_frames_count = sample_rate as usize * config.fade_ms as usize / 1000;

    // average buffer size in frames
    let min_buffer_frames_count =
        sample_rate as usize * config.average_buffering_ms as usize / 1000;

    let audio_state = Arc::new(Mutex::new(AudioState::default()));

    // store the stream in a thread (because !Send) and extract the playback handle
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    thread::spawn({
        let audio_state = audio_state.clone();
        move || -> StrResult {
            let (_stream, handle) = trace_err!(OutputStream::try_from_device(&device.inner))?;

            let source = StreamingSource {
                audio_state,
                current_frame: vec![],
                channel_index: 0,
                channels_count: channels_count as _,
                sample_rate,
                fade_frames_count,
            };
            trace_err!(handle.play_raw(source))?;

            shutdown_receiver.recv().ok();
            Ok(())
        }
    });

    receive_samples_loop(
        receiver,
        audio_state,
        channels_count as _,
        fade_frames_count,
        min_buffer_frames_count,
    )
    .await
}
