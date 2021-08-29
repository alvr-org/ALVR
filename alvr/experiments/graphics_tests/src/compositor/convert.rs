use super::{Compositor, Context, Swapchain};
use alvr_common::prelude::*;
use ash::{extensions::khr, vk};
use openxr_sys as sys;
use std::{ffi::CStr, slice};
use wgpu::{
    DeviceDescriptor, Extent3d, Features, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};
use wgpu_hal as hal;

pub const TARGET_VULKAN_VERSION: u32 = vk::make_api_version(1, 0, 0, 0);

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

// Corresponds to xrGetVulkanDeviceExtensionsKHR. Copied from wgpu.
// Wgpu could need more extensions in future versions. Some extensions should be conditionally
// enabled depending on the instance. todo: get directly from wgpu adapter (this can be achieved by
// keeping track of the instance using a map with the physical device as key)
pub fn get_vulkan_device_extensions(version: u32) -> Vec<&'static CStr> {
    let mut extensions = vec![khr::Swapchain::name()];

    if version < vk::API_VERSION_1_1 {
        extensions.push(vk::KhrMaintenance1Fn::name());
        extensions.push(vk::KhrMaintenance2Fn::name());
    }

    extensions
}

// Create wgpu-compatible Vulkan device. Corresponds to xrCreateVulkanDeviceKHR
pub fn create_vulkan_device(
    entry: &ash::Entry,
    version: u32,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    create_info: &vk::DeviceCreateInfo,
) -> StrResult<ash::Device> {
    let mut extensions_ptrs = get_vulkan_device_extensions(version)
        .iter()
        .map(|x| x.as_ptr())
        .collect::<Vec<_>>();

    extensions_ptrs.extend_from_slice(unsafe {
        slice::from_raw_parts(
            create_info.pp_enabled_extension_names,
            create_info.enabled_extension_count as _,
        )
    });

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

impl Context {
    // This constructor is used primarily for the vulkan layer. It corresponds to xrCreateSession
    // with GraphicsBindingVulkanKHR. If owned == false, this Context must be dropped before
    // destroying vk_instance and vk_device.
    pub fn from_vulkan(
        owned: bool, // should wgpu be in change of destrying the vulkan objects
        entry: ash::Entry,
        version: u32,
        vk_instance: ash::Instance,
        adapter_index: Option<usize>,
        vk_device: ash::Device,
        queue_family_index: u32,
        queue_index: u32,
    ) -> StrResult<Self> {
        let mut flags = hal::InstanceFlags::empty();
        if cfg!(debug_assertions) {
            flags |= hal::InstanceFlags::VALIDATION;
            flags |= hal::InstanceFlags::DEBUG;
        };

        let extensions = get_vulkan_instance_extensions(&entry, version)?;

        let instance = unsafe {
            trace_err!(<hal::api::Vulkan as hal::Api>::Instance::from_raw(
                entry,
                vk_instance.clone(),
                version,
                extensions,
                flags,
                owned.then(|| Box::new(()) as _)
            ))?
        };

        let physical_device = get_vulkan_graphics_device(&vk_instance, adapter_index)?;
        let exposed_adapter = trace_none!(instance.expose_adapter(physical_device))?;

        let open_device = unsafe {
            trace_err!(exposed_adapter.adapter.device_from_raw(
                vk_device,
                owned,
                &get_vulkan_device_extensions(version),
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
                        features: Features::PUSH_CONSTANTS,
                        limits: adapter.limits(),
                    },
                    None,
                ))?
            };

            Ok(Self {
                instance,
                device,
                queue,
            })
        }

        #[cfg(target_os = "macos")]
        unimplemented!()
    }

    // This constructor is used for the Windows OpenVR driver
    pub fn new(adapter_index: Option<usize>) -> StrResult<Self> {
        let entry = unsafe { trace_err!(ash::Entry::new())? };

        let vk_instance = trace_err!(create_vulkan_instance(
            &entry,
            &vk::InstanceCreateInfo::builder()
                .application_info(
                    &vk::ApplicationInfo::builder().api_version(TARGET_VULKAN_VERSION)
                )
                .build()
        ))?;

        let physical_device = get_vulkan_graphics_device(&vk_instance, adapter_index)?;

        let queue_family_index = unsafe {
            vk_instance
                .get_physical_device_queue_family_properties(physical_device)
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

        let vk_device = trace_err!(create_vulkan_device(
            &entry,
            TARGET_VULKAN_VERSION,
            &vk_instance,
            physical_device,
            &vk::DeviceCreateInfo::builder().queue_create_infos(&[
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&[1.0])
                    .build()
            ])
        ))?;

        Self::from_vulkan(
            true,
            entry,
            TARGET_VULKAN_VERSION,
            vk_instance,
            adapter_index,
            vk_device,
            queue_family_index,
            queue_index,
        )
    }
}

