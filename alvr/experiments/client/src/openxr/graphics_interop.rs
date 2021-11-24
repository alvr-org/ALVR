use std::{mem, sync::Arc};

use super::{OpenxrContext, OpenxrSwapchain};
use alvr_common::prelude::*;
use alvr_graphics::{
    convert::{
        self, SwapchainCreateData, SwapchainCreateInfo, TextureType, DEVICE_FEATURES,
        TARGET_VULKAN_VERSION,
    },
    GraphicsContext,
};
use ash::vk::{self, Handle};
use openxr as xr;
use parking_lot::Mutex;
use wgpu::{Device, TextureFormat, TextureViewDescriptor};
use wgpu_hal as hal;

pub fn create_graphics_context(xr_context: &OpenxrContext) -> StrResult<GraphicsContext> {
    let entry = unsafe { ash::Entry::new().unwrap() };

    let raw_instance = unsafe {
        let extensions_ptrs =
            convert::get_vulkan_instance_extensions(&entry, TARGET_VULKAN_VERSION)?
                .iter()
                .map(|x| x.as_ptr())
                .collect::<Vec<_>>();
        let raw_instance_ptr =
            trace_err!(trace_err!(xr_context.instance.create_vulkan_instance(
                xr_context.system,
                mem::transmute(entry.static_fn().get_instance_proc_addr),
                &vk::InstanceCreateInfo::builder()
                    .application_info(
                        &vk::ApplicationInfo::builder().api_version(TARGET_VULKAN_VERSION),
                    )
                    .enabled_extension_names(&extensions_ptrs) as *const _
                    as *const _,
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
        let extensions = temp_adapter.required_device_extensions(DEVICE_FEATURES);
        let mut features = temp_adapter.physical_device_features(
            &extensions,
            DEVICE_FEATURES,
            wgpu_hal::UpdateAfterBindTypes::empty(),
        );

        let queue_infos = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build()];
        let extensions_ptrs = extensions.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();
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

    GraphicsContext::from_vulkan(
        entry,
        TARGET_VULKAN_VERSION,
        raw_instance,
        raw_physical_device,
        raw_device,
        queue_family_index,
        queue_index,
        Some(Box::new(xr_context.instance.clone())),
    )
}

pub fn create_swapchain(
    device: &Device,
    session: &xr::Session<xr::Vulkan>,
    size: (u32, u32),
) -> OpenxrSwapchain {
    const FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;
    const USAGE: xr::SwapchainUsageFlags = xr::SwapchainUsageFlags::COLOR_ATTACHMENT;

    let swapchain = session
        .create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: USAGE,
            format: FORMAT.as_raw() as _,
            sample_count: 1,
            width: size.0,
            height: size.1,
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
            vk_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk_format: FORMAT,
            hal_usage: hal::TextureUses::COLOR_TARGET,
            drop_guard: Some(Arc::clone(&swapchain) as _),
        },
        SwapchainCreateInfo {
            usage: USAGE,
            format: TextureFormat::Rgba8UnormSrgb,
            sample_count: 1,
            width: size.0,
            height: size.1,
            texture_type: TextureType::Flat { array_size: 1 },
            mip_count: 1,
        },
    );

    OpenxrSwapchain {
        handle: swapchain,
        views: textures
            .iter()
            .map(|tex| {
                tex.create_view(&TextureViewDescriptor {
                    base_array_layer: 0,
                    ..Default::default()
                })
            })
            .collect(),
        view_size: (size.0 as _, size.1 as _),
    }
}
