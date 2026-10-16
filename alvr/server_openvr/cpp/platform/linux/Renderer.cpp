#include "Renderer.hpp"

#include <cstdint>
#include <cstring>
#include <optional>
#include <stdexcept>
#include <unistd.h>
#include <vulkan/vulkan_structs.hpp>

#include "alvr_server/bindings.h"

namespace alvr::render::detail {

RenderPipeline::RenderPipeline(
    VkContext const& ctx, vk::DescriptorSetLayout& layout, PipelineCreateInfo& pipelineCI
) {
    vk::ShaderModuleCreateInfo shaderCI {
        .codeSize = pipelineCI.shaderData.size(),
        .pCode = reinterpret_cast<u32 const*>(pipelineCI.shaderData.data()),
    };
    shader = ctx.dev.createShaderModule(shaderCI);

    // Every pass shares one push-constant range; shaders that do not declare
    // the block simply ignore it.
    vk::PushConstantRange pushRange {
        .stageFlags = vk::ShaderStageFlagBits::eCompute,
        .offset = 0,
        .size = sizeof(PushConstants),
    };
    vk::PipelineLayoutCreateInfo pipeLayoutCI {
        .setLayoutCount = 1,
        .pSetLayouts = &layout,
        .pushConstantRangeCount = 1,
        .pPushConstantRanges = &pushRange,
    };
    pipeLayout = ctx.dev.createPipelineLayout(pipeLayoutCI);

    vk::SpecializationInfo specInfo {
        .mapEntryCount = static_cast<u32>(pipelineCI.specs.size()),
        .pMapEntries = pipelineCI.specs.data(),
        .dataSize = pipelineCI.specData.size(),
        .pData = pipelineCI.specData.data(),
    };

    vk::PipelineShaderStageCreateInfo stageCI {
        .stage = vk::ShaderStageFlagBits::eCompute,
        .module = shader,
        .pName = "main",
        .pSpecializationInfo = not pipelineCI.specs.empty() ? &specInfo : nullptr,
    };
    vk::ComputePipelineCreateInfo pipeCI {
        .stage = stageCI,
        .layout = pipeLayout,
    };
    auto [result, pipes] = ctx.dev.createComputePipelines({}, pipeCI);

    assert(result == vk::Result::eSuccess);
    pipe = pipes[0];
};

void RenderPipeline::render(
    VkContext const& ctx,
    vk::CommandBuffer cmdBuf,
    vk::ImageView in,
    vk::ImageView out,
    vk::Extent2D outSize,
    PushConstants const& pushConstants
) {
    cmdBuf.bindPipeline(vk::PipelineBindPoint::eCompute, pipe);
    cmdBuf.pushConstants(
        pipeLayout, vk::ShaderStageFlagBits::eCompute, 0, sizeof(PushConstants), &pushConstants
    );

    vk::DescriptorImageInfo descImgInfoIn {
        .imageView = in,
        .imageLayout = vk::ImageLayout::eShaderReadOnlyOptimal,
    };
    vk::DescriptorImageInfo descImgInfoOut {
        .imageView = out,
        .imageLayout = vk::ImageLayout::eGeneral,
    };

    vk::WriteDescriptorSet descWriteSets[2];
    descWriteSets[0] = {
        .dstBinding = 0,
        .descriptorCount = 1,
        .descriptorType = vk::DescriptorType::eCombinedImageSampler,
        .pImageInfo = &descImgInfoIn,
    };
    descWriteSets[1] = {
        .dstBinding = 1,
        .descriptorCount = 1,
        .descriptorType = vk::DescriptorType::eStorageImage,
        .pImageInfo = &descImgInfoOut,
    };
    cmdBuf.pushDescriptorSetKHR(
        vk::PipelineBindPoint::eCompute, pipeLayout, 0, descWriteSets, ctx.dispatch
    );

    cmdBuf.dispatch((outSize.width + 7) / 8, (outSize.height + 7) / 8, 1);
}

void RenderPipeline::destroy(VkContext const& ctx) {
    ctx.dev.destroy(pipe);
    ctx.dev.destroy(pipeLayout);
    ctx.dev.destroy(shader);
}

}

#ifndef DRM_FORMAT_INVALID
#define DRM_FORMAT_INVALID 0
#define fourcc_code(a, b, c, d)                                                                    \
    ((uint32_t)(a) | ((uint32_t)(b) << 8) | ((uint32_t)(c) << 16) | ((uint32_t)(d) << 24))
