use crate::{GraphicsContext, QueuesFamilyInfo, RawGraphicsHandles, VideoQueueFamilyInfo};
use alvr_common::{glam::UVec2, prelude::*};
use ash::vk;
use std::{any::Any, ffi::CStr, slice, sync::Arc};
use wgpu::{
    Device, DeviceDescriptor, Extent3d, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};
use wgpu_hal as hal;

pub const TARGET_VULKAN_VERSION: u32 = vk::make_api_version(0, 1, 1, 0);

// Get extensions needed by wgpu. Corresponds to xrGetVulkanInstanceExtensionsKHR
pub fn get_vulkan_instance_extensions(entry: &ash::Entry) -> StrResult<Vec<&'static CStr>> {
    let mut flags = hal::InstanceFlags::empty();
    if cfg!(debug_assertions) {
        flags |= hal::InstanceFlags::VALIDATION;
        flags |= hal::InstanceFlags::DEBUG;
    }

    <hal::api::Vulkan as hal::Api>::Instance::required_extensions(entry, flags).map_err(err!())
}

// Create wgpu-compatible Vulkan instance. Corresponds to xrCreateVulkanInstanceKHR
pub fn create_vulkan_instance(
    entry: &ash::Entry,
    info: &vk::InstanceCreateInfo,
) -> StrResult<ash::Instance> {
    let mut extensions_ptrs = get_vulkan_instance_extensions(entry)?
        .iter()
        .map(|x| x.as_ptr())
        .collect::<Vec<_>>();

    extensions_ptrs.extend_from_slice(unsafe {
        slice::from_raw_parts(
            info.pp_enabled_extension_names,
            info.enabled_extension_count as _,
        )
    });

    let layers = vec![CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()];
    let layers_ptrs = layers.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();

    unsafe {
        entry
            .create_instance(
                &vk::InstanceCreateInfo {
                    enabled_extension_count: extensions_ptrs.len() as _,
                    pp_enabled_extension_names: extensions_ptrs.as_ptr(),
                    enabled_layer_count: layers_ptrs.len() as _,
                    pp_enabled_layer_names: layers_ptrs.as_ptr(),
                    ..*info
                },
                None,
            )
            .map_err(err!())
    }
}

// Corresponds to xrGetVulkanGraphicsDeviceKHR
pub fn get_vulkan_graphics_device(
    instance: &ash::Instance,
    adapter_index: Option<usize>,
) -> StrResult<vk::PhysicalDevice> {
    let mut physical_devices = unsafe { instance.enumerate_physical_devices().map_err(err!())? };

    Ok(physical_devices.remove(adapter_index.unwrap_or(0)))
}

