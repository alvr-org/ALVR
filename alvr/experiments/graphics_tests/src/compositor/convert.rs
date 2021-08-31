use super::{Compositor, Swapchain, TextureType};
use alvr_common::prelude::*;
use openxr_sys as sys;
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use wgpu_hal as hal;

pub enum SwapchainCreateData {
    // Used for the Vulkan layer
    #[cfg(target_os = "linux")]
    External {
        images: Vec<vk::Image>,
        vk_usage: vk::ImageUsageFlags,
        vk_format: vk::Format,
        hal_usage: hal::TextureUses,
    },

    // Used for the Windows OpenVR driver (Some) or for a OpenXR runtime (None)
    Count(Option<usize>),
}

pub struct SwapchainCreateInfo {
    usage: sys::SwapchainUsageFlags,
    format: TextureFormat,
    sample_count: u32,
    width: u32,
    height: u32,
    texture_type: TextureType,
    mip_count: u32,
}

impl Compositor {
    // corresponds to xrCreateSwapchain
    pub fn create_swapchain(
        &self,
        data: SwapchainCreateData,
        info: SwapchainCreateInfo,
    ) -> StrResult<Swapchain> {
        let wgpu_usage = {
            let mut wgpu_usage = TextureUsages::TEXTURE_BINDING; // Always required

            if info
                .usage
                .contains(sys::SwapchainUsageFlags::COLOR_ATTACHMENT)
            {
                wgpu_usage |= TextureUsages::RENDER_ATTACHMENT;
            }
            if info
                .usage
                .contains(sys::SwapchainUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            {
                wgpu_usage |= TextureUsages::RENDER_ATTACHMENT;
            }
            if info.usage.contains(sys::SwapchainUsageFlags::TRANSFER_SRC) {
                wgpu_usage |= TextureUsages::COPY_SRC;
            }
            if info.usage.contains(sys::SwapchainUsageFlags::TRANSFER_DST) {
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
            TextureType::D2 { array_size } => array_size,
            TextureType::Cubemap => 6,
        };

        let texture_descriptor = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: info.width,
                height: info.height,
                depth_or_array_layers,
            },
            mip_level_count: info.mip_count,
            sample_count: info.sample_count,
            dimension: TextureDimension::D2,
            format: info.format,
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
            SwapchainCreateData::Count(count) => (0..count.unwrap_or(2))
                .map(|_| self.context.device().create_texture(&texture_descriptor))
                .collect(),
        };

        let array_size = match info.texture_type {
            TextureType::D2 { array_size } => array_size,
            TextureType::Cubemap => 1,
        };

        Ok(self.inner_create_swapchain(textures, array_size))
    }
}
