use crate::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, Sample, SampleFormat, Stream, StreamConfig,
};
use std::{collections::VecDeque, sync::mpsc as smpsc};
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
pub fn set_mute_audio_device(device_index: Option<u64>, mute: bool) -> StrResult {
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

pub fn get_vb_cable_audio_device_index() -> StrResult<Option<u64>> {
    let host = cpal::default_host();

    Ok(trace_err!(host.output_devices())?
        .enumerate()
        .filter_map(|(i, d)| Some((i as _, d.name().ok()?)))
        .filter(|(_, name)| name.contains("CABLE Input"))
        .map(|(i, _)| i)
        .next())
}

fn get_output_audio_device(device_index: Option<u64>) -> StrResult<Device> {
    let host = cpal::default_host();

    if let Some(index) = device_index {
        if let Some(device) = trace_err!(host.output_devices())?.nth(index as _) {
            Ok(device)
        } else {
            fmt_e!("Cannot find audio device at index {}", index)
        }
    } else if let Some(device) = host.default_output_device() {
        Ok(device)
    } else {
        fmt_e!("No output audio device found")
    }
}

pub fn get_output_sample_rate(device_index: Option<u64>) -> StrResult<u32> {
    let device = trace_err!(get_output_audio_device(device_index))?;

    let mut configs = trace_err!(device.supported_output_configs())?;

    // Assumption: device is in shared mode: this means that sample rate, format and channel count
    // is fixed
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

pub struct AudioSession {
    _stream: Stream,
}

impl AudioSession {
    pub fn start_recording(
        device_index: Option<u64>,
        loopback: bool,
        channels_count: u16,
        sample_rate: u32,
        sender: tmpsc::UnboundedSender<Vec<u8>>,
    ) -> StrResult<Self> {
        let host = cpal::default_host();
        let device = if let Some(index) = device_index {
            let mut devices = trace_err!(if loopback {
                host.output_devices()
            } else {
                host.input_devices()
            })?;

            if let Some(device) = devices.nth(index as _) {
                device
            } else {
                return fmt_e!("Cannot find audio device at index {}", index);
            }
        } else if loopback {
            trace_none!(host.default_output_device())?
        } else {
            trace_none!(host.default_input_device())?
        };

        let config = trace_none!(trace_err!(device.supported_output_configs())?.next())?;

        if sample_rate != config.min_sample_rate().0 {
            return fmt_e!("Sample rate not supported");
        }

        let stream_config = StreamConfig {
            channels: config.channels(),
            sample_rate: config.min_sample_rate(),
            buffer_size: BufferSize::Default,
        };

        let stream = trace_err!(device.build_input_stream_raw(
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
                sender.send(data).ok();
            },
            |e| warn!("Error while recording audio: {}", e),
        ))?;

        trace_err!(stream.play())?;

        Ok(Self { _stream: stream })
    }

    pub fn start_playing(
        device_index: Option<u64>,
        channels_count: u16,
        sample_rate: u32,
        buffer_range_multiplier: u64,
        receiver: smpsc::Receiver<Vec<u8>>,
    ) -> StrResult<Self> {
        let device = trace_err!(get_output_audio_device(device_index))?;

        let config = trace_none!(trace_err!(device.supported_output_configs())?.next())?;

        if sample_rate != config.min_sample_rate().0 {
            return fmt_e!("Sample rate not supported");
        }

        let stream_config = StreamConfig {
            channels: config.channels(),
            sample_rate: config.min_sample_rate(),
            buffer_size: BufferSize::Default,
        };

        let mut sample_buffer_bytes = VecDeque::new();
        let stream = trace_err!(device.build_output_stream_raw(
            &stream_config,
            config.sample_format(),
            move |data, _| {
                while let Ok(packet) = receiver.try_recv() {
                    let mut data =
                        convert_channels_count(&packet, channels_count, config.channels());
                    if config.sample_format() == SampleFormat::F32 {
                        data = data
                            .chunks_exact(2)
                            .flat_map(|c| {
                                i16::from_ne_bytes([c[0], c[1]])
                                    .to_f32()
                                    .to_ne_bytes()
                                    .to_vec()
                            })
                            .collect()
                    };
                    sample_buffer_bytes.extend(data);
                }

                let data_bytes_len = data.bytes().len();
                if sample_buffer_bytes.len() >= data_bytes_len {
                    data.bytes_mut().copy_from_slice(
                        &sample_buffer_bytes
                            .drain(0..data_bytes_len)
                            .collect::<Vec<_>>(),
                    )
                } else {
                    warn!("audio buffer too small! size: {}", sample_buffer_bytes.len());
                }

                // todo: use smarter policy with EventTiming
                if sample_buffer_bytes.len() > 2 * buffer_range_multiplier as usize * data_bytes_len
                {
                    warn!("draining audio buffer. size: {}", sample_buffer_bytes.len());

                    sample_buffer_bytes.drain(
                        0..(sample_buffer_bytes.len()
                            - buffer_range_multiplier as usize * data_bytes_len),
                    );
                }
            },
            |e| warn!("Error while playing audio: {}", e),
        ))?;

        trace_err!(stream.play())?;

        Ok(Self { _stream: stream })
    }
}
