use crate::{data::*, *};
use cpal::{
    traits::{DeviceTrait, HostTrait},
    SampleFormat, SupportedBufferSize, SupportedStreamConfigRange,
};
use std::{
    cmp::{max, min, Ordering},
    ptr,
};
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

// pub fn output_audio_device_names() -> StrResult<AudioDevices> {
//     let host = cpal::default_host();
//     let default = if let Some(device) = host.default_output_device() {
//         Some(trace_err!(device.name())?)
//     } else {
//         None
//     };

//     let mut list = vec![];
//     for device in trace_err!(host.output_devices())? {
//         list.push(trace_err!(device.name())?);
//     }

//     Ok(AudioDevices { list, default })
// }

// from AudioEndPointDescriptor::GetDeviceName
fn get_device_name(mm_device: ComPtr<IMMDevice>) -> StrResult<String> {
    unsafe {
        let mut property_store_ptr: *mut IPropertyStore = ptr::null_mut();
        let hr = mm_device.OpenPropertyStore(STGM_READ, &mut property_store_ptr as _);
        if FAILED(hr) {
            return trace_str!("IMMDevice::OpenPropertyStore failed: hr = 0x{:08x}", hr);
        }
        let property_store = ComPtr::from_raw(property_store_ptr);

        let mut prop_variant = PROPVARIANT::default();
        let hr = property_store.GetValue(&PKEY_Device_FriendlyName, &mut prop_variant);
        if FAILED(hr) {
            return trace_str!("IPropertyStore::GetValue failed: hr = 0x{:08x}", hr);
        }

        if prop_variant.vt as u32 != VT_LPWSTR {
            return trace_str!(
                "PKEY_Device_FriendlyName variant type is {} - expected VT_LPWSTR",
                prop_variant.vt
            );
        }

        let res = trace_err!(U16CStr::from_ptr_str(*prop_variant.data.pwszVal()).to_string());

        let hr = PropVariantClear(&mut prop_variant);
        if FAILED(hr) {
            return trace_str!("PropVariantClear failed: hr = 0x{:08x}", hr);
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
            return trace_str!(
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
            return trace_str!("No default audio endpoint found. No audio device?");
        }
        if FAILED(hr) {
            return trace_str!(
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
            return trace_str!(
                "IMMDeviceEnumerator::EnumAudioEndpoints failed: hr = 0x{:08x}",
                hr
            );
        }
        let mm_device_collection = ComPtr::from_raw(mm_device_collection_ptr);

        #[allow(unused_mut)]
        let mut count = 0; // without mut this is UB
        let hr = mm_device_collection.GetCount(&count);
        if FAILED(hr) {
            return trace_str!("IMMDeviceCollection::GetCount failed: hr = 0x{:08x}", hr);
        }
        debug!("Active render endpoints found: {}", count);

        debug!("DefaultDevice:{} ID:{}", default_name, default_id);

        for i in 0..count {
            let mut mm_device_ptr: *mut IMMDevice = ptr::null_mut();
            let hr = mm_device_collection.Item(i, &mut mm_device_ptr as _);
            if FAILED(hr) {
                warn!("Crash!");
                return trace_str!("IMMDeviceCollection::Item failed: hr = 0x{:08x}", hr);
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

///////////////////////////////////////////////////////////////

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

fn get_audio_config_ranges(configs: Vec<SupportedStreamConfigRange>) -> Vec<AudioConfigRange> {
    configs
        .iter()
        .filter_map(|c| {
            if c.sample_format() == SampleFormat::U16 {
                let buffer_sizes = if let SupportedBufferSize::Range { min, max } = c.buffer_size()
                {
                    Some(*min..=*max)
                } else {
                    None
                };
                Some(AudioConfigRange {
                    channels: c.channels(),
                    sample_rates: c.min_sample_rate().0..=c.max_sample_rate().0,
                    buffer_sizes,
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn supported_audio_input_configs() -> StrResult<Vec<AudioConfigRange>> {
    let host = cpal::default_host();
    let device = trace_none!(host.default_input_device(), "No input audio device found")?;

    let configs = get_audio_config_ranges(trace_err!(device.supported_input_configs())?.collect());

    if !configs.is_empty() {
        Ok(configs)
    } else {
        Err("No input audio configuration found".into())
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
    let device = if let Some(device) = maybe_device {
        device
    } else {
        trace_none!(host.default_output_device(), "No output audio device found")?
    };

    let configs = get_audio_config_ranges(trace_err!(device.supported_output_configs())?.collect());

    if !configs.is_empty() {
        Ok(configs)
    } else {
        Err("No output audio configuration found".into())
    }
}

pub fn select_audio_config(
    configs1: Vec<AudioConfigRange>,
    configs2: Vec<AudioConfigRange>,
    preferred_config: AudioConfig,
) -> StrResult<AudioConfig> {
    let mut valid_configs = vec![];
    for config_range1 in configs1 {
        for config_range2 in &configs2 {
            if config_range1.channels == config_range2.channels {
                let channels = config_range1.channels;
                let buffer_sizes = if let Some(sizes1) = &config_range1.buffer_sizes {
                    if let Some(sizes2) = &config_range2.buffer_sizes {
                        let min_size = max(sizes1.start(), sizes2.start());
                        let max_size = min(sizes1.end(), sizes2.end());
                        if min_size <= max_size {
                            Some(*min_size..=*max_size)
                        } else {
                            continue;
                        }
                    } else {
                        Some(sizes1.clone())
                    }
                } else if let Some(sizes2) = &config_range2.buffer_sizes {
                    Some(sizes2.clone())
                } else {
                    None
                };
                let min_sample_rate = max(
                    config_range1.sample_rates.start(),
                    config_range2.sample_rates.start(),
                );
                let max_sample_rate = min(
                    config_range1.sample_rates.end(),
                    config_range2.sample_rates.end(),
                );

                if min_sample_rate <= max_sample_rate {
                    valid_configs.push(AudioConfigRange {
                        channels,
                        sample_rates: *min_sample_rate..=*min_sample_rate,
                        buffer_sizes,
                    })
                }
            }
        }
    }

    let mut candidates = vec![];
    for config_range in valid_configs {
        // scores: lower is better. Precedece: channels, sample_rate, buffer_size

        let channels_score =
            (config_range.channels as i32 - preferred_config.preferred_channels_count as i32).abs();

        let candidate_sample_rate;
        let sample_rate_score;
        if preferred_config.preferred_sample_rate >= *config_range.sample_rates.start()
            && preferred_config.preferred_sample_rate <= *config_range.sample_rates.end()
        {
            candidate_sample_rate = preferred_config.preferred_sample_rate;
            sample_rate_score = 0;
        } else {
            let min_dist_score =
                config_range.sample_rates.start() - preferred_config.preferred_sample_rate;
            let max_dist_score =
                preferred_config.preferred_sample_rate - config_range.sample_rates.end();
            if min_dist_score > max_dist_score {
                candidate_sample_rate = *config_range.sample_rates.start();
                sample_rate_score = min_dist_score;
            } else {
                candidate_sample_rate = *config_range.sample_rates.end();
                sample_rate_score = max_dist_score;
            }
        };

        let candidate_buffer_size;
        let buffer_size_score;
        if let Some(buffer_sizes) = config_range.buffer_sizes {
            if preferred_config.preferred_buffer_size >= *buffer_sizes.start()
                && preferred_config.preferred_buffer_size <= *buffer_sizes.end()
            {
                candidate_buffer_size = preferred_config.preferred_buffer_size;
                buffer_size_score = 0;
            } else {
                let min_dist_score = buffer_sizes.start() - preferred_config.preferred_buffer_size;
                let max_dist_score = preferred_config.preferred_buffer_size - buffer_sizes.end();
                if min_dist_score > max_dist_score {
                    candidate_buffer_size = *buffer_sizes.start();
                    buffer_size_score = min_dist_score;
                } else {
                    candidate_buffer_size = *buffer_sizes.end();
                    buffer_size_score = max_dist_score;
                }
            }
        } else {
            candidate_buffer_size = preferred_config.preferred_buffer_size;
            buffer_size_score = u32::MAX;
        };

        candidates.push((
            AudioConfig {
                preferred_channels_count: config_range.channels,
                preferred_sample_rate: candidate_sample_rate,
                preferred_buffer_size: candidate_buffer_size,
            },
            channels_score,
            sample_rate_score,
            buffer_size_score,
        ));
    }

    candidates.sort_by(|(_, c1, sr1, bs1), (_, c2, sr2, bs2)| {
        let res = c1.cmp(c2);
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
    });

    if let Some((audio_config, ..)) = candidates.into_iter().next() {
        Ok(audio_config)
    } else {
        Err("No matching configuration found".into())
    }
}

async fn start_recording(config: AudioConfig, loopback: bool) -> StrResult {
    todo!();
}

async fn start_playing(config: AudioConfig) -> StrResult {
    todo!();
}
