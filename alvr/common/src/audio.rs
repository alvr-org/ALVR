use crate::*;
// use cpal::traits::{DeviceTrait, HostTrait, DeviceTrait};
use std::ptr;
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
    pub default: Option<String>,
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

    let default = Some(device_list[0].0.clone());
    let audio_devices_desc = AudioDevicesDesc {
        list: device_list,
        default,
    };

    Ok(audio_devices_desc)
}