#define DRM_FORMAT_ARGB8888 fourcc_code('A', 'R', '2', '4')
#define DRM_FORMAT_ABGR8888 fourcc_code('A', 'B', '2', '4')
#define fourcc_mod_code(vendor, val) ((((uint64_t)vendor) << 56) | ((val) & 0x00ffffffffffffffULL))
#define DRM_FORMAT_MOD_INVALID fourcc_mod_code(0, ((1ULL << 56) - 1))
#define DRM_FORMAT_MOD_LINEAR fourcc_mod_code(0, 0)
#define DRM_FORMAT_MOD_VENDOR_AMD 0x02
#define AMD_FMT_MOD_DCC_SHIFT 13
#define AMD_FMT_MOD_DCC_MASK 0x1
#define IS_AMD_FMT_MOD(val) (((val) >> 56) == DRM_FORMAT_MOD_VENDOR_AMD)
#define AMD_FMT_MOD_GET(field, value)                                                              \
    (((value) >> AMD_FMT_MOD_##field##_SHIFT) & AMD_FMT_MOD_##field##_MASK)
#endif

static bool filter_modifier(uint64_t modifier) {
    if (IS_AMD_FMT_MOD(modifier)) {
        // DCC not supported as encode input
        if (AMD_FMT_MOD_GET(DCC, modifier)) {
            return false;
        }
    }
    return true;
}

static u32 to_drm_format(vk::Format format) {
    switch (format) {
    case vk::Format::eB8G8R8A8Unorm:
        return DRM_FORMAT_ARGB8888;
    case vk::Format::eR8G8B8A8Unorm:
        return DRM_FORMAT_ABGR8888;
    default:
        // assert(false);
        return DRM_FORMAT_INVALID;
    }
}

using namespace alvr;
using namespace alvr::render;

u32 memoryTypeIndex(VkContext const& ctx, vk::MemoryPropertyFlags properties, u32 typeBits) {
    auto prop = ctx.physDev.getMemoryProperties();
    for (u32 i = 0; i < prop.memoryTypeCount; ++i) {
        if ((prop.memoryTypes[i].propertyFlags & properties) == properties && typeBits & (1 << i)) {
            return i;
        }
    }
    throw std::runtime_error("No matching memoryTypeIndex found");
}

// Create or import Image, depending on whether an fd is passed. viewFormat
// overrides the view's format; the image must be MUTABLE_FORMAT for that to
// be legal when it differs from imageCI.format.
inline Image createImage(
    VkContext const& ctx,
    vk::ImageCreateInfo imageCI,
    std::optional<int> fd = std::nullopt,
    std::optional<vk::Format> viewFormat = std::nullopt
) {
    vk::ExternalMemoryImageCreateInfo extMemImgCI {
        .handleTypes = vk::ExternalMemoryHandleTypeFlagBits::eOpaqueFd,
    };
    if (fd.has_value()) {
        extMemImgCI.pNext = imageCI.pNext;
        imageCI.pNext = &extMemImgCI;
    }

    Image img {
        .image = ctx.dev.createImage(imageCI),
        .layout = imageCI.initialLayout,
    };

    vk::MemoryDedicatedRequirements dedicatedReqs {};
    vk::MemoryRequirements2 memReqs {
        .pNext = &dedicatedReqs,
    };
    vk::ImageMemoryRequirementsInfo2 memReqInfo {
        .image = img.image,
    };
    ctx.dev.getImageMemoryRequirements2(&memReqInfo, &memReqs);

    vk::ImportMemoryFdInfoKHR importMemInfo {
        .handleType = vk::ExternalMemoryHandleTypeFlagBits::eOpaqueFd,
        .fd = fd.value_or(-1),
    };
    vk::MemoryDedicatedAllocateInfo memDedicatedInfo {
        .pNext = fd.has_value() ? &importMemInfo : nullptr,
        .image = img.image,
    };
    vk::MemoryAllocateInfo memAllocInfo {
        .pNext = &memDedicatedInfo,
        .allocationSize = memReqs.memoryRequirements.size,
        .memoryTypeIndex = memoryTypeIndex(
            ctx, vk::MemoryPropertyFlagBits::eDeviceLocal, memReqs.memoryRequirements.memoryTypeBits
        ),
    };
    img.memory = ctx.dev.allocateMemory(memAllocInfo);

    vk::BindImageMemoryInfo bindImgInfo {
        .image = img.image,
        .memory = img.memory,
        .memoryOffset = 0,
    };
    ctx.dev.bindImageMemory2(bindImgInfo);

    vk::ImageViewCreateInfo imgViewCI {
        .image = img.image,
        .viewType = vk::ImageViewType::e2D,
        .format = viewFormat.value_or(imageCI.format),
        .components {
            .r = vk::ComponentSwizzle::eIdentity,
            .g = vk::ComponentSwizzle::eIdentity,
            .b = vk::ComponentSwizzle::eIdentity,
            .a = vk::ComponentSwizzle::eIdentity,
        },
        .subresourceRange {
            .aspectMask = vk::ImageAspectFlagBits::eColor,
            .baseMipLevel = 0,
            .levelCount = imageCI.mipLevels,
            .baseArrayLayer = 0,
            .layerCount = imageCI.arrayLayers,
        },
    };
    img.view = ctx.dev.createImageView(imgViewCI);

    return img;
}

Output createOutputImage(
    VkContext const& ctx, vk::Extent2D extent, vk::Format format, HandleType handleType
) {
    Image img;
    Output out;

    vk::ImageCreateInfo imgCI {
        .imageType = vk::ImageType::e2D,
        .format = format,
        .extent = {
            .width = extent.width,
            .height = extent.height,
            .depth = 1,
        },
        .mipLevels = 1,
        .arrayLayers = 1,
        .samples = vk::SampleCountFlagBits::e1,
        .usage = vk::ImageUsageFlagBits::eStorage | vk::ImageUsageFlagBits::eSampled,
        .sharingMode = vk::SharingMode::eExclusive,
        .initialLayout = vk::ImageLayout::eUndefined,
    };

    img.layout = imgCI.initialLayout;

    vk::ExternalMemoryImageCreateInfo extImgCI {};

    // Lifetime
    std::vector<uint64_t> imageModifiers;

    // TODO: Actually import this
    bool haveDmaBuf = true;
    bool haveDrmModifiers = true;
    std::vector<vk::DrmFormatModifierPropertiesEXT> modProps;

    if (haveDrmModifiers && handleType == HandleType::DmaBuf) {
        imgCI.tiling = vk::ImageTiling::eDrmFormatModifierEXT;

        vk::DrmFormatModifierPropertiesListEXT modPropList {};
        vk::FormatProperties2 formatProps {
            .pNext = &modPropList,
        };
        ctx.physDev.getFormatProperties2(imgCI.format, &formatProps);

        modProps.resize(modPropList.drmFormatModifierCount);
        modPropList.pDrmFormatModifierProperties = modProps.data();
        ctx.physDev.getFormatProperties2(imgCI.format, &formatProps);

        for (auto& prop : modProps) {
            if (!filter_modifier(prop.drmFormatModifier)) {
                continue;
            }

            vk::PhysicalDeviceImageDrmFormatModifierInfoEXT modInfo {
                .drmFormatModifier = prop.drmFormatModifier,
                .sharingMode = imgCI.sharingMode,
                .queueFamilyIndexCount = imgCI.queueFamilyIndexCount,
                .pQueueFamilyIndices = imgCI.pQueueFamilyIndices,
            };
            vk::PhysicalDeviceImageFormatInfo2 formatInfo {
                .pNext = &modInfo,
                .format = imgCI.format,
                .type = imgCI.imageType,
                .tiling = imgCI.tiling,
                .usage = imgCI.usage,
                .flags = imgCI.flags,
            };
            vk::ImageFormatProperties2 imgFormatProps {
                .pNext = nullptr,
            };
            auto result = ctx.physDev.getImageFormatProperties2(&formatInfo, &imgFormatProps);
            if (result == vk::Result::eSuccess)
                imageModifiers.push_back(prop.drmFormatModifier);
        }

        extImgCI.handleTypes = vk::ExternalMemoryHandleTypeFlagBits::eDmaBufEXT;

        vk::ImageDrmFormatModifierListCreateInfoEXT modListCI {
            .pNext = &extImgCI,
            .drmFormatModifierCount = static_cast<u32>(imageModifiers.size()),
            .pDrmFormatModifiers = imageModifiers.data(),
        };

        imgCI.pNext = &modListCI;

        img.image = ctx.dev.createImage(imgCI);
    } else if (haveDmaBuf && handleType == HandleType::DmaBuf) {
        extImgCI.handleTypes = vk::ExternalMemoryHandleTypeFlagBits::eDmaBufEXT;
        imgCI.pNext = &extImgCI;
        imgCI.tiling = vk::ImageTiling::eLinear;

        img.image = ctx.dev.createImage(imgCI);
    } else if (handleType == HandleType::OpaqueFd) {
        extImgCI.handleTypes = vk::ExternalMemoryHandleTypeFlagBits::eOpaqueFd;
        imgCI.pNext = &extImgCI;
        imgCI.tiling = vk::ImageTiling::eOptimal;

        img.image = ctx.dev.createImage(imgCI);
    } else if (handleType == HandleType::None) {
        imgCI.tiling = vk::ImageTiling::eOptimal;

        img.image = ctx.dev.createImage(imgCI);
    }

    vk::MemoryDedicatedRequirements dedicatedReqs {};
    vk::MemoryRequirements2 memReqs {
        .pNext = &dedicatedReqs,
    };

    vk::ImageMemoryRequirementsInfo2 memReqInfo {
        .image = img.image,
    };
    ctx.dev.getImageMemoryRequirements2(&memReqInfo, &memReqs);

    vk::MemoryDedicatedAllocateInfo memDedicatedInfo {
        .image = img.image,
    };
    vk::ExportMemoryAllocateInfo memExportInfo {
        .handleTypes = extImgCI.handleTypes,
    };
    if (handleType != HandleType::None)
        memDedicatedInfo.pNext = &memExportInfo;

    vk::MemoryAllocateInfo memAllocInfo {
        .pNext = &memDedicatedInfo,
        .allocationSize = memReqs.memoryRequirements.size,
        .memoryTypeIndex = memoryTypeIndex(
            ctx, vk::MemoryPropertyFlagBits::eDeviceLocal, memReqs.memoryRequirements.memoryTypeBits
        ),
    };
    img.memory = ctx.dev.allocateMemory(memAllocInfo);

    vk::BindImageMemoryInfo bindImgInfo {
        .image = img.image,
        .memory = img.memory,
        .memoryOffset = 0,
    };
    ctx.dev.bindImageMemory2(bindImgInfo);

    // Only the dma-buf export produces a DRM descriptor; the opaque-fd path
    // leaves out.drm zeroed.
    if (handleType == HandleType::DmaBuf) {
        vk::MemoryGetFdInfoKHR memFdInfo {
            .memory = img.memory,
            .handleType = vk::ExternalMemoryHandleTypeFlagBits::eDmaBufEXT,
        };
        out.drm.fd = ctx.dev.getMemoryFdKHR(memFdInfo, ctx.dispatch);

        if (haveDrmModifiers) {
            auto imgDrmProps
                = ctx.dev.getImageDrmFormatModifierPropertiesEXT(img.image, ctx.dispatch);

            out.drm.modifier = imgDrmProps.drmFormatModifier;
            for (auto prop : modProps) {
                if (prop.drmFormatModifier == out.drm.modifier)
                    out.drm.planes = prop.drmFormatModifierPlaneCount;
            }
        } else {
            out.drm.modifier = DRM_FORMAT_MOD_INVALID;
            out.drm.planes = 1;
        }

        for (u32 i = 0; i < out.drm.planes; ++i) {
            vk::ImageSubresource subresource {
                .aspectMask = static_cast<vk::ImageAspectFlags>(
                    haveDrmModifiers ? VK_IMAGE_ASPECT_MEMORY_PLANE_0_BIT_EXT << i
                                     : VK_IMAGE_ASPECT_COLOR_BIT
                ),
            };
            auto layout = ctx.dev.getImageSubresourceLayout(img.image, subresource);
            out.drm.strides[i] = layout.rowPitch;
            out.drm.offsets[i] = layout.offset;
        }

        out.drm.format = to_drm_format(imgCI.format);
    }

    vk::ImageViewCreateInfo imgViewCI {
        .image = img.image,
        .viewType = vk::ImageViewType::e2D,
        .format = imgCI.format,
        .components {
            .r = vk::ComponentSwizzle::eIdentity,
            .g = vk::ComponentSwizzle::eIdentity,
            .b = vk::ComponentSwizzle::eIdentity,
            .a = vk::ComponentSwizzle::eIdentity,
        },
        .subresourceRange {
            .aspectMask = vk::ImageAspectFlagBits::eColor,
            .baseMipLevel = 0,
            .levelCount = imgCI.mipLevels,
            .baseArrayLayer = 0,
            .layerCount = imgCI.arrayLayers,
        },
    };
    img.view = ctx.dev.createImageView(imgViewCI);

    // out.semaphore = ctx.dev.createSemaphore({});

    out.imageCI = imgCI;
    out.size = memReqs.memoryRequirements.size;
    out.image = img;

    return out;
}

namespace alvr::render {

// Matches layer.comp's push constant block (std430).
struct LayerPush {
    int32_t dstOffset[2];
    int32_t dstSize[2];
    float uvRect[4];
    int32_t opaque;
};

Renderer::Renderer(
    VkContext const& vkCtx, RendererCreateInfo& createInfo, std::vector<PipelineCreateInfo> pipeCIs
) {
    eyeExtent = createInfo.inputEyeExtent;
    outExtent = createInfo.outputExtent;

    // SteamVR's sRGB textures are sampled through UNORM views (see
    // importTexture), so everything downstream is UNORM: the shaders
    // imageStore through rgba8 and the encoder consumes raw bytes with no
    // notion of sRGB.
    vk::Format unormFormat = createInfo.format;
    if (unormFormat == vk::Format::eR8G8B8A8Srgb)
        unormFormat = vk::Format::eR8G8B8A8Unorm;

    if (unormFormat == vk::Format::eB8G8R8A8Srgb)
        unormFormat = vk::Format::eB8G8R8A8Unorm;

    // The working images hold both eyes side by side at the size of the
    // first layer 0 texture set; later sets of another size (SteamVR Home's)
    // are scaled into it by layer.comp.
    vk::ImageCreateInfo stagingImgCI {
        .imageType = vk::ImageType::e2D,
        .format = unormFormat,
        .extent = {
            .width = eyeExtent.width * 2,
            .height = eyeExtent.height,
            .depth = 1,
        },
        .mipLevels = 1,
        .arrayLayers = 1,
        .samples = vk::SampleCountFlagBits::e1,
        .usage = vk::ImageUsageFlagBits::eTransferSrc | vk::ImageUsageFlagBits::eInputAttachment
            | vk::ImageUsageFlagBits::eSampled | vk::ImageUsageFlagBits::eTransferDst
            | vk::ImageUsageFlagBits::eStorage,
        .sharingMode = vk::SharingMode::eExclusive,
        .initialLayout = vk::ImageLayout::eUndefined,
    };

    for (auto& img : stagingImgs) {
        img = createImage(vkCtx, stagingImgCI);
    }

    // VAAPI imports through DRM-PRIME and wants a dma-buf; NvEnc imports
    // through CUDA interop and wants an opaque fd.
    output = createOutputImage(
        vkCtx,
        createInfo.outputExtent,
        stagingImgCI.format,
        vkCtx.meta.vendor == Vendor::Nvidia ? HandleType::OpaqueFd : HandleType::DmaBuf
    );

    vk::QueryPoolCreateInfo poolCI {
        .queryType = vk::QueryType::eTimestamp,
        .queryCount = 2,
    };
    timestampPool = vkCtx.dev.createQueryPool(poolCI);

    vk::CommandPoolCreateInfo cmdPoolCI {
        .flags = vk::CommandPoolCreateFlagBits::eResetCommandBuffer,
        .queueFamilyIndex = vkCtx.meta.queueFamily,
    };
    cmdPool = vkCtx.dev.createCommandPool(cmdPoolCI);

    vk::CommandBufferAllocateInfo cmdBufAllocInfo {
        .commandPool = cmdPool,
        .level = vk::CommandBufferLevel::ePrimary,
        .commandBufferCount = 1,
    };
    cmdBuf = vkCtx.dev.allocateCommandBuffers(cmdBufAllocInfo)[0];

    vk::SamplerCreateInfo samplerCI {
        .magFilter = vk::Filter::eLinear,
        .minFilter = vk::Filter::eLinear,
        .mipmapMode = vk::SamplerMipmapMode::eLinear,
        .addressModeU = vk::SamplerAddressMode::eRepeat,
        .addressModeV = vk::SamplerAddressMode::eRepeat,
        .addressModeW = vk::SamplerAddressMode::eRepeat,

        .anisotropyEnable = true,
        .maxAnisotropy = 16.f,

        .borderColor = vk::BorderColor::eFloatTransparentBlack,
    };
    sampler = vkCtx.dev.createSampler(samplerCI);

    vk::DescriptorSetLayoutBinding descBindings[2];
    descBindings[0] = {
        .binding = 0,
        .descriptorType = vk::DescriptorType::eCombinedImageSampler,
        .descriptorCount = 1,
        .stageFlags = vk::ShaderStageFlagBits::eCompute,
        .pImmutableSamplers = &sampler,
    };
    descBindings[1] = {
        .binding = 1,
        .descriptorType = vk::DescriptorType::eStorageImage,
        .descriptorCount = 1,
        .stageFlags = vk::ShaderStageFlagBits::eCompute,
    };

    vk::DescriptorSetLayoutCreateInfo descLayoutCI {
        .flags = vk::DescriptorSetLayoutCreateFlagBits::ePushDescriptorKHR,
        .bindingCount = 2,
        .pBindings = descBindings,
    };
    descLayout = vkCtx.dev.createDescriptorSetLayout(descLayoutCI);

    pipes.reserve(pipeCIs.size());
    for (auto& pipeCI : pipeCIs) {
        pipes.emplace_back(vkCtx, descLayout, pipeCI);
    }

    vk::SemaphoreTypeCreateInfo timelineCI {
        .semaphoreType = vk::SemaphoreType::eTimeline,
        .initialValue = 0,
    };
    vk::SemaphoreCreateInfo semCI {
        .pNext = &timelineCI,
    };
    // renderFinishedSem = vkCtx.dev.createSemaphore(semCI);

    fence = vkCtx.dev.createFence({});

    // Copied out of the embedded bytes, which carry no 4-byte alignment.
    std::vector<u32> layerCode((LAYER_SHADER_COMP_SPV_LEN + 3) / 4);
    memcpy(layerCode.data(), LAYER_SHADER_COMP_SPV_PTR, LAYER_SHADER_COMP_SPV_LEN);
    vk::ShaderModuleCreateInfo layerShaderCI {
        .codeSize = LAYER_SHADER_COMP_SPV_LEN,
        .pCode = layerCode.data(),
    };
    layerShader = vkCtx.dev.createShaderModule(layerShaderCI);
    vk::PushConstantRange layerPushRange {
        .stageFlags = vk::ShaderStageFlagBits::eCompute,
        .offset = 0,
        .size = sizeof(LayerPush),
    };
    vk::PipelineLayoutCreateInfo layerPipeLayoutCI {
        .setLayoutCount = 1,
        .pSetLayouts = &descLayout,
        .pushConstantRangeCount = 1,
        .pPushConstantRanges = &layerPushRange,
    };
    layerPipeLayout = vkCtx.dev.createPipelineLayout(layerPipeLayoutCI);
    vk::ComputePipelineCreateInfo layerPipeCI {
        .stage = {
            .stage = vk::ShaderStageFlagBits::eCompute,
            .module = layerShader,
            .pName = "main",
        },
        .layout = layerPipeLayout,
    };
    auto [layerPipeResult, layerPipes] = vkCtx.dev.createComputePipelines({}, layerPipeCI);
    if (layerPipeResult != vk::Result::eSuccess) {
        throw std::runtime_error("Could not create the layer pipeline");
    }
    layerPipe = layerPipes[0];
}

void Renderer::render(VkContext& vkCtx, std::vector<Layer> const& layers) {
    if (layers.empty()) {
        return;
    }
    // Imports happen before recording: they create Vulkan objects.
    std::vector<std::vector<Image*>> sources;
    for (auto const& layer : layers) {
        sources.push_back({ &importTexture(vkCtx, layer, layer.eyes[0]),
                            &importTexture(vkCtx, layer, layer.eyes[1]) });
    }

    vk::CommandBufferBeginInfo beginInfo {};
    cmdBuf.begin(beginInfo);

    cmdBuf.resetQueryPool(timestampPool, 0, 2);
    cmdBuf.writeTimestamp(vk::PipelineStageFlagBits::eTopOfPipe, timestampPool, 0);

    auto flushBarriers = [&](std::vector<vk::ImageMemoryBarrier>& barriers,
                             vk::PipelineStageFlags srcStage,
                             vk::PipelineStageFlags dstStage) {
        if (!barriers.empty()) {
            cmdBuf.pipelineBarrier(srcStage, dstStage, {}, {}, {}, barriers);
            barriers.clear();
        }
    };

    auto transitionImage = [&](Image& image,
                               vk::ImageLayout newLayout,
                               vk::AccessFlags srcAccessMask,
                               vk::AccessFlags dstAccessMask,
                               std::vector<vk::ImageMemoryBarrier>& barriers) {
        if (image.layout == newLayout) {
            return;
        }

        barriers.push_back(vk::ImageMemoryBarrier {
            .srcAccessMask = srcAccessMask,
            .dstAccessMask = dstAccessMask,
            .oldLayout = image.layout,
            .newLayout = newLayout,
            .srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
            .dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
            .image = image.image,
            .subresourceRange {
                .aspectMask = vk::ImageAspectFlagBits::eColor,
                .baseMipLevel = 0,
                .levelCount = 1,
                .baseArrayLayer = 0,
                .layerCount = 1,
            },
        });
        image.layout = newLayout;
    };

    std::vector<vk::ImageMemoryBarrier> barriers;

    // Draw layer 0 into the first working image, then blend each further
    // layer over it, in submission order. layer.comp scales each eye's valid
    // sub-rectangle to the eye; with full bounds and the same size it is a
    // plain copy.
    Image* stageOut = &stagingImgs[0];
    for (auto const& eyes : sources) {
        for (Image* source : eyes) {
            transitionImage(
                *source,
                vk::ImageLayout::eGeneral,
                vk::AccessFlagBits::eNone,
                vk::AccessFlagBits::eShaderRead,
                barriers
            );
        }
    }
    transitionImage(
        *stageOut,
        vk::ImageLayout::eGeneral,
        vk::AccessFlagBits::eNone,
        vk::AccessFlagBits::eShaderWrite,
        barriers
    );
    flushBarriers(
        barriers, vk::PipelineStageFlagBits::eAllCommands, vk::PipelineStageFlagBits::eComputeShader
    );

    cmdBuf.bindPipeline(vk::PipelineBindPoint::eCompute, layerPipe);
    for (usize layerIdx = 0; layerIdx < layers.size(); ++layerIdx) {
        if (layerIdx > 0) {
            // A blend reads what the previous layer wrote.
            vk::MemoryBarrier memBarrier {
                .srcAccessMask = vk::AccessFlagBits::eShaderWrite,
                .dstAccessMask = vk::AccessFlagBits::eShaderRead | vk::AccessFlagBits::eShaderWrite,
            };
            cmdBuf.pipelineBarrier(
                vk::PipelineStageFlagBits::eComputeShader,
                vk::PipelineStageFlagBits::eComputeShader,
                {},
                memBarrier,
                {},
                {}
            );
        }
        for (int eye = 0; eye < 2; ++eye) {
            drawLayerEye(
                vkCtx, *sources[layerIdx][eye], eye, layers[layerIdx].eyes[eye], layerIdx == 0
            );
        }
    }
    Image* prev = stageOut;

    PushConstants const pushConstants {
        .warp = warpParams,
        .centerShifts = foveationCenterShifts,
    };

    for (usize pipeIdx = 0; pipeIdx < pipes.size(); ++pipeIdx) {
        Image* nextOut = pipeIdx + 1 == pipes.size()
            ? &output.image
            : &stagingImgs[(pipeIdx + 1) % StagingImgCount];
        vk::Extent2D targetExtent = pipeIdx + 1 == pipes.size() ? outExtent
                                                                : vk::Extent2D {
                                                                      .width = eyeExtent.width * 2,
                                                                      .height = eyeExtent.height,
                                                                  };

        transitionImage(
            *prev,
            vk::ImageLayout::eShaderReadOnlyOptimal,
            vk::AccessFlagBits::eTransferWrite | vk::AccessFlagBits::eShaderWrite,
            vk::AccessFlagBits::eShaderRead,
            barriers
        );
        transitionImage(
            *nextOut,
            vk::ImageLayout::eGeneral,
            vk::AccessFlagBits::eNone,
            vk::AccessFlagBits::eShaderWrite,
            barriers
        );

        flushBarriers(
            barriers,
            vk::PipelineStageFlagBits::eTransfer | vk::PipelineStageFlagBits::eComputeShader,
            vk::PipelineStageFlagBits::eComputeShader
        );

        pipes[pipeIdx].render(
            vkCtx, cmdBuf, prev->view, nextOut->view, targetExtent, pushConstants
        );
        prev = nextOut;
    }

    cmdBuf.writeTimestamp(vk::PipelineStageFlagBits::eBottomOfPipe, timestampPool, 1);

    cmdBuf.end();

    vk::SubmitInfo submitInfo {
        .commandBufferCount = 1,
        .pCommandBuffers = &cmdBuf,
    };

    vkCtx.useQueue([&](auto& queue) { queue.submit(submitInfo, fence); });
    assert(vkCtx.dev.waitForFences(fence, true, UINT64_MAX) == vk::Result::eSuccess);
    vkCtx.dev.resetFences(fence);
}

// Imports one of SteamVR's textures the first time its handle is seen, the
// way the textures were always imported here: OPAQUE_FD memory, SteamVR's
// real format with MUTABLE_FORMAT and no format list (as the driver created
// it in CreateSwapTextureSet, so the memory layout matches the exporter's),
// and a UNORM view of an sRGB format. No new requirement on any vendor.
Image& Renderer::importTexture(VkContext& vkCtx, Layer const& layer, LayerEye const& eye) {
    auto it = textures.find(eye.handle);
    if (it != textures.end()) {
        return it->second;
    }
    int fd = dup(eye.fd);
    if (fd < 0) {
        throw std::runtime_error("Could not duplicate a SteamVR texture fd");
    }
    vk::Format unormFormat = layer.format;
    if (unormFormat == vk::Format::eR8G8B8A8Srgb)
        unormFormat = vk::Format::eR8G8B8A8Unorm;
    if (unormFormat == vk::Format::eB8G8R8A8Srgb)
        unormFormat = vk::Format::eB8G8R8A8Unorm;
    vk::ImageCreateInfo textureCI {
        .flags = vk::ImageCreateFlagBits::eMutableFormat,
        .imageType = vk::ImageType::e2D,
        .format = layer.format,
        .extent = { .width = layer.extent.width, .height = layer.extent.height, .depth = 1 },
        .mipLevels = 1,
        .arrayLayers = 1,
        .samples = vk::SampleCountFlagBits::e1,
        .usage = vk::ImageUsageFlagBits::eTransferSrc | vk::ImageUsageFlagBits::eInputAttachment
            | vk::ImageUsageFlagBits::eSampled,
        .sharingMode = vk::SharingMode::eExclusive,
        .initialLayout = vk::ImageLayout::eUndefined,
    };
    return textures.emplace(eye.handle, createImage(vkCtx, textureCI, fd, unormFormat))
        .first->second;
}

void Renderer::drawLayerEye(
    VkContext& vkCtx, Image& source, int eye, LayerEye const& rect, bool opaque
) {
    vk::DescriptorImageInfo sourceInfo {
        .imageView = source.view,
        .imageLayout = vk::ImageLayout::eGeneral,
    };
    vk::DescriptorImageInfo targetInfo {
        .imageView = stagingImgs[0].view,
        .imageLayout = vk::ImageLayout::eGeneral,
    };
    vk::WriteDescriptorSet writes[2];
    writes[0] = {
        .dstBinding = 0,
        .descriptorCount = 1,
        .descriptorType = vk::DescriptorType::eCombinedImageSampler,
        .pImageInfo = &sourceInfo,
    };
    writes[1] = {
        .dstBinding = 1,
        .descriptorCount = 1,
        .descriptorType = vk::DescriptorType::eStorageImage,
        .pImageInfo = &targetInfo,
    };
    cmdBuf.pushDescriptorSetKHR(
        vk::PipelineBindPoint::eCompute, layerPipeLayout, 0, writes, vkCtx.dispatch
    );
    LayerPush push {
        .dstOffset = { int32_t(eye * eyeExtent.width), 0 },
        .dstSize = { int32_t(eyeExtent.width), int32_t(eyeExtent.height) },
        .uvRect = { rect.uMin, rect.vMin, rect.uMax, rect.vMax },
        .opaque = opaque ? 1 : 0,
    };
    cmdBuf.pushConstants(
        layerPipeLayout, vk::ShaderStageFlagBits::eCompute, 0, sizeof(push), &push
    );
    cmdBuf.dispatch((eyeExtent.width + 7) / 8, (eyeExtent.height + 7) / 8, 1);
}

void Renderer::releaseTextures(VkContext const& vkCtx, std::vector<u64> const& handles) {
    for (u64 handle : handles) {
        auto it = textures.find(handle);
        if (it != textures.end()) {
            it->second.destroy(vkCtx);
            textures.erase(it);
        }
    }
}

void Renderer::destroy(VkContext const& ctx) {
    // ctx.dev.destroy(renderFinishedSem);

    for (auto& pipe : pipes) {
        pipe.destroy(ctx);
    }

    ctx.dev.destroy(fence);
    ctx.dev.destroy(descLayout);
    ctx.dev.destroy(sampler);

    ctx.dev.destroy(cmdPool);
    ctx.dev.destroy(timestampPool);

    output.destroy(ctx);

    for (auto& img : stagingImgs) {
        img.destroy(ctx);
    }

    for (auto& [handle, img] : textures) {
        img.destroy(ctx);
    }
    textures.clear();
    ctx.dev.destroy(layerPipe);
    ctx.dev.destroy(layerPipeLayout);
    ctx.dev.destroy(layerShader);
}

}
