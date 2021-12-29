use super::{OpenxrSwapchain, XrContext};
use alvr_common::{glam::UVec2, prelude::*};
use alvr_graphics::{
    ash::{
        self,
        extensions::khr,
        vk::{self, Handle},
    },
    convert::{
        self, GraphicsContextVulkanInitDesc, SwapchainCreateData, SwapchainCreateInfo, TextureType,
        TARGET_VULKAN_VERSION,
    },
    wgpu::{Device, TextureFormat, TextureViewDescriptor},
    wgpu_hal as hal, GraphicsContext,
};
use openxr as xr;
use parking_lot::Mutex;
use std::{
    ffi::{CStr, CString},
    mem,
    sync::Arc,
};

pub fn create_graphics_context(xr_context: &XrContext) -> StrResult<GraphicsContext> {
    let entry = unsafe { ash::Entry::new().unwrap() };

    let raw_instance = unsafe {
        let extensions_ptrs =
            convert::get_vulkan_instance_extensions(&entry, TARGET_VULKAN_VERSION)?
                .iter()
                .map(|x| x.as_ptr())
                .collect::<Vec<_>>();
        let layers = vec![CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()];
        let layers_ptrs = layers.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();

        let raw_instance_ptr =
            trace_err!(trace_err!(xr_context.instance.create_vulkan_instance(
                xr_context.system,
                mem::transmute(entry.static_fn().get_instance_proc_addr),
                &vk::InstanceCreateInfo::builder()
                    .application_info(
                        &vk::ApplicationInfo::builder().api_version(TARGET_VULKAN_VERSION),
                    )
                    .enabled_extension_names(&extensions_ptrs)
                    .enabled_layer_names(&layers_ptrs) as *const _ as *const _,
            ))?)?;
        ash::Instance::load(
            entry.static_fn(),
            vk::Instance::from_raw(raw_instance_ptr as _),
        )
    };

    let raw_physical_device = vk::PhysicalDevice::from_raw(trace_err!(xr_context
        .instance
        .vulkan_graphics_device(xr_context.system, raw_instance.handle().as_raw() as _))?
        as _);

    // unsafe {
    //     let device_exts = raw_instance
    //         .enumerate_device_extension_properties(raw_physical_device)
    //         .unwrap();
    //     let device_exts_cstrs = device_exts
    //         .iter()
    //         .map(|ext| CStr::from_ptr(ext.extension_name.as_ptr() as _))
    //         .collect::<Vec<_>>();
    //     dbg!(device_exts_cstrs);
    // }

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

    let raw_device = unsafe {
        let temp_adapter = convert::get_temporary_hal_adapter(
            entry.clone(),
            TARGET_VULKAN_VERSION,
            raw_instance.clone(),
            raw_physical_device,
        )?;

        let extensions = temp_adapter
            .adapter
            .required_device_extensions(temp_adapter.features);
        let mut extensions_ptrs = extensions.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();
        if cfg!(target_os = "android") {
            // For importing decoder images into Vulkan
            let extra_extensions = [
                vk::KhrExternalMemoryFn::name(),
                vk::AndroidExternalMemoryAndroidHardwareBufferFn::name(),
            ]
            .into_iter()
            .map(|ext| ext.as_ptr());

            extensions_ptrs.extend(extra_extensions);
        }

        let mut features = temp_adapter.adapter.physical_device_features(
            &extensions,
            temp_adapter.features,
            hal::UpdateAfterBindTypes::empty(),
        );

        let queue_infos = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build()];
        let info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&extensions_ptrs);
        let info = features.add_to_device_create_builder(info);

        let raw_device_ptr = trace_err!(trace_err!(xr_context.instance.create_vulkan_device(
            xr_context.system,
            mem::transmute(entry.static_fn().get_instance_proc_addr),
            raw_physical_device.as_raw() as _,
            &info as *const _ as *const _,
        ))?)?;
        ash::Device::load(
            raw_instance.fp_v1_0(),
            vk::Device::from_raw(raw_device_ptr as _),
        )
    };

    GraphicsContext::from_vulkan(GraphicsContextVulkanInitDesc {
        entry,
        version: TARGET_VULKAN_VERSION,
        raw_instance,
        raw_physical_device,
        raw_device,
        queue_family_index,
        queue_index,
        drop_guard: Some(Box::new(xr_context.instance.clone())),
    })
}

pub fn create_swapchain(
    device: &Device,
    session: &xr::Session<xr::Vulkan>,
    size: UVec2,
) -> OpenxrSwapchain {
    const FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

    let usage = xr::SwapchainUsageFlags::COLOR_ATTACHMENT | xr::SwapchainUsageFlags::SAMPLED;
    // This corresponds to USAGE
    let hal_usage = hal::TextureUses::COLOR_TARGET | hal::TextureUses::RESOURCE;

    let swapchain = session
        .create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: usage,
            format: FORMAT.as_raw() as _,
            sample_count: 1,
            width: size.x,
            height: size.y,
            face_count: 1,
            array_size: 1,
            mip_count: 1,
        })
        .unwrap();
    let swapchain = Arc::new(Mutex::new(swapchain));

    let textures = convert::create_texture_set(
        device,
        SwapchainCreateData::External {
            images: swapchain
                .lock()
                .enumerate_images()
                .unwrap()
                .iter()
                .map(|raw_image| vk::Image::from_raw(*raw_image))
                .collect(),
            hal_usage,
            drop_guard: Some(Arc::new(()) as _),
        },
        SwapchainCreateInfo {
            usage,
            format: TextureFormat::Rgba8UnormSrgb,
            sample_count: 1,
            size,
            texture_type: TextureType::Flat { array_size: 1 },
            mip_count: 1,
        },
    );

    OpenxrSwapchain {
        handle: swapchain,
        views: textures
            .iter()
            .map(|tex| {
                Arc::new(tex.create_view(&TextureViewDescriptor {
                    base_array_layer: 0,
                    ..Default::default()
                }))
            })
            .collect(),
        size,
    }
}
