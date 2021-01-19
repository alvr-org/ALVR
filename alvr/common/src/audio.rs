use crate::{data::*, *};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, Stream, StreamConfig, SupportedBufferSize, SupportedStreamConfigRange,
};
use std::{
    cmp::{max, min, Ordering},
    collections::VecDeque,
    ptr,
    sync::mpsc as smpsc,
};
use tokio::sync::mpsc as tmpsc;

#[cfg(windows)]
mod winaudio {
    use super::*;

    use widestring::*;
    use winapi::{
        shared::{winerror::*, wtypes::VT_LPWSTR},
        um::{
            combaseapi::*, coml2api::STGM_READ,
            functiondiscoverykeys_devpkey::PKEY_Device_FriendlyName, mmdeviceapi::*,
            objbase::CoInitialize, propidl::PROPVARIANT, propsys::IPropertyStore,
        },
        Class, Interface,
    };
    use wio::com::ComPtr;

    #[derive(serde::Serialize)]
    pub struct AudioDevicesDesc {
        pub list: Vec<(String, String)>,
        pub default_game_audio: Option<String>,
        pub default_microphone: Option<String>,
    }

    // from AudioEndPointDescriptor::GetDeviceName
    fn get_device_name(mm_device: ComPtr<IMMDevice>) -> StrResult<String> {
        unsafe {
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

            let res = trace_err!(U16CStr::from_ptr_str(*prop_variant.data.pwszVal()).to_string());

            let hr = PropVariantClear(&mut prop_variant);
            if FAILED(hr) {
                return fmt_e!("PropVariantClear failed: hr = 0x{:08x}", hr);
            }

            res
        }
    }

    // from AudioEndPointDescriptor contructor
    fn get_audio_device_id_and_name(device: ComPtr<IMMDevice>) -> StrResult<(String, String)> {
        let id_str = unsafe {
            let mut id_str_ptr = ptr::null_mut();
            device.GetId(&mut id_str_ptr);
            let id_str = trace_err!(U16CStr::from_ptr_str(id_str_ptr).to_string())?;
            CoTaskMemFree(id_str_ptr as _);

            id_str
        };

        Ok((id_str, get_device_name(device)?))
    }

