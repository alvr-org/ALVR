use alvr_common::anyhow::{bail, Context, Result};
use alvr_session::{CustomAudioDeviceConfig, MicrophoneDevicesConfig};
use cpal::traits::HostTrait;
use rodio::DeviceTrait;

use crate::{device_from_custom_config, AudioDevice};

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

    pub fn input_sample_rate(&self) -> Result<u32> {
        let config = self
            .inner
            .default_input_config()
            // On Windows, loopback devices are not recognized as input devices. Use output config.
            .or_else(|_| self.inner.default_output_config())?;

        Ok(config.sample_rate().0)
    }
}