// Hal adapter used to get required device extensions and features
pub fn get_temporary_hal_adapter(
    entry: ash::Entry,
    version: u32,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> StrResult<hal::ExposedAdapter<hal::api::Vulkan>> {
    let instance_extensions = get_vulkan_instance_extensions(&entry)?;

    let mut flags = hal::InstanceFlags::empty();
    if cfg!(debug_assertions) {
        flags |= hal::InstanceFlags::VALIDATION;
        flags |= hal::InstanceFlags::DEBUG;
    };

    let hal_instance = unsafe {
        <hal::api::Vulkan as hal::Api>::Instance::from_raw(
            entry,
            instance,
            version,
            instance_extensions,
            flags,
            false,
            None, // <-- the instance is not destroyed on drop
        )
        .map_err(err!())?
    };

    hal_instance
        .expose_adapter(physical_device)
        .ok_or_else(enone!())
}

pub fn get_vulkan_queue_families_info(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> QueuesFamilyInfo {
    let size =
        unsafe { instance.get_physical_device_queue_family_properties2_len(physical_device) };

    let mut queue_family_properties = vec![
        vk::QueueFamilyProperties2::builder()
            .push_next(&mut vk::VideoQueueFamilyProperties2KHR::default())
            .build();
        size
    ];
    unsafe {
        instance.get_physical_device_queue_family_properties2(
            physical_device,
            &mut queue_family_properties,
        )
    };

    fn get_queue_family_info(
        props: &[vk::QueueFamilyProperties2],
        queue_type: vk::QueueFlags,
        codec_op: vk::VideoCodecOperationFlagsKHR,
    ) -> Vec<VideoQueueFamilyInfo> {
        props
            .iter()
            .enumerate()
            .filter_map(|(index, info)| unsafe {
                if info
                    .queue_family_properties
                    .queue_flags
                    .contains(queue_type)
                    && (*(info.p_next as *mut vk::VideoQueueFamilyProperties2KHR))
                        .video_codec_operations
                        .contains(codec_op)
                {
                    Some(VideoQueueFamilyInfo {
                        index: index as _,
                        queues_count: info.queue_family_properties.queue_count as _,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    let rendering_index = get_queue_family_info(
        &queue_family_properties,
        vk::QueueFlags::GRAPHICS,
        vk::VideoCodecOperationFlagsKHR::INVALID,
    )[0]
    .index;

    // NB: we don't care if this overlaps with the rendering queue family index, since the rendering
    // and encode operations never overlap because of resource management
    let h264_encoding_queue_family_info = get_queue_family_info(
        &queue_family_properties,
        vk::QueueFlags::VIDEO_ENCODE_KHR,
        vk::VideoCodecOperationFlagsKHR::ENCODE_H264_EXT,
    );
    let h265_encoding_queue_family_info = get_queue_family_info(
        &queue_family_properties,
        vk::QueueFlags::VIDEO_ENCODE_KHR,
        vk::VideoCodecOperationFlagsKHR::ENCODE_H265_EXT,
    );
    let h264_decoding_queue_family_info = get_queue_family_info(
        &queue_family_properties,
        vk::QueueFlags::VIDEO_ENCODE_KHR,
        vk::VideoCodecOperationFlagsKHR::DECODE_H264_EXT,
    );
    let h265_decoding_queue_family_info = get_queue_family_info(
        &queue_family_properties,
        vk::QueueFlags::VIDEO_ENCODE_KHR,
        vk::VideoCodecOperationFlagsKHR::DECODE_H265_EXT,
    );

    QueuesFamilyInfo {
        rendering_index,
        h264_encoding_queue_family_info,
        h265_encoding_queue_family_info,
        h264_decoding_queue_family_info,
        h265_decoding_queue_family_info,
    }
}

// Create wgpu-compatible Vulkan device. Corresponds to xrCreateVulkanDeviceKHR
pub fn create_vulkan_device(
    entry: ash::Entry,
    version: u32,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    create_info: &vk::DeviceCreateInfo,
) -> StrResult<ash::Device> {
    let temp_adapter =
        get_temporary_hal_adapter(entry, version, instance.clone(), physical_device)?;

    let wgpu_extensions = temp_adapter
        .adapter
        .required_device_extensions(temp_adapter.features);
    let mut extensions_ptrs = wgpu_extensions
        .iter()
        .map(|x| x.as_ptr())
        .collect::<Vec<_>>();
    let mut enabled_phd_features = temp_adapter.adapter.physical_device_features(
        &wgpu_extensions,
        temp_adapter.features,
        hal::UpdateAfterBindTypes::empty(),
    );

    extensions_ptrs.extend_from_slice(unsafe {
        slice::from_raw_parts(
            create_info.pp_enabled_extension_names,
            create_info.enabled_extension_count as _,
        )
    });

    let temp_create_info = vk::DeviceCreateInfo::builder();
    let temp_create_info = enabled_phd_features.add_to_device_create_builder(temp_create_info);
    // todo

    // todo: add required wgpu features from temp_adapter
    let mut features = if !create_info.p_enabled_features.is_null() {
        unsafe { *create_info.p_enabled_features }
    } else {
        vk::PhysicalDeviceFeatures::default()
    };
    features.robust_buffer_access = true as _;
    features.independent_blend = true as _;
    features.sample_rate_shading = true as _;

    unsafe {
        instance
            .create_device(
                physical_device,
                &vk::DeviceCreateInfo {
                    enabled_extension_count: extensions_ptrs.len() as _,
                    pp_enabled_extension_names: extensions_ptrs.as_ptr(),
                    p_enabled_features: &features as *const _,
                    ..*create_info
                },
                None,
            )
            .map_err(err!())
    }
}

pub struct GraphicsContextVulkanInitDesc {
    pub entry: ash::Entry,
    pub version: u32,
    pub raw: RawGraphicsHandles,
    pub drop_guard: Option<Box<dyn Any + Send + Sync>>,
}

impl GraphicsContext {
    // This constructor is used primarily for the vulkan layer. It corresponds to xrCreateSession
    // with GraphicsBindingVulkanKHR. If owned == false, this Context must be dropped before
    // destroying vk_instance and vk_device.
    pub fn from_vulkan(desc: GraphicsContextVulkanInitDesc) -> StrResult<Self> {
        let mut flags = hal::InstanceFlags::empty();
        if cfg!(debug_assertions) {
            flags |= hal::InstanceFlags::VALIDATION;
            flags |= hal::InstanceFlags::DEBUG;
        };

        let instance_extensions = get_vulkan_instance_extensions(&desc.entry)?;

        let handle_is_owned = desc.drop_guard.is_some();

        let instance = unsafe {
            <hal::api::Vulkan as hal::Api>::Instance::from_raw(
                desc.entry,
                desc.raw.instance.clone(),
                desc.version,
                instance_extensions,
                flags,
                false,
                desc.drop_guard,
            )
            .map_err(err!())?
        };

        let exposed_adapter = instance
            .expose_adapter(desc.raw.physical_device)
            .ok_or_else(enone!())?;
        let device_extensions = exposed_adapter
            .adapter
            .required_device_extensions(exposed_adapter.features);

        let open_device = unsafe {
            exposed_adapter
                .adapter
                .device_from_raw(
                    desc.raw.device.clone(),
                    handle_is_owned,
                    &device_extensions,
                    exposed_adapter.features,
                    hal::UpdateAfterBindTypes::empty(), // todo: proper initialization
                    desc.raw.queues_family_info.rendering_index,
                    desc.raw.rendering_queue_index,
                )
                .map_err(err!())?
        };

        #[cfg(not(target_os = "macos"))]
        {
            let instance = unsafe { wgpu::Instance::from_hal::<hal::api::Vulkan>(instance) };
            let adapter = unsafe { instance.create_adapter_from_hal(exposed_adapter) };
            let (device, queue) = unsafe {
                adapter
                    .create_device_from_hal(
                        open_device,
                        &DeviceDescriptor {
                            label: None,
                            features: adapter.features(),
                            limits: adapter.limits(),
                        },
                        None,
                    )
                    .map_err(err!())?
            };

            Ok(Self {
                instance: Arc::new(instance),
                adapter: Arc::new(adapter),
                device: Arc::new(device),
                queue: Arc::new(queue),
                raw_instance: desc.raw_instance,
                raw_physical_device: desc.raw_physical_device,
                raw_device: desc.raw_device,
                queue_family_index: desc.queue_family_index,
                queue_index: desc.queue_index,
            })
        }

        #[cfg(target_os = "macos")]
        unimplemented!()
    }
}

#[cfg(not(target_os = "macos"))]
pub fn to_vulkan_images(textures: &[Texture]) -> Vec<vk::Image> {
    textures
        .iter()
        .map(|tex| unsafe {
            let mut handle = vk::Image::null();
            tex.as_hal::<hal::api::Vulkan, _>(|tex| {
                handle = tex.unwrap().raw_handle();
            });

            handle
        })
        .collect()
}

pub enum SwapchainCreateData {
    // Used for the Vulkan layer and client
    External {
        images: Vec<vk::Image>,
        hal_usage: hal::TextureUses,
        drop_guard: Option<Arc<dyn Any + Send + Sync>>,
    },

    // Used for the Windows OpenVR driver (Some) or for a OpenXR runtime (None)
    Count(Option<usize>),
}

pub enum TextureType {
    Flat { array_size: u32 },
    Cubemap, // for now cubemaps cannot be used in the compositor
}

pub struct SwapchainCreateInfo {
    pub usage: TextureUsages,
    pub format: TextureFormat,
    pub sample_count: u32,
    pub size: UVec2,
    pub texture_type: TextureType,
    pub mip_count: u32,
}

pub fn create_texture_set(
    device: &Device,
    data: SwapchainCreateData,
    info: SwapchainCreateInfo,
) -> Vec<Texture> {
    let depth_or_array_layers = match info.texture_type {
        TextureType::Flat { array_size } => array_size,
        TextureType::Cubemap => 6,
    };

    let size = Extent3d {
        width: info.size.x,
        height: info.size.y,
        depth_or_array_layers,
    };

    let texture_descriptor = TextureDescriptor {
        label: None,
        size,
        mip_level_count: info.mip_count,
        sample_count: info.sample_count,
        dimension: TextureDimension::D2,
        format: info.format,
        usage: info.usage,
    };

    match data {
        SwapchainCreateData::External {
            images,
            hal_usage,
            drop_guard,
        } => images
            .into_iter()
            .map(|vk_image| {
                let hal_texture = unsafe {
                    <hal::api::Vulkan as hal::Api>::Device::texture_from_raw(
                        vk_image,
                        &hal::TextureDescriptor {
                            label: None,
                            size,
                            mip_level_count: info.mip_count,
                            sample_count: info.sample_count,
                            dimension: TextureDimension::D2,
                            format: info.format,
                            usage: hal_usage,
                            memory_flags: hal::MemoryFlags::empty(),
                        },
                        drop_guard.clone().map(|guard| Box::new(guard) as _),
                    )
                };

                #[cfg(not(target_os = "macos"))]
                unsafe {
                    device.create_texture_from_hal::<hal::api::Vulkan>(
                        hal_texture,
                        &texture_descriptor,
                    )
                }
                #[cfg(target_os = "macos")]
                unimplemented!()
            })
            .collect(),
        SwapchainCreateData::Count(count) => (0..count.unwrap_or(2))
            .map(|_| device.create_texture(&texture_descriptor))
            .collect(),
    }
}
