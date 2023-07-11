use crate::AudioDevice;
use alvr_common::prelude::*;
use rodio::DeviceTrait;

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
pub fn set_mute_windows_device(device: &AudioDevice, mute: bool) -> StrResult {
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
