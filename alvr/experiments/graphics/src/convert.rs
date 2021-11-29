use alvr_common::{glam::UVec2, prelude::*};
use ash::{extensions::khr, vk};
use core::sync;
use openxr_sys::SwapchainUsageFlags;
use std::{any::Any, ffi::CStr, slice, sync::Arc};
use wgpu::{
    Device, DeviceDescriptor, Extent3d, Features, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};
use wgpu_hal as hal;

use crate::GraphicsContext;

pub const TARGET_VULKAN_VERSION: u32 = vk::make_api_version(1, 0, 0, 0);
pub const DEVICE_FEATURES: Features = Features::PUSH_CONSTANTS;

// Get extensions needed by wgpu. Corresponds to xrGetVulkanInstanceExtensionsKHR
pub fn get_vulkan_instance_extensions(
    entry: &ash::Entry,
    version: u32,
) -> StrResult<Vec<&'static CStr>> {
    let mut flags = hal::InstanceFlags::empty();
    if cfg!(debug_assertions) {
        flags |= hal::InstanceFlags::VALIDATION;
        flags |= hal::InstanceFlags::DEBUG;
    }

    trace_err!(<hal::api::Vulkan as hal::Api>::Instance::required_extensions(entry, version, flags))
}

// Create wgpu-compatible Vulkan instance. Corresponds to xrCreateVulkanInstanceKHR
pub fn create_vulkan_instance(
    entry: &ash::Entry,
    info: &vk::InstanceCreateInfo,
) -> StrResult<ash::Instance> {
    let mut extensions_ptrs =
        get_vulkan_instance_extensions(entry, unsafe { (*info.p_application_info).api_version })?
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

    extensions_ptrs.extend_from_slice(unsafe {
        slice::from_raw_parts(
            info.pp_enabled_extension_names,
            info.enabled_extension_count as _,
        )
    });

    unsafe {
        trace_err!(entry.create_instance(
            &vk::InstanceCreateInfo {
                enabled_extension_count: extensions_ptrs.len() as _,
                pp_enabled_extension_names: extensions_ptrs.as_ptr(),
                ..*info
            },
            None,
        ))
    }
}

// Corresponds to xrGetVulkanGraphicsDeviceKHR
pub fn get_vulkan_graphics_device(
    instance: &ash::Instance,
    adapter_index: Option<usize>,
) -> StrResult<vk::PhysicalDevice> {
    let mut physical_devices = unsafe { trace_err!(instance.enumerate_physical_devices())? };

    Ok(physical_devices.remove(adapter_index.unwrap_or(0)))
}

// Hal adapter used to get required device extensions and features
pub fn get_temporary_hal_adapter(
    entry: ash::Entry,
    version: u32,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> StrResult<<hal::api::Vulkan as hal::Api>::Adapter> {
    let instance_extensions = get_vulkan_instance_extensions(&entry, version)?;

    let mut flags = hal::InstanceFlags::empty();
    if cfg!(debug_assertions) {
        flags |= hal::InstanceFlags::VALIDATION;
        flags |= hal::InstanceFlags::DEBUG;
    };

    let hal_instance = unsafe {
        trace_err!(<hal::api::Vulkan as hal::Api>::Instance::from_raw(
            entry,
            instance,
            version,
            instance_extensions,
            flags,
            false,
            None, // <-- the instance is not destroyed on drop
        ))?
    };

    let exposed_adapter = trace_none!(hal_instance.expose_adapter(physical_device))?;

    Ok(exposed_adapter.adapter)
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

    let mut extensions_ptrs = temp_adapter
        .required_device_extensions(DEVICE_FEATURES)
        .iter()
        .map(|x| x.as_ptr())
        .collect::<Vec<_>>();

    extensions_ptrs.extend_from_slice(unsafe {
        slice::from_raw_parts(
            create_info.pp_enabled_extension_names,
            create_info.enabled_extension_count as _,
        )
    });

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
        trace_err!(instance.create_device(
            physical_device,
            &vk::DeviceCreateInfo {
                enabled_extension_count: extensions_ptrs.len() as _,
                pp_enabled_extension_names: extensions_ptrs.as_ptr(),
                p_enabled_features: &features as *const _,
                ..*create_info
            },
            None
        ))
    }
}

