use crate::*;
use cpal::traits::{DeviceTrait, HostTrait};

#[derive(serde::Serialize)]
pub struct AudioDevices {
    pub list: Vec<String>,
    pub default: Option<String>,
}

pub fn output_audio_device_names() -> StrResult<AudioDevices> {
    let host = cpal::default_host();
    let default = if let Some(device) = host.default_output_device() {
        Some(trace_err!(device.name())?)
    } else {
        None
    };

    let mut list = vec![];
    for device in trace_err!(host.output_devices())? {
        list.push(trace_err!(device.name())?)
    }

    Ok(AudioDevices { list, default })
}