pub enum SwapchainCreateData {
    // Used for the Vulkan layer
    #[cfg(target_os = "linux")]
    External {
        images: Vec<vk::Image>,
        vk_usage: vk::ImageUsageFlags,
        vk_format: vk::Format,
        hal_usage: hal::TextureUses,
    },

    // Used for the Windows OpenVR driver
    Count(usize),

    // Used for a OpenXR runtime
    None,
}

impl Compositor {
    // corresponds to xrCreateSwapchain
    pub fn create_swapchain(
        &self,
        data: SwapchainCreateData,
        usage: openxr_sys::SwapchainUsageFlags,
        format: TextureFormat,
        sample_count: u32,
        width: u32,
        height: u32,
        // cubemap: bool,
        array_size: u32,
        mip_count: u32,
    ) -> StrResult<Swapchain> {
        let wgpu_usage = {
            let mut wgpu_usage = TextureUsages::empty();

            if usage.contains(sys::SwapchainUsageFlags::COLOR_ATTACHMENT) {
                wgpu_usage |= TextureUsages::RENDER_ATTACHMENT;
            }
            if usage.contains(sys::SwapchainUsageFlags::DEPTH_STENCIL_ATTACHMENT) {
                wgpu_usage |= TextureUsages::RENDER_ATTACHMENT;
            }
            if usage.contains(sys::SwapchainUsageFlags::UNORDERED_ACCESS) {
                // ?
            }
            if usage.contains(sys::SwapchainUsageFlags::TRANSFER_SRC) {
                wgpu_usage |= TextureUsages::COPY_SRC;
            }
            if usage.contains(sys::SwapchainUsageFlags::TRANSFER_DST) {
                wgpu_usage |= TextureUsages::COPY_DST;
            }
            if usage.contains(sys::SwapchainUsageFlags::SAMPLED) {
                wgpu_usage |= TextureUsages::TEXTURE_BINDING;
            }
            if usage.contains(sys::SwapchainUsageFlags::MUTABLE_FORMAT) {
                // ?
            }
            if usage.contains(sys::SwapchainUsageFlags::INPUT_ATTACHMENT) {
                // ?
            }

            wgpu_usage
        };

        let texture_descriptor = TextureDescriptor {
            label: None,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: array_size,
            },
            mip_level_count: mip_count,
            sample_count,
            dimension: TextureDimension::D2,
            format,
            usage: wgpu_usage,
        };

        let textures = match data {
            #[cfg(target_os = "linux")]
            SwapchainCreateData::External {
                images,
                vk_usage,
                vk_format,
                hal_usage,
            } => images
                .into_iter()
                .map(|vk_image| {
                    let hal_texture = unsafe {
                        <hal::api::Vulkan as hal::Api>::Device::texture_from_raw(
                            vk_image,
                            &hal::TextureDescriptor {
                                label: None,
                                size: Extent3d {
                                    width,
                                    height,
                                    depth_or_array_layers: array_size,
                                },
                                mip_level_count: mip_count,
                                sample_count,
                                dimension: TextureDimension::D2,
                                format,
                                usage: hal_usage,
                                memory_flags: hal::MemoryFlags::empty(),
                            },
                            None,
                        )
                    };

                    unsafe {
                        self.context
                            .device
                            .create_texture_from_hal::<hal::api::Vulkan>(
                                hal_texture,
                                &texture_descriptor,
                            )
                    }
                })
                .collect(),
            other => {
                let count = if let SwapchainCreateData::Count(count) = other {
                    count
                } else {
                    2
                };

                (0..count)
                    .map(|_| self.context.device.create_texture(&texture_descriptor))
                    .collect()
            }
        };

        Ok(self.swapchain(textures, array_size))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn to_vulkan_images(textures: &[Texture]) -> Vec<vk::Image> {
    textures
        .iter()
        .map(|tex| unsafe {
            let hal_texture = tex.as_hal::<hal::api::Vulkan>();
            hal_texture.as_inner().unwrap().raw_handle()
        })
        .collect()
}
