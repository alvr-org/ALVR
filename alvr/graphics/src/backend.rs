use crate::GraphicsContext;
use alvr_common::{anyhow::{bail, Result}, glam::UVec2, once_cell::sync::Lazy, ToAny};
use ash::vk::{self, Handle};
use std::{
    ffi::{c_char, CStr},
    mem,
    os::raw::c_void,
    sync::Arc,
};
use wgpu::{
    hal, Backends, DeviceDescriptor, Extent3d, Features, Instance, InstanceDescriptor,
    InstanceFlags, PowerPreference, RequestAdapterOptions, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

pub const TARGET_FORMAT_VK: vk::Format = vk::Format::R8G8B8A8_UNORM;

pub type GetInstanceProcAddrFn =
    unsafe extern "system" fn(*const c_void, *const c_char) -> Option<unsafe extern "system" fn()>;

// Note: Multiview and NV12 support are implemented in wgpu as Vulkan 1.1 features
// todo: PR extension-based implementation to wgpu over Vulkan 1.0
static REQUIRED_FEATURES: Lazy<Features> =
    Lazy::new(|| Features::PUSH_CONSTANTS | Features::MULTIVIEW | Features::TEXTURE_FORMAT_NV12);

pub enum ColorModel {
    YCbCrUnknown,
    YCbCr709,
    YCbCr601,
    YCbCr2020,
}

pub enum ColorRange {
    Full,
    Limited,
}

pub enum ChromaLocation {
    CositedEven,
    Midpoint,
}

pub struct YuvColorProperties {
    pub model: ColorModel,
    pub range: ColorRange,
    pub x_chroma_offset: ChromaLocation,
    pub y_chroma_offset: ChromaLocation,
}

#[derive(Clone)]
pub struct VulkanBackend {
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue_family_index: u32,
    queue_index: u32,
}

pub struct VulkanInitCallbacks<'a> {
    /// (get_instance_proc_addr, instance_create info) -> vk_instance,
    pub create_instance: &'a dyn Fn(GetInstanceProcAddrFn, *const c_void) -> Result<*const c_void>,
    /// (vk_instance) -> vk_physical_device,
    pub get_physical_device: &'a dyn Fn(*const c_void) -> Result<*const c_void>,
    /// (get_instance_proc_addr, vk_physical_device, device_create_info) -> vk_device,
    pub create_device:
        &'a dyn Fn(GetInstanceProcAddrFn, *const c_void, *const c_void) -> Result<*const c_void>,
}

pub struct RawVulkanHandles {
    pub instance: *const c_void,
    pub physical_device: *const c_void,
    pub device: *const c_void,
    pub queue_family_index: u32,
    pub queue_index: u32,
}