impl GraphicsContext {
    // This constructor is used primarily for the vulkan layer. It corresponds to xrCreateSession
    // with GraphicsBindingVulkanKHR. If owned == false, this Context must be dropped before
    // destroying vk_instance and vk_device.
    pub fn from_vulkan(
        entry: ash::Entry,
        version: u32,
        raw_instance: ash::Instance,
        raw_physical_device: vk::PhysicalDevice,
        raw_device: ash::Device,
        queue_family_index: u32,
        queue_index: u32,
        drop_guard: Option<Box<dyn Any + Send + Sync>>,
    ) -> StrResult<Self> {
        let mut flags = hal::InstanceFlags::empty();
        if cfg!(debug_assertions) {
            flags |= hal::InstanceFlags::VALIDATION;
            flags |= hal::InstanceFlags::DEBUG;
        };

        let extensions = get_vulkan_instance_extensions(&entry, version)?;

        let handle_is_owned = drop_guard.is_some();

        let instance = unsafe {
            trace_err!(<hal::api::Vulkan as hal::Api>::Instance::from_raw(
                entry,
                raw_instance.clone(),
                version,
                extensions,
                flags,
                false,
                drop_guard,
            ))?
        };

        let exposed_adapter = trace_none!(instance.expose_adapter(raw_physical_device))?;
        let device_extensions = exposed_adapter
            .adapter
            .required_device_extensions(DEVICE_FEATURES);

        let open_device = unsafe {
            trace_err!(exposed_adapter.adapter.device_from_raw(
                raw_device.clone(),
                handle_is_owned,
                &device_extensions,
                hal::UpdateAfterBindTypes::empty(), // todo: proper initialization
                queue_family_index,
                queue_index,
            ))?
        };

        #[cfg(not(target_os = "macos"))]
        {
            let instance = unsafe { wgpu::Instance::from_hal::<hal::api::Vulkan>(instance) };
            let adapter = unsafe { instance.create_adapter_from_hal(exposed_adapter) };
            let (device, queue) = unsafe {
                trace_err!(adapter.create_device_from_hal(
                    open_device,
                    &DeviceDescriptor {
                        label: None,
                        features: DEVICE_FEATURES,
                        limits: adapter.limits(),
                    },
                    None,
                ))?
            };

            Ok(Self {
                instance: Arc::new(instance),
                adapter: Arc::new(adapter),
                device: Arc::new(device),
                queue: Arc::new(queue),
                raw_instance,
                raw_physical_device,
                raw_device,
                queue_family_index,
                queue_index,
            })
        }

        #[cfg(target_os = "macos")]
        unimplemented!()
    }

    // This constructor is used for the Windows OpenVR driver
    pub fn new(adapter_index: Option<usize>) -> StrResult<Self> {
        let entry = unsafe { trace_err!(ash::Entry::new())? };

        let raw_instance = trace_err!(create_vulkan_instance(
            &entry,
            &vk::InstanceCreateInfo::builder()
                .application_info(
                    &vk::ApplicationInfo::builder().api_version(TARGET_VULKAN_VERSION)
                )
                .build()
        ))?;

        let raw_physical_device = get_vulkan_graphics_device(&raw_instance, adapter_index)?;

        let queue_family_index = unsafe {
            raw_instance
                .get_physical_device_queue_family_properties(raw_physical_device)
                .into_iter()
                .enumerate()
                .find_map(|(queue_family_index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        Some(queue_family_index as u32)
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let queue_index = 0;

        let raw_device = trace_err!(create_vulkan_device(
            entry.clone(),
            TARGET_VULKAN_VERSION,
            &raw_instance,
            raw_physical_device,
            &vk::DeviceCreateInfo::builder().queue_create_infos(&[
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&[1.0])
                    .build()
            ])
        ))?;

        Self::from_vulkan(
            entry,
            TARGET_VULKAN_VERSION,
            raw_instance,
            raw_physical_device,
            raw_device,
            queue_family_index,
            queue_index,
            Some(Box::new(())),
        )
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
        vk_usage: vk::ImageUsageFlags,
        vk_format: vk::Format,
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
    pub usage: SwapchainUsageFlags,
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
    let wgpu_usage = {
        let mut wgpu_usage = TextureUsages::TEXTURE_BINDING; // Always required

        if info.usage.contains(SwapchainUsageFlags::COLOR_ATTACHMENT) {
            wgpu_usage |= TextureUsages::RENDER_ATTACHMENT;
        }
        if info
            .usage
            .contains(SwapchainUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            wgpu_usage |= TextureUsages::RENDER_ATTACHMENT;
        }
        if info.usage.contains(SwapchainUsageFlags::TRANSFER_SRC) {
            wgpu_usage |= TextureUsages::COPY_SRC;
        }
        if info.usage.contains(SwapchainUsageFlags::TRANSFER_DST) {
            wgpu_usage |= TextureUsages::COPY_DST;
        }

        // Unused flags:
        // * UNORDERED_ACCESS: No-op on vulkan
        // * SAMPLED: Already required
        // * MUTABLE_FORMAT: wgpu does not support this, but it should be no-op for the internal
        //   Vulkan images (todo: check)
        // * INPUT_ATTACHMENT: Always enabled on wgpu (todo: check)

        wgpu_usage
    };

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
        usage: wgpu_usage,
    };

    match data {
        SwapchainCreateData::External {
            images,
            vk_usage,
            vk_format,
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