    // from AudioCapture::list_devices
    pub fn output_audio_devices() -> StrResult<AudioDevicesDesc> {
        let mut device_list = vec![];
        unsafe {
            CoInitialize(ptr::null_mut());

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

            let mut default_mm_device_ptr: *mut IMMDevice = ptr::null_mut();
            let hr = mm_device_enumerator.GetDefaultAudioEndpoint(
                eRender,
                eConsole,
                &mut default_mm_device_ptr as *mut _,
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
            let default_mm_device = ComPtr::from_raw(default_mm_device_ptr);
            let (default_id, default_name) = get_audio_device_id_and_name(default_mm_device)?;
            device_list.push((default_id.clone(), default_name.clone()));

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

            #[allow(unused_mut)]
            let mut count = 0; // without mut this is UB
            let hr = mm_device_collection.GetCount(&count);
            if FAILED(hr) {
                return fmt_e!("IMMDeviceCollection::GetCount failed: hr = 0x{:08x}", hr);
            }
            debug!("Active render endpoints found: {}", count);

            debug!("DefaultDevice:{} ID:{}", default_name, default_id);

            for i in 0..count {
                let mut mm_device_ptr: *mut IMMDevice = ptr::null_mut();
                let hr = mm_device_collection.Item(i, &mut mm_device_ptr as _);
                if FAILED(hr) {
                    warn!("Crash!");
                    return fmt_e!("IMMDeviceCollection::Item failed: hr = 0x{:08x}", hr);
                }
                let mm_device = ComPtr::from_raw(mm_device_ptr);
                let (id, name) = get_audio_device_id_and_name(mm_device)?;
                if id == default_id {
                    continue;
                }
                debug!("Device{}:{} ID:{}", i, name, id);
                device_list.push((id, name));
            }
        }

        let default_game_audio = device_list.get(0).map(|dev| dev.0.clone());
        let default_microphone = device_list
            .iter()
            .find(|(_, name)| name.to_uppercase().contains("CABLE"))
            .map(|dev| dev.0.clone());
        let audio_devices_desc = AudioDevicesDesc {
            list: device_list,
            default_game_audio,
            default_microphone,
        };

        Ok(audio_devices_desc)
    }
}
#[cfg(windows)]
pub use winaudio::*;

#[derive(serde::Serialize)]
pub struct VirtualMicDevicesDesc {
    devices: Vec<String>,
    default: Option<String>,
}

pub fn virtual_mic_devices() -> StrResult<VirtualMicDevicesDesc> {
    let host = cpal::default_host();

    let devices = trace_err!(host.output_devices())?
        .filter_map(|d| d.name().ok())
        .collect::<Vec<_>>();
    let default = devices
        .iter()
        .find(|d| d.to_uppercase().contains("CABLE"))
        .cloned();

    Ok(VirtualMicDevicesDesc { devices, default })
}

// The following code is used to do a handhake between server and client to determine a common set
// of capabilities supported by both. Due to limitations of Windows WASAPI, most of this
// code is useless right now, but could still be useful for a Linux server.

fn get_audio_config_ranges(configs: Vec<SupportedStreamConfigRange>) -> Vec<AudioConfigRange> {
    configs
        .iter()
        .map(|c| {
            let buffer_sizes = if let SupportedBufferSize::Range { min, max } = c.buffer_size() {
                Some(*min..=*max)
            } else {
                None
            };

            // The 16 bit format is ubiquitously supported on Windows, but it gets misreported as
            // float 32 bit with CPAL
            #[cfg(not(windows))]
            let sample_format = if let Ok(format) = SampleFormat::from_cpal(c.sample_format()) {
                format
            } else {
                logging::show_e("Unsupported audio format");
                SampleFormat::Int16
            };
            #[cfg(windows)]
            let sample_format = SampleFormat::Int16;

            AudioConfigRange {
                channels: c.channels(),
                sample_rates: c.min_sample_rate().0..=c.max_sample_rate().0,
                buffer_sizes,
                sample_format,
            }
        })
        .collect()
}

pub fn supported_audio_input_configs() -> StrResult<Vec<AudioConfigRange>> {
    let host = cpal::default_host();
    let device = if let Some(device) = host.default_input_device() {
        device
    } else {
        return fmt_e!("No input audio device found");
    };

    let configs = get_audio_config_ranges(trace_err!(device.supported_input_configs())?.collect());

    if !configs.is_empty() {
        Ok(configs)
    } else {
        fmt_e!("No input audio configuration found")
    }
}

pub fn supported_audio_output_configs(
    device_name: Option<String>,
) -> StrResult<Vec<AudioConfigRange>> {
    let host = cpal::default_host();

    let mut maybe_device = None;
    if let (Some(name), Ok(devices)) = (device_name, host.output_devices()) {
        for device in devices {
            if let Ok(cur_name) = device.name() {
                if cur_name == name {
                    maybe_device = Some(device);
                    break;
                }
            }
        }
    }
    let device = if let Some(device) = maybe_device.or_else(|| host.default_output_device()) {
        device
    } else {
        return fmt_e!("No output audio device found");
    };

    Ok(get_audio_config_ranges(
        trace_err!(device.supported_output_configs())?.collect(),
    ))
}

pub fn select_audio_config(
    source_configs: Vec<AudioConfigRange>,
    sink_configs: Vec<AudioConfigRange>,
    preferred_config: AudioConfig,
) -> StrResult<AudioConfig> {
    let mut valid_configs = vec![];
    for source_config_range in source_configs {
        for sink_config_range in &sink_configs {
            if source_config_range.channels == sink_config_range.channels
                && source_config_range.sample_format == sink_config_range.sample_format
            {
                let channels = source_config_range.channels;
                let buffer_sizes = if let Some(source_sizes) = &source_config_range.buffer_sizes {
                    if let Some(sink_sizes) = &sink_config_range.buffer_sizes {
                        let min_size = max(source_sizes.start(), sink_sizes.start());
                        let max_size = min(source_sizes.end(), sink_sizes.end());
                        if min_size <= max_size {
                            Some(*min_size..=*max_size)
                        } else {
                            continue;
                        }
                    } else {
                        Some(source_sizes.clone())
                    }
                } else {
                    // if the buffer size of the source is unknown, then let always the source
                    // decide the buffer size.
                    None
                };
                let min_sample_rate = max(
                    source_config_range.sample_rates.start(),
                    sink_config_range.sample_rates.start(),
                );
                let max_sample_rate = min(
                    source_config_range.sample_rates.end(),
                    sink_config_range.sample_rates.end(),
                );

                if min_sample_rate <= max_sample_rate {
                    valid_configs.push(AudioConfigRange {
                        channels,
                        sample_rates: *min_sample_rate..=*min_sample_rate,
                        buffer_sizes,
                        sample_format: source_config_range.sample_format,
                    })
                }
            }
        }
    }

    let mut candidates = vec![];
    for config_range in valid_configs {
        // Scores: lower is better. Precedece: channels, sample_format, sample_rate, buffer_size

        let channels_score =
            (config_range.channels as i32 - preferred_config.channels_count as i32).abs();

        let candidate_sample_format;
        let sample_format_score;
        if config_range.sample_format == preferred_config.sample_format {
            candidate_sample_format = preferred_config.sample_format;
            sample_format_score = 0;
        } else {
            candidate_sample_format = config_range.sample_format;
            sample_format_score = u32::MAX;
        }

        let candidate_sample_rate;
        let sample_rate_score;
        if preferred_config.sample_rate >= *config_range.sample_rates.start()
            && preferred_config.sample_rate <= *config_range.sample_rates.end()
        {
            candidate_sample_rate = preferred_config.sample_rate;
            sample_rate_score = 0;
        } else {
            let min_dist_score = config_range.sample_rates.start() - preferred_config.sample_rate;
            let max_dist_score = preferred_config.sample_rate - config_range.sample_rates.end();
            if min_dist_score > max_dist_score {
                candidate_sample_rate = *config_range.sample_rates.start();
                sample_rate_score = min_dist_score;
            } else {
                candidate_sample_rate = *config_range.sample_rates.end();
                sample_rate_score = max_dist_score;
            }
        };

        let candidate_buffer_size; // can be None
        let buffer_size_score;
        if let Some(buffer_sizes) = config_range.buffer_sizes {
            if let Some(preferred_buffer_size) = preferred_config.buffer_size {
                if preferred_buffer_size >= *buffer_sizes.start()
                    && preferred_buffer_size <= *buffer_sizes.end()
                {
                    candidate_buffer_size = Some(preferred_buffer_size);
                    buffer_size_score = 0;
                } else {
                    let min_dist_score = buffer_sizes.start() - preferred_buffer_size;
                    let max_dist_score = preferred_buffer_size - buffer_sizes.end();
                    if min_dist_score > max_dist_score {
                        candidate_buffer_size = Some(*buffer_sizes.start());
                        buffer_size_score = min_dist_score;
                    } else {
                        candidate_buffer_size = Some(*buffer_sizes.end());
                        buffer_size_score = max_dist_score;
                    }
                }
            } else {
                // if no preference, choose the smallest supported buffer size
                candidate_buffer_size = Some(*buffer_sizes.start());
                buffer_size_score = 0;
            }
        } else {
            candidate_buffer_size = preferred_config.buffer_size;
            buffer_size_score = u32::MAX;
        };

        candidates.push((
            AudioConfig {
                channels_count: config_range.channels,
                sample_rate: candidate_sample_rate,
                buffer_size: candidate_buffer_size,
                sample_format: candidate_sample_format,
                max_buffer_count_extra: preferred_config.max_buffer_count_extra,
            },
            channels_score,
            sample_format_score,
            sample_rate_score,
            buffer_size_score,
        ));
    }

    candidates.sort_by(|(_, c1, sf1, sr1, bs1), (_, c2, sf2, sr2, bs2)| {
        let res = c1.cmp(c2);
        if res == Ordering::Equal {
            let res = sf1.cmp(sf2);
            if res == Ordering::Equal {
                let res = sr1.cmp(sr2);
                if res == Ordering::Equal {
                    bs1.cmp(bs2)
                } else {
                    res
                }
            } else {
                res
            }
        } else {
            res
        }
    });

    if let Some((audio_config, ..)) = candidates.into_iter().next() {
        if audio_config != preferred_config {
            warn!(
                "Specified audio settings cannot be satisfied. Using the following settings: {:?}",
                audio_config
            );
        }

        Ok(audio_config)
    } else {
        fmt_e!("No matching configuration found")
    }
}

fn audio_config_to_cpal(config: &AudioConfig) -> StreamConfig {
    StreamConfig {
        channels: config.channels_count,
        sample_rate: SampleRate(config.sample_rate),
        buffer_size: if let Some(buffer_size) = config.buffer_size {
            BufferSize::Fixed(buffer_size)
        } else {
            BufferSize::Default
        },
    }
}

pub struct AudioSession {
    _stream: Stream,
}

impl AudioSession {
    pub fn start_recording(
        device_name: Option<String>,
        config: AudioConfig,
        loopback: bool,
        sender: tmpsc::UnboundedSender<Vec<u8>>,
    ) -> StrResult<Self> {
        let host = cpal::default_host();
        let device = if let Some(device_name) = device_name {
            let devices = trace_err!(if loopback {
                host.output_devices()
            } else {
                host.input_devices()
            })?;
            let maybe_device = devices
                .filter_map(|d| Some((d.name().ok()?, d)))
                .find_map(|(d_name, d)| if d_name == device_name { Some(d) } else { None });

            if let Some(device) = maybe_device {
                device
            } else {
                return fmt_e!("Cannot find device with name \"{}\"", device_name);
            }
        } else if loopback {
            trace_none!(host.default_output_device())?
        } else {
            trace_none!(host.default_input_device())?
        };

        let stream = trace_err!(device.build_input_stream_raw(
            &audio_config_to_cpal(&config),
            config.sample_format.to_cpal(),
            move |data, _| {
                sender.send(data.bytes().to_vec()).ok();
            },
            |e| warn!("Error while recording audio: {}", e),
        ))?;

        trace_err!(stream.play())?;

        Ok(Self { _stream: stream })
    }

