mod graphics;
mod video_decoder;

use alvr_common::prelude::*;
use alvr_graphics::{
    convert::{
        self, SwapchainCreateData, SwapchainCreateInfo, DEVICE_FEATURES, TARGET_VULKAN_VERSION,
    },
    Context,
};
use ash::vk::{self, Handle};
use openxr as xr;
use std::{ffi::CString, mem, sync::Arc};
use wgpu::{Device, TextureDimension, TextureFormat, TextureView, TextureViewDescriptor};
use wgpu_hal as hal;

struct Swapchain {
    handle: Arc<xr::Swapchain<xr::Vulkan>>,
    views: Vec<TextureView>,
}

fn create_swapchain(
    device: &Device,
    session: &xr::Session<xr::Vulkan>,
    size: (u32, u32),
) -> Swapchain {
    const FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;
    const USAGE: xr::SwapchainUsageFlags = xr::SwapchainUsageFlags::COLOR_ATTACHMENT;

    let swapchain = Arc::new(
        session
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
            .unwrap(),
    );

    let textures = convert::create_texture_set(
        device,
        SwapchainCreateData::External {
            images: swapchain
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
            texture_type: convert::TextureType::Flat { array_size: 1 },
            mip_count: 1,
        },
    );

    Swapchain {
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
    }
}

pub fn create_context_and_session(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> StrResult<(
    Context,
    xr::Session<xr::Vulkan>,
    xr::FrameWaiter,
    xr::FrameStream<xr::Vulkan>,
)> {
    // this call is required by spec
    let reqs = xr_instance
        .graphics_requirements::<xr::Vulkan>(system)
        .unwrap();

    // Oculus Go requires baseline Vulkan v1.0.0
    if reqs.min_api_version_supported > xr::Version::new(1, 0, 0) {
        return fmt_e!("Incompatible vulkan version");
    }

    let vk_entry = unsafe { ash::Entry::new().unwrap() };

    let vk_instance = unsafe {
        let extensions_ptrs =
            convert::get_vulkan_instance_extensions(&vk_entry, TARGET_VULKAN_VERSION)?
                .iter()
                .map(|x| x.as_ptr())
                .collect::<Vec<_>>();
        let raw_instance = trace_err!(trace_err!(xr_instance.create_vulkan_instance(
            system,
            mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
            &vk::InstanceCreateInfo::builder()
                .application_info(
                    &vk::ApplicationInfo::builder().api_version(TARGET_VULKAN_VERSION),
                )
                .enabled_extension_names(&extensions_ptrs) as *const _ as *const _,
        ))?)?;
        ash::Instance::load(
            vk_entry.static_fn(),
            vk::Instance::from_raw(raw_instance as _),
        )
    };

    let vk_physical_device = vk::PhysicalDevice::from_raw(trace_err!(
        xr_instance.vulkan_graphics_device(system, vk_instance.handle().as_raw() as _)
    )? as _);

    let queue_family_index = unsafe {
        vk_instance
            .get_physical_device_queue_family_properties(vk_physical_device)
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

    let vk_device = unsafe {
        let temp_adapter = convert::get_temporary_hal_adapter(
            vk_entry.clone(),
            TARGET_VULKAN_VERSION,
            vk_instance.clone(),
            vk_physical_device,
        )?;
        let extensions = temp_adapter.required_device_extensions(DEVICE_FEATURES);
        let mut features = temp_adapter.physical_device_features(&extensions, DEVICE_FEATURES);

        let queue_infos = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build()];
        let extensions_ptrs = extensions.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();
        let info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&extensions_ptrs);
        let info = features.add_to_device_create_builder(info);

        let raw_device = trace_err!(trace_err!(xr_instance.create_vulkan_device(
            system,
            mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
            vk_physical_device.as_raw() as _,
            &info as *const _ as *const _,
        ))?)?;
        ash::Device::load(vk_instance.fp_v1_0(), vk::Device::from_raw(raw_device as _))
    };

    let (session, mut frame_wait, mut frame_stream) = unsafe {
        trace_err!(xr_instance.create_session::<xr::Vulkan>(
            system,
            &xr::vulkan::SessionCreateInfo {
                instance: vk_instance.handle().as_raw() as _,
                physical_device: vk_physical_device.as_raw() as _,
                device: vk_device.handle().as_raw() as _,
                queue_family_index,
                queue_index: 0,
            },
        ))?
    };

    Ok((
        Context::from_vulkan(
            true,
            vk_entry,
            TARGET_VULKAN_VERSION,
            vk_instance,
            vk_physical_device,
            vk_device,
            queue_family_index,
            queue_index,
        )?,
        session,
        frame_wait,
        frame_stream,
    ))
}

pub fn run() -> StrResult {
    let entry = xr::Entry::load().unwrap();

    #[cfg(target_os = "android")]
    entry.initialize_android_loader().unwrap();

    let available_extensions = entry.enumerate_extensions().unwrap();

    let mut enabled_extensions = xr::ExtensionSet::default();
    enabled_extensions.khr_vulkan_enable2 = true;
    #[cfg(target_os = "android")]
    {
        enabled_extensions.khr_android_create_instance = true;
    }
    let xr_instance = entry
        .create_instance(
            &xr::ApplicationInfo {
                application_name: "ALVR client",
                application_version: 0,
                engine_name: "ALVR",
                engine_version: 0,
            },
            &enabled_extensions,
            &[],
        )
        .unwrap();

    let system = xr_instance
        .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
        .unwrap();

    let environment_blend_mode = xr_instance
        .enumerate_environment_blend_modes(system, xr::ViewConfigurationType::PRIMARY_STEREO)
        .unwrap()[0];

    let (context, session, frame_wait, frame_stream) =
        create_context_and_session(&xr_instance, system)?;

    todo!()
}

#[cfg_attr(target_os = "android", ndk_glue::main)]
pub fn main() {
    show_err(run());

    #[cfg(target_os = "android")]
    ndk_glue::native_activity().finish();
}