impl GraphicsContext<VulkanBackend> {
    pub fn new_vulkan() -> Result<Self> {
        let mut flags = InstanceFlags::empty();
        if cfg!(debug_assertions) {
            flags |= InstanceFlags::VALIDATION;
            flags |= InstanceFlags::DEBUG;
        }

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            flags,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::None, // todo?
            force_fallback_adapter: false,
            compatible_surface: None, // todo?
        }))
        .to_any()?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: *REQUIRED_FEATURES,
                required_limits: adapter.limits(),
            },
            None,
        ))?;

        let backend_handles = unsafe {
            VulkanBackend {
                instance: instance
                    .as_hal::<hal::api::Vulkan>()
                    .unwrap()
                    .shared_instance()
                    .raw_instance()
                    .clone(),
                physical_device: adapter.as_hal::<hal::api::Vulkan, _, _>(|adapter| {
                    adapter.unwrap().raw_physical_device()
                }),
                device: device
                    .as_hal::<hal::api::Vulkan, _, _>(|device| device.unwrap().raw_device().clone())
                    .unwrap(),
                queue_family_index: 0, // This is what wgpu uses
                queue_index: 0,
            }
        };

        Ok(Self {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            backend_handles,
        })
    }

    // This constructor is mainly used for interop with OpenXR XR_KHR_vulkan_enable2
    pub fn new_vulkan_external(callbacks: VulkanInitCallbacks) -> Result<Self> {
        let entry = unsafe { ash::Entry::load()? };

        #[cfg(target_os = "android")]
        let android_sdk_version = android_system_properties::AndroidSystemProperties::new()
            .get("ro.build.version.sdk")
            .to_any()?
            .parse::<u32>()?;
        #[cfg(not(target_os = "android"))]
        let android_sdk_version = 0;

        let api_version = entry
            .try_enumerate_instance_version()?
            .unwrap_or(vk::API_VERSION_1_0);

        let application_info = vk::ApplicationInfo::builder().api_version(api_version);

        let mut flags = InstanceFlags::empty();
        if cfg!(debug_assertions) {
            flags |= InstanceFlags::VALIDATION;
            flags |= InstanceFlags::DEBUG;
        }

        let instance_extensions = <hal::api::Vulkan as hal::Api>::Instance::desired_extensions(
            &entry,
            api_version,
            flags,
        )?;

        let instance_extensions_ptrs = instance_extensions
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        // todo: contribute better way to get layers from wgpu
        let layers_ptrs = entry
            .enumerate_instance_layer_properties()
            .unwrap()
            .iter()
            .filter_map(|props| {
                let name = unsafe { CStr::from_ptr(props.layer_name.as_ptr()) };
                if name.to_str().unwrap() == "VK_LAYER_KHRONOS_validation" {
                    Some(props.layer_name.as_ptr())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // todo: debug utils

        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_extension_names(&instance_extensions_ptrs)
            .enabled_layer_names(&layers_ptrs)
            .build();

        let instance_ptr = (callbacks.create_instance)(
            unsafe { mem::transmute(entry.static_fn().get_instance_proc_addr) },
            &instance_create_info as *const _ as *const c_void,
        )?;
        let instance_vk = unsafe {
            ash::Instance::load(entry.static_fn(), vk::Instance::from_raw(instance_ptr as _))
        };
        let hal_instance = unsafe {
            <hal::api::Vulkan as hal::Api>::Instance::from_raw(
                entry.clone(),
                instance_vk.clone(),
                api_version,
                android_sdk_version,
                None,
                instance_extensions,
                flags,
                false,
                Some(Box::new(())),
            )?
        };

        let physical_device_ptr = (callbacks.get_physical_device)(instance_ptr)?;
        let physical_device_vk = vk::PhysicalDevice::from_raw(physical_device_ptr as _);
        let exposed_adapter = hal_instance.expose_adapter(physical_device_vk).to_any()?;

        // assert!(exposed_adapter.features.contains(*REQUIRED_FEATURES));

        let features = exposed_adapter.features;

        // code below is mostly copied from
        // https://github.com/gfx-rs/wgpu/blob/f9509bcf9ec2b63a64eb7fea93f7f44cd5ae4d2e/wgpu-hal/src/vulkan/adapter.rs#L1597-L1598
        let mut device_extensions = exposed_adapter.adapter.required_device_extensions(features);
        if cfg!(target_os = "android") {
            // For importing decoder images into Vulkan
            // https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_ANDROID_external_memory_android_hardware_buffer.html#_extension_and_version_dependencies
            device_extensions.extend([
                // vk::KhrMaintenance1Fn::name(), // promoted to Vulkan 1.1
                // vk::KhrBindMemory2Fn::name(), // promoted to Vulkan 1.1
                // vk::KhrGetMemoryRequirements2Fn::name(), // promoted to Vulkan 1.1
                // vk::KhrGetPhysicalDeviceProperties2Fn::name(), // promoted to Vulkan 1.1
                // vk::KhrSamplerYcbcrConversionFn::name(), // promoted to Vulkan 1.1
                // vk::KhrExternalMemoryCapabilitiesFn::name(), // promoted to Vulkan 1.1
                // vk::KhrExternalMemoryFn::name(), // promoted to Vulkan 1.1
                // vk::KhrDedicatedAllocationFn::name(), // promoted to Vulkan 1.1
                vk::ExtQueueFamilyForeignFn::name(), // Needs Vulkan 1.1 or KhrExternalMemoryFn
                vk::AndroidExternalMemoryAndroidHardwareBufferFn::name(),
            ]);
        };
        let device_extensions_ptrs = device_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let mut physical_device_features = exposed_adapter
            .adapter
            .physical_device_features(&device_extensions, features);

        let queue_family_index = 0; // This is what wgpu uses
        let queue_create_infos = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build()];
        let queue_index = 0;

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions_ptrs);
        let device_create_info = physical_device_features
            .add_to_device_create_builder(device_create_info)
            .build();

        let device_ptr = (callbacks.create_device)(
            unsafe { mem::transmute(entry.static_fn().get_instance_proc_addr) },
            physical_device_ptr,
            &device_create_info as *const _ as *const c_void,
        )?;
        let device_vk = unsafe {
            ash::Device::load(instance_vk.fp_v1_0(), vk::Device::from_raw(device_ptr as _))
        };
        let open_device = unsafe {
            exposed_adapter.adapter.device_from_raw(
                device_vk.clone(),
                false,
                &device_extensions,
                features,
                queue_family_index,
                queue_index,
            )?
        };

        let instance = unsafe { Instance::from_hal::<hal::api::Vulkan>(hal_instance) };
        let adapter = unsafe { instance.create_adapter_from_hal(exposed_adapter) };
        let (device, queue) = unsafe {
            adapter.create_device_from_hal(
                open_device,
                &DeviceDescriptor {
                    label: None,
                    required_features: features,
                    required_limits: adapter.limits(),
                },
                None,
            )?
        };

        Ok(Self {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            backend_handles: VulkanBackend {
                instance: instance_vk,
                physical_device: physical_device_vk,
                device: device_vk,
                queue_family_index,
                queue_index,
            },
        })
    }

    pub fn vulkan_handles(&self) -> RawVulkanHandles {
        RawVulkanHandles {
            instance: self.backend_handles.instance.handle().as_raw() as _,
            physical_device: self.backend_handles.physical_device.as_raw() as _,
            device: self.backend_handles.device.handle().as_raw() as _,
            queue_family_index: self.backend_handles.queue_family_index,
            queue_index: self.backend_handles.queue_index,
        }
    }

    pub fn create_vulkan_swapchain(
        &self,
        swapchain_size: u32,
        resolution: UVec2,
        layer_depth: u32,
    ) -> Vec<Texture> {
        (0..swapchain_size)
            .map(|_| {
                self.device.create_texture(&TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: resolution.x,
                        height: resolution.y,
                        depth_or_array_layers: layer_depth,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
            })
            .collect()
    }

    // This expects texture arrays of depth 2, RGBA8UnormSrgb
    pub fn create_vulkan_swapchain_external(
        &self,
        image_handles: &[u64],
        resolution: UVec2,
        layer_depth: u32,
    ) -> Vec<Texture> {
        image_handles
            .iter()
            .map(|image_handle| unsafe {
                let size = Extent3d {
                    width: resolution.x,
                    height: resolution.y,
                    depth_or_array_layers: layer_depth,
                };

                let hal_texture = <hal::api::Vulkan as hal::Api>::Device::texture_from_raw(
                    vk::Image::from_raw(*image_handle),
                    &hal::TextureDescriptor {
                        label: None,
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8UnormSrgb,
                        usage: hal::TextureUses::COLOR_TARGET,
                        memory_flags: hal::MemoryFlags::empty(),
                        view_formats: vec![],
                    },
                    Some(Box::new(())),
                );

                self.device.create_texture_from_hal::<hal::api::Vulkan>(
                    hal_texture,
                    &TextureDescriptor {
                        label: None,
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8UnormSrgb,
                        usage: TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    },
                )
            })
            .collect()
    }

    /// Note: The returned texture has format NV12
    /// # Safety
    /// `buffer_ptr` must be a valid pointer
    /// `resolution` is the resolution of the buffer, not the requested streaming resolution
    pub unsafe fn ahardwarebuffer_to_texture(
        &self,
        buffer_ptr: *const c_void,
        resolution: UVec2,
    ) -> Result<(Texture, YuvColorProperties)> {
        let vk_instance = &self.backend_handles.instance;
        let vk_physical_device = self.backend_handles.physical_device;
        let vk_device = &self.backend_handles.device;

        let physical_device_memory_properties =
            vk_instance.get_physical_device_memory_properties(vk_physical_device);

        let mut hardware_buffer_format_properties =
            vk::AndroidHardwareBufferFormatPropertiesANDROID::builder()
                .format(vk::Format::UNDEFINED)
                .format_features(vk::FormatFeatureFlags::SAMPLED_IMAGE)
                .build();
        let mut hardware_buffer_properties = vk::AndroidHardwareBufferPropertiesANDROID::builder()
            .push_next(&mut hardware_buffer_format_properties)
            .build();
        (vk::AndroidExternalMemoryAndroidHardwareBufferFn::load(|name: &std::ffi::CStr| {
            mem::transmute(vk_instance.get_device_proc_addr(vk_device.handle(), name.as_ptr()))
        })
        .get_android_hardware_buffer_properties_android)(
            vk_device.handle(),
            buffer_ptr as _,
            &mut hardware_buffer_properties as _,
        )
        .result()?;

        let external_format = vk::ExternalFormatANDROID::builder()
            .external_format(hardware_buffer_format_properties.external_format)
            .build();
        let mut external_memory_image_create_info = vk::ExternalMemoryImageCreateInfo::builder()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID)
            .build();
        external_memory_image_create_info.p_next = &external_format as *const _ as _;
        let image = vk_device.create_image(
            &vk::ImageCreateInfo::builder()
                .push_next(&mut external_memory_image_create_info)
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::UNDEFINED)
                .extent(vk::Extent3D {
                    width: resolution.x,
                    height: resolution.y,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .queue_family_indices(&[])
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .flags(vk::ImageCreateFlags::empty())
                .build(),
            None,
        )?;

        let import_android_hardware_buffer_info =
            vk::ImportAndroidHardwareBufferInfoANDROID::builder()
                .buffer(buffer_ptr as _)
                .build();
        let mut memory_dedicated_allocate_info = vk::MemoryDedicatedAllocateInfo::builder()
            .image(image)
            .build();
        memory_dedicated_allocate_info.p_next =
            &import_android_hardware_buffer_info as *const _ as _;
        let memory_requirements = vk::MemoryRequirements::builder()
            .size(hardware_buffer_properties.allocation_size)
            .alignment(0)
            .memory_type_bits(hardware_buffer_properties.memory_type_bits)
            .build();
        let memory_type_index = physical_device_memory_properties
            .memory_types
            .iter()
            .take(physical_device_memory_properties.memory_type_count as usize)
            .enumerate()
            .find(|(idx, memory_type)| {
                memory_requirements.memory_type_bits & (1 << idx) != 0
                    && memory_type
                        .property_flags
                        .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .map(|(idx, _)| idx)
            .to_any()?;
        let device_memory = vk_device.allocate_memory(
            &vk::MemoryAllocateInfo::builder()
                .push_next(&mut memory_dedicated_allocate_info)
                .allocation_size(memory_requirements.size)
                .memory_type_index(memory_type_index as u32)
                .build(),
            None,
        )?;

        vk_device.bind_image_memory(image, device_memory, 0)?;

        let hal_texture = <hal::api::Vulkan as hal::Api>::Device::texture_from_raw(
            image,
            &hal::TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: resolution.x,
                    height: resolution.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::NV12,
                usage: hal::TextureUses::RESOURCE,
                memory_flags: hal::MemoryFlags::empty(),
                view_formats: vec![],
            },
            None,
        );
        let texture = self.device.create_texture_from_hal::<hal::api::Vulkan>(
            hal_texture,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: resolution.x,
                    height: resolution.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::NV12,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let color_properties = YuvColorProperties {
            model: match hardware_buffer_format_properties.suggested_ycbcr_model {
                vk::SamplerYcbcrModelConversion::YCBCR_IDENTITY => ColorModel::YCbCrUnknown,
                vk::SamplerYcbcrModelConversion::YCBCR_709 => ColorModel::YCbCr709,
                vk::SamplerYcbcrModelConversion::YCBCR_601 => ColorModel::YCbCr601,
                vk::SamplerYcbcrModelConversion::YCBCR_2020 => ColorModel::YCbCr2020,
                _ => bail!("Unsupported YCbCr model"),
            },
            range: match hardware_buffer_format_properties.suggested_ycbcr_range {
                vk::SamplerYcbcrRange::ITU_FULL => ColorRange::Full,
                vk::SamplerYcbcrRange::ITU_NARROW => ColorRange::Limited,
                _ => bail!("Unsupported YCbCr range"),
            },
            x_chroma_offset: match hardware_buffer_format_properties.suggested_x_chroma_offset {
                vk::ChromaLocation::COSITED_EVEN => ChromaLocation::CositedEven,
                vk::ChromaLocation::MIDPOINT => ChromaLocation::Midpoint,
                _ => bail!("Unsupported chroma location"),
            },
            y_chroma_offset: match hardware_buffer_format_properties.suggested_y_chroma_offset {
                vk::ChromaLocation::COSITED_EVEN => ChromaLocation::CositedEven,
                vk::ChromaLocation::MIDPOINT => ChromaLocation::Midpoint,
                _ => bail!("Unsupported chroma location"),
            },
        };

        Ok((texture, color_properties))
    }
}

pub fn get_texture_vk_handle(texture: &Texture) -> u64 {
    let mut handle = 0;
    unsafe {
        texture.as_hal::<hal::api::Vulkan, _>(|texture| {
            handle = texture.unwrap().raw_handle().as_raw();
        })
    }

    handle
}