    pub fn start_audio_playing(
        config: AudioConfig,
        receiver: smpsc::Receiver<Vec<u8>>,
    ) -> StrResult<Self> {
        let host = cpal::default_host();
        let device = trace_none!(host.default_output_device())?;

        let mut sample_buffer = VecDeque::new();
        let frame_size =
            config.channels_count as usize * config.sample_format.to_cpal().sample_size();
        let stream = trace_err!(device.build_output_stream_raw(
            &audio_config_to_cpal(&config),
            config.sample_format.to_cpal(),
            move |data, _| {
                while let Ok(packet) = receiver.try_recv() {
                    sample_buffer.extend(packet);
                }

                let data_ref = data.bytes_mut();

                if sample_buffer.len() >= data_ref.len() {
                    data_ref.copy_from_slice(
                        &sample_buffer.drain(0..data_ref.len()).collect::<Vec<_>>(),
                    )
                }

                // trickle drain overgrown buffer. todo: use smarter policy with EventTiming
                if sample_buffer.len()
                    >= data_ref.len() * config.max_buffer_count_extra as usize + frame_size
                {
                    sample_buffer.drain(0..frame_size);
                }
            },
            |e| warn!("Error while recording audio: {}", e),
        ))?;

        trace_err!(stream.play())?;

        Ok(Self { _stream: stream })
    }
}
