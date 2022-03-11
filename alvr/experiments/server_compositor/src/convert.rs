use alvr_common::prelude::*;
use alvr_graphics::{
    ash::{self, vk},
    convert::{
        self, get_vulkan_queue_families_info, GraphicsContextVulkanInitDesc, TARGET_VULKAN_VERSION,
    },
    GraphicsContext, RawGraphicsHandles,
};

pub fn create_windows_graphics_context(adapter_index: Option<usize>) -> StrResult<GraphicsContext> {
    let entry = unsafe { ash::Entry::load().map_err(err!())? };

    let instance = convert::create_vulkan_instance(
        &entry,
        &vk::InstanceCreateInfo::builder()
            .application_info(&vk::ApplicationInfo::builder().api_version(TARGET_VULKAN_VERSION))
            .build(),
    )
    .map_err(err!())?;

    let physical_device = convert::get_vulkan_graphics_device(&instance, adapter_index)?;

    let queues_family_info = get_vulkan_queue_families_info(&instance, physical_device);

    let device = convert::create_vulkan_device(
        entry.clone(),
        TARGET_VULKAN_VERSION,
        &instance,
        physical_device,
        &vk::DeviceCreateInfo::builder().queue_create_infos(&[vk::DeviceQueueCreateInfo::builder(
        )
        .queue_family_index(queues_family_info.rendering_index)
        .queue_priorities(&[1.0])
        .build()]),
    )
    .map_err(err!())?;

    GraphicsContext::from_vulkan(GraphicsContextVulkanInitDesc {
        entry,
        version: TARGET_VULKAN_VERSION,
        raw: RawGraphicsHandles {
            instance,
            physical_device,
            device,
            queues_family_info,
            rendering_queue_index: 0,
        },
        drop_guard: Some(Box::new(())),
    })
}
