#include "Renderer.h"

#include <algorithm>
#include <array>
#include <cassert>
#include <cstring>
#include <fstream>
#include <iostream>

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

struct Vertex {
    float position[2];
};

static uint32_t to_drm_format(VkFormat format) {
    switch (format) {
    case VK_FORMAT_B8G8R8A8_UNORM:
        return DRM_FORMAT_ARGB8888;
    case VK_FORMAT_R8G8B8A8_UNORM:
        return DRM_FORMAT_ABGR8888;
    default:
        std::cerr << "Unsupported format " << format << std::endl;
        return DRM_FORMAT_INVALID;
    }
}

static bool filter_modifier(uint64_t modifier) {
    if (IS_AMD_FMT_MOD(modifier)) {
        // DCC not supported as encode input
        if (AMD_FMT_MOD_GET(DCC, modifier)) {
            return false;
        }
    }
    return true;
}

Renderer::Renderer(
    const VkInstance& inst,
    const VkDevice& dev,
    const VkPhysicalDevice& physDev,
    uint32_t queueIdx,
    const std::vector<const char*>& devExtensions
)
    : m_inst(inst)
    , m_dev(dev)
    , m_physDev(physDev)
    , m_queueFamilyIndex(queueIdx) {
    auto checkExtension = [devExtensions](const char* name) {
        return std::find_if(
                   devExtensions.begin(),
                   devExtensions.end(),
                   [name](const char* ext) { return strcmp(ext, name) == 0; }
               )
            != devExtensions.end();
    };
    d.haveDmaBuf = checkExtension(VK_EXT_EXTERNAL_MEMORY_DMA_BUF_EXTENSION_NAME);
    d.haveDrmModifiers = checkExtension(VK_EXT_IMAGE_DRM_FORMAT_MODIFIER_EXTENSION_NAME);
    d.haveCalibratedTimestamps = checkExtension(VK_EXT_CALIBRATED_TIMESTAMPS_EXTENSION_NAME);

    if (!checkExtension(VK_KHR_PUSH_DESCRIPTOR_EXTENSION_NAME)) {
        throw std::runtime_error("Vulkan: Required extension " VK_KHR_PUSH_DESCRIPTOR_EXTENSION_NAME
                                 " not available");
    }

#define VK_LOAD_PFN(name) d.name = (PFN_##name)vkGetInstanceProcAddr(m_inst, #name)
    VK_LOAD_PFN(vkImportSemaphoreFdKHR);
    VK_LOAD_PFN(vkGetMemoryFdKHR);
    VK_LOAD_PFN(vkGetMemoryFdPropertiesKHR);
    VK_LOAD_PFN(vkGetImageDrmFormatModifierPropertiesEXT);
    VK_LOAD_PFN(vkGetCalibratedTimestampsEXT);
    VK_LOAD_PFN(vkCmdPushDescriptorSetKHR);
#undef VK_LOAD_PFN

    VkPhysicalDeviceProperties props = {};
    vkGetPhysicalDeviceProperties(m_physDev, &props);
    m_timestampPeriod = props.limits.timestampPeriod;
}

Renderer::~Renderer() {
    vkDeviceWaitIdle(m_dev);

    for (const InputImage& image : m_images) {
        vkDestroyImageView(m_dev, image.view, nullptr);
        vkDestroyImage(m_dev, image.image, nullptr);
        vkFreeMemory(m_dev, image.memory, nullptr);
        vkDestroySemaphore(m_dev, image.semaphore, nullptr);
    }

    for (const StagingImage& image : m_stagingImages) {
        vkDestroyImageView(m_dev, image.view, nullptr);
        vkDestroyImage(m_dev, image.image, nullptr);
        vkFreeMemory(m_dev, image.memory, nullptr);
    }

    vkDestroyImageView(m_dev, m_output.view, nullptr);
    vkDestroyImage(m_dev, m_output.image, nullptr);
    vkFreeMemory(m_dev, m_output.memory, nullptr);
    vkDestroySemaphore(m_dev, m_output.semaphore, nullptr);

    vkDestroyQueryPool(m_dev, m_queryPool, nullptr);
    vkDestroyCommandPool(m_dev, m_commandPool, nullptr);
    vkDestroySampler(m_dev, m_sampler, nullptr);
    vkDestroyDescriptorSetLayout(m_dev, m_descriptorLayout, nullptr);
    vkDestroyFence(m_dev, m_fence, nullptr);
}

void Renderer::Startup(uint32_t width, uint32_t height, VkFormat format) {
    m_format = format;
    m_imageSize.width = width;
    m_imageSize.height = height;

    vkGetDeviceQueue(m_dev, m_queueFamilyIndex, 0, &m_queue);

    // Timestamp query
    VkQueryPoolCreateInfo queryPoolInfo = {};
    queryPoolInfo.sType = VK_STRUCTURE_TYPE_QUERY_POOL_CREATE_INFO;
    queryPoolInfo.queryType = VK_QUERY_TYPE_TIMESTAMP;
    queryPoolInfo.queryCount = 2;
    VK_CHECK(vkCreateQueryPool(m_dev, &queryPoolInfo, nullptr, &m_queryPool));

    // Command buffer
    VkCommandPoolCreateInfo cmdPoolInfo = {};
    cmdPoolInfo.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
    cmdPoolInfo.queueFamilyIndex = m_queueFamilyIndex;
    cmdPoolInfo.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;
    VK_CHECK(vkCreateCommandPool(m_dev, &cmdPoolInfo, nullptr, &m_commandPool));

    VkCommandBufferAllocateInfo commandBufferInfo = {};
    commandBufferInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
    commandBufferInfo.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    commandBufferInfo.commandPool = m_commandPool;
    commandBufferInfo.commandBufferCount = 1;
    VK_CHECK(vkAllocateCommandBuffers(m_dev, &commandBufferInfo, &m_commandBuffer));

    // Sampler
    VkSamplerCreateInfo samplerInfo = {};
    samplerInfo.sType = VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO;
    samplerInfo.magFilter = VK_FILTER_LINEAR;
    samplerInfo.minFilter = VK_FILTER_LINEAR;
    samplerInfo.mipmapMode = VK_SAMPLER_MIPMAP_MODE_NEAREST;
    samplerInfo.addressModeU = VK_SAMPLER_ADDRESS_MODE_REPEAT;
    samplerInfo.addressModeV = VK_SAMPLER_ADDRESS_MODE_REPEAT;
    samplerInfo.addressModeW = VK_SAMPLER_ADDRESS_MODE_REPEAT;
    samplerInfo.anisotropyEnable = VK_TRUE;
    samplerInfo.maxAnisotropy = 16.0f;
    samplerInfo.borderColor = VK_BORDER_COLOR_FLOAT_TRANSPARENT_BLACK;
    VK_CHECK(vkCreateSampler(m_dev, &samplerInfo, nullptr, &m_sampler));

    // Descriptors
    VkDescriptorSetLayoutBinding descriptorBindings[2] = {};
    descriptorBindings[0].descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorBindings[0].stageFlags = VK_SHADER_STAGE_COMPUTE_BIT;
    descriptorBindings[0].descriptorCount = 1;
    descriptorBindings[0].pImmutableSamplers = &m_sampler;
    descriptorBindings[0].binding = 0;
    descriptorBindings[1].descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
    descriptorBindings[1].stageFlags = VK_SHADER_STAGE_COMPUTE_BIT;
    descriptorBindings[1].descriptorCount = 1;
    descriptorBindings[1].binding = 1;

    VkDescriptorSetLayoutCreateInfo descriptorSetLayoutInfo = {};
    descriptorSetLayoutInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO;
    descriptorSetLayoutInfo.flags = VK_DESCRIPTOR_SET_LAYOUT_CREATE_PUSH_DESCRIPTOR_BIT_KHR;
    descriptorSetLayoutInfo.bindingCount = 2;
    descriptorSetLayoutInfo.pBindings = descriptorBindings;
    VK_CHECK(
        vkCreateDescriptorSetLayout(m_dev, &descriptorSetLayoutInfo, nullptr, &m_descriptorLayout)
    );

    // Fence
    VkFenceCreateInfo fenceInfo = {};
    fenceInfo.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
    VK_CHECK(vkCreateFence(m_dev, &fenceInfo, nullptr, &m_fence));
}

void Renderer::AddImage(
    VkImageCreateInfo imageInfo, size_t memoryIndex, int imageFd, int semaphoreFd
) {
    VkExternalMemoryImageCreateInfo extMemImageInfo = {};
    extMemImageInfo.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO;
    extMemImageInfo.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
    imageInfo.pNext = &extMemImageInfo;
    imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    VkImage image;
    VK_CHECK(vkCreateImage(m_dev, &imageInfo, nullptr, &image));

    VkMemoryRequirements req;
    vkGetImageMemoryRequirements(m_dev, image, &req);

    VkMemoryDedicatedAllocateInfo dedicatedMemInfo = {};
    dedicatedMemInfo.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO;
    dedicatedMemInfo.image = image;

    VkImportMemoryFdInfoKHR importMemInfo = {};
    importMemInfo.sType = VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR;
    importMemInfo.pNext = &dedicatedMemInfo;
    importMemInfo.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
    importMemInfo.fd = imageFd;

    VkMemoryAllocateInfo memAllocInfo = {};
    memAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    memAllocInfo.pNext = &importMemInfo;
    memAllocInfo.allocationSize = req.size;
    memAllocInfo.memoryTypeIndex = memoryIndex;

    VkDeviceMemory mem;
    VK_CHECK(vkAllocateMemory(m_dev, &memAllocInfo, nullptr, &mem));
    VK_CHECK(vkBindImageMemory(m_dev, image, mem, 0));

    VkSemaphoreTypeCreateInfo timelineInfo = {};
    timelineInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO;
    timelineInfo.semaphoreType = VK_SEMAPHORE_TYPE_TIMELINE;

    VkSemaphoreCreateInfo semInfo = {};
    semInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    semInfo.pNext = &timelineInfo;
    VkSemaphore semaphore;
    VK_CHECK(vkCreateSemaphore(m_dev, &semInfo, nullptr, &semaphore));

    VkImportSemaphoreFdInfoKHR impSemInfo = {};
    impSemInfo.sType = VK_STRUCTURE_TYPE_IMPORT_SEMAPHORE_FD_INFO_KHR;
    impSemInfo.semaphore = semaphore;
    impSemInfo.handleType = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT;
    impSemInfo.fd = semaphoreFd;
    VK_CHECK(d.vkImportSemaphoreFdKHR(m_dev, &impSemInfo));

    VkImageViewCreateInfo viewInfo = {};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = imageInfo.format;
    viewInfo.image = image;
    viewInfo.subresourceRange = {};
    viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    viewInfo.subresourceRange.baseMipLevel = 0;
    viewInfo.subresourceRange.levelCount = 1;
    viewInfo.subresourceRange.baseArrayLayer = 0;
    viewInfo.subresourceRange.layerCount = 1;
    viewInfo.components.r = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.g = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.b = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.a = VK_COMPONENT_SWIZZLE_IDENTITY;
    VkImageView view;
    VK_CHECK(vkCreateImageView(m_dev, &viewInfo, nullptr, &view));

    m_images.push_back({ image, VK_IMAGE_LAYOUT_UNDEFINED, mem, semaphore, view });
}

void Renderer::AddPipeline(RenderPipeline* pipeline) {
    pipeline->Build();
    m_pipelines.push_back(pipeline);

    if (m_pipelines.size() > 1 && m_stagingImages.size() < 2) {
        addStagingImage(m_imageSize.width, m_imageSize.height);
    }
}

void Renderer::CreateOutput(uint32_t width, uint32_t height, ExternalHandle handle) {
    m_output.imageInfo = {};
    m_output.imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    m_output.imageInfo.imageType = VK_IMAGE_TYPE_2D;
    m_output.imageInfo.format = m_format;
    m_output.imageInfo.extent.width = width;
    m_output.imageInfo.extent.height = height;
    m_output.imageInfo.extent.depth = 1;
    m_output.imageInfo.mipLevels = 1;
    m_output.imageInfo.arrayLayers = 1;
    m_output.imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
    m_output.imageInfo.usage = VK_IMAGE_USAGE_STORAGE_BIT | VK_IMAGE_USAGE_SAMPLED_BIT;
    m_output.imageInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    m_output.imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;

    std::vector<VkDrmFormatModifierPropertiesEXT> modifierProps;

    VkExternalMemoryImageCreateInfo extMemImageInfo = {};
    extMemImageInfo.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO;

    if (d.haveDrmModifiers && handle == ExternalHandle::DmaBuf) {
        VkImageDrmFormatModifierListCreateInfoEXT modifierListInfo = {};
        modifierListInfo.sType = VK_STRUCTURE_TYPE_IMAGE_DRM_FORMAT_MODIFIER_LIST_CREATE_INFO_EXT;

        m_output.imageInfo.pNext = &modifierListInfo;
        m_output.imageInfo.tiling = VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT;

        VkDrmFormatModifierPropertiesListEXT modifierPropsList = {};
        modifierPropsList.sType = VK_STRUCTURE_TYPE_DRM_FORMAT_MODIFIER_PROPERTIES_LIST_EXT;

        VkFormatProperties2 formatProps = {};
        formatProps.sType = VK_STRUCTURE_TYPE_FORMAT_PROPERTIES_2;
        formatProps.pNext = &modifierPropsList;
        vkGetPhysicalDeviceFormatProperties2(m_physDev, m_output.imageInfo.format, &formatProps);

        modifierProps.resize(modifierPropsList.drmFormatModifierCount);
        modifierPropsList.pDrmFormatModifierProperties = modifierProps.data();
        vkGetPhysicalDeviceFormatProperties2(m_physDev, m_output.imageInfo.format, &formatProps);

        std::vector<uint64_t> imageModifiers;
        std::cout << "Available modifiers:" << std::endl;
        for (const VkDrmFormatModifierPropertiesEXT& prop : modifierProps) {
            std::cout << "modifier: " << prop.drmFormatModifier
                      << " planes: " << prop.drmFormatModifierPlaneCount << std::endl;
            if (!filter_modifier(prop.drmFormatModifier)) {
                std::cout << " filtered" << std::endl;
                continue;
            }

            VkPhysicalDeviceImageDrmFormatModifierInfoEXT modInfo = {};
            modInfo.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_IMAGE_DRM_FORMAT_MODIFIER_INFO_EXT;
            modInfo.drmFormatModifier = prop.drmFormatModifier;
            modInfo.sharingMode = m_output.imageInfo.sharingMode;
            modInfo.queueFamilyIndexCount = m_output.imageInfo.queueFamilyIndexCount;
            modInfo.pQueueFamilyIndices = m_output.imageInfo.pQueueFamilyIndices;

            VkPhysicalDeviceImageFormatInfo2 formatInfo = {};
            formatInfo.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_IMAGE_FORMAT_INFO_2;
            formatInfo.pNext = &modInfo;
            formatInfo.format = m_output.imageInfo.format;
            formatInfo.type = m_output.imageInfo.imageType;
            formatInfo.tiling = m_output.imageInfo.tiling;
            formatInfo.usage = m_output.imageInfo.usage;
            formatInfo.flags = m_output.imageInfo.flags;

            VkImageFormatProperties2 imageFormatProps = {};
            imageFormatProps.sType = VK_STRUCTURE_TYPE_IMAGE_FORMAT_PROPERTIES_2;
            imageFormatProps.pNext = NULL;

            VkResult r = vkGetPhysicalDeviceImageFormatProperties2(
                m_physDev, &formatInfo, &imageFormatProps
            );
            if (r == VK_SUCCESS) {
                imageModifiers.push_back(prop.drmFormatModifier);
            }
        }
        modifierListInfo.drmFormatModifierCount = imageModifiers.size();
        modifierListInfo.pDrmFormatModifiers = imageModifiers.data();

        extMemImageInfo.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
        modifierListInfo.pNext = &extMemImageInfo;

        VK_CHECK(vkCreateImage(m_dev, &m_output.imageInfo, nullptr, &m_output.image));
    } else if (d.haveDmaBuf && handle == ExternalHandle::DmaBuf) {
        extMemImageInfo.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
        m_output.imageInfo.pNext = &extMemImageInfo;

        m_output.imageInfo.tiling = VK_IMAGE_TILING_LINEAR;
        VK_CHECK(vkCreateImage(m_dev, &m_output.imageInfo, nullptr, &m_output.image));
    } else if (handle == ExternalHandle::OpaqueFd) {
        extMemImageInfo.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
        m_output.imageInfo.pNext = &extMemImageInfo;

        m_output.imageInfo.tiling = VK_IMAGE_TILING_OPTIMAL;
        VK_CHECK(vkCreateImage(m_dev, &m_output.imageInfo, nullptr, &m_output.image));
    } else {
        m_output.imageInfo.tiling = VK_IMAGE_TILING_OPTIMAL;
        VK_CHECK(vkCreateImage(m_dev, &m_output.imageInfo, nullptr, &m_output.image));
    }

    VkMemoryDedicatedRequirements mdr = {};
    mdr.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_REQUIREMENTS;

    VkMemoryRequirements2 memoryReqs = {};
    memoryReqs.sType = VK_STRUCTURE_TYPE_MEMORY_REQUIREMENTS_2;
    memoryReqs.pNext = &mdr;

    VkImageMemoryRequirementsInfo2 memoryReqsInfo = {};
    memoryReqsInfo.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_REQUIREMENTS_INFO_2;
    memoryReqsInfo.image = m_output.image;
    vkGetImageMemoryRequirements2(m_dev, &memoryReqsInfo, &memoryReqs);
    m_output.size = memoryReqs.memoryRequirements.size;

    VkExportMemoryAllocateInfo memory_export_info = {};
    memory_export_info.sType = VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO;
    memory_export_info.handleTypes = extMemImageInfo.handleTypes;

    VkMemoryDedicatedAllocateInfo memory_dedicated_info = {};
    memory_dedicated_info.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO;
    memory_dedicated_info.image = m_output.image;
    if (handle != ExternalHandle::None) {
        memory_dedicated_info.pNext = &memory_export_info;
    }

    VkMemoryAllocateInfo memi = {};
    memi.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    memi.pNext = &memory_dedicated_info;
    memi.allocationSize = memoryReqs.memoryRequirements.size;
    memi.memoryTypeIndex = memoryTypeIndex(
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memoryReqs.memoryRequirements.memoryTypeBits
    );
    VK_CHECK(vkAllocateMemory(m_dev, &memi, nullptr, &m_output.memory));

    VkBindImageMemoryInfo bimi = {};
    bimi.sType = VK_STRUCTURE_TYPE_BIND_IMAGE_MEMORY_INFO;
    bimi.image = m_output.image;
    bimi.memory = m_output.memory;
    bimi.memoryOffset = 0;
    VK_CHECK(vkBindImageMemory2(m_dev, 1, &bimi));

    // DRM export
    if (d.haveDmaBuf) {
        VkMemoryGetFdInfoKHR memoryGetFdInfo = {};
        memoryGetFdInfo.sType = VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR;
        memoryGetFdInfo.memory = m_output.memory;
        memoryGetFdInfo.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
        VkResult res = d.vkGetMemoryFdKHR(m_dev, &memoryGetFdInfo, &m_output.drm.fd);
        if (res != VK_SUCCESS) {
            std::cout << "vkGetMemoryFdKHR " << result_to_str(res) << std::endl;
        } else {
            if (d.haveDrmModifiers) {
                VkImageDrmFormatModifierPropertiesEXT imageDrmProps = {};
                imageDrmProps.sType = VK_STRUCTURE_TYPE_IMAGE_DRM_FORMAT_MODIFIER_PROPERTIES_EXT;
                d.vkGetImageDrmFormatModifierPropertiesEXT(m_dev, m_output.image, &imageDrmProps);
                if (res != VK_SUCCESS) {
                    std::cout << "vkGetImageDrmFormatModifierPropertiesEXT " << result_to_str(res)
                              << std::endl;
                } else {
                    m_output.drm.modifier = imageDrmProps.drmFormatModifier;
                    for (VkDrmFormatModifierPropertiesEXT prop : modifierProps) {
                        if (prop.drmFormatModifier == m_output.drm.modifier) {
                            m_output.drm.planes = prop.drmFormatModifierPlaneCount;
                        }
                    }
                }
            } else {
                m_output.drm.modifier = DRM_FORMAT_MOD_INVALID;
                m_output.drm.planes = 1;
            }

            for (uint32_t i = 0; i < m_output.drm.planes; i++) {
                VkImageSubresource subresource = {};
                if (d.haveDrmModifiers) {
                    subresource.aspectMask = VK_IMAGE_ASPECT_MEMORY_PLANE_0_BIT_EXT << i;
                } else {
                    subresource.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
                }
                VkSubresourceLayout layout;
                vkGetImageSubresourceLayout(m_dev, m_output.image, &subresource, &layout);
                m_output.drm.strides[i] = layout.rowPitch;
                m_output.drm.offsets[i] = layout.offset;
            }
        }
        m_output.drm.format = to_drm_format(m_output.imageInfo.format);
    }

    VkImageViewCreateInfo viewInfo = {};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = m_output.imageInfo.format;
    viewInfo.image = m_output.image;
    viewInfo.subresourceRange = {};
    viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    viewInfo.subresourceRange.baseMipLevel = 0;
    viewInfo.subresourceRange.levelCount = 1;
    viewInfo.subresourceRange.baseArrayLayer = 0;
    viewInfo.subresourceRange.layerCount = 1;
    viewInfo.components.r = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.g = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.b = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.a = VK_COMPONENT_SWIZZLE_IDENTITY;
    VK_CHECK(vkCreateImageView(m_dev, &viewInfo, nullptr, &m_output.view));

    VkSemaphoreCreateInfo semInfo = {};
    semInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    VK_CHECK(vkCreateSemaphore(m_dev, &semInfo, nullptr, &m_output.semaphore));
}

void Renderer::ImportOutput(const DrmImage& drm) {
    vkDestroyImageView(m_dev, m_output.view, nullptr);
    vkDestroyImage(m_dev, m_output.image, nullptr);
    vkFreeMemory(m_dev, m_output.memory, nullptr);

    m_output.drm = drm;
    m_output.imageInfo.tiling = VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT;

    VkExternalMemoryImageCreateInfo extMemImageInfo = {};
    extMemImageInfo.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO;
    extMemImageInfo.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
    m_output.imageInfo.pNext = &extMemImageInfo;

    VkSubresourceLayout layouts[4] = {};
    for (uint32_t i = 0; i < drm.planes; ++i) {
        layouts[i].offset = drm.offsets[i];
        layouts[i].rowPitch = drm.strides[i];
    }
    VkImageDrmFormatModifierExplicitCreateInfoEXT modifierInfo = {};
    modifierInfo.sType = VK_STRUCTURE_TYPE_IMAGE_DRM_FORMAT_MODIFIER_EXPLICIT_CREATE_INFO_EXT;
    modifierInfo.drmFormatModifier = drm.modifier;
    modifierInfo.drmFormatModifierPlaneCount = drm.planes;
    modifierInfo.pPlaneLayouts = layouts;
    extMemImageInfo.pNext = &modifierInfo;

    VK_CHECK(vkCreateImage(m_dev, &m_output.imageInfo, NULL, &m_output.image));

    VkMemoryFdPropertiesKHR fdProps = {};
    fdProps.sType = VK_STRUCTURE_TYPE_MEMORY_FD_PROPERTIES_KHR;
    VK_CHECK(d.vkGetMemoryFdPropertiesKHR(
        m_dev, VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT, drm.fd, &fdProps
    ));

    VkImageMemoryRequirementsInfo2 memoryReqsInfo = {};
    memoryReqsInfo.image = m_output.image;
    memoryReqsInfo.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_REQUIREMENTS_INFO_2;

    VkMemoryRequirements2 memoryReqs = {};
    memoryReqs.sType = VK_STRUCTURE_TYPE_MEMORY_REQUIREMENTS_2;
    vkGetImageMemoryRequirements2(m_dev, &memoryReqsInfo, &memoryReqs);

    VkMemoryAllocateInfo memoryAllocInfo = {};
    memoryAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    memoryAllocInfo.allocationSize = memoryReqs.memoryRequirements.size;
    memoryAllocInfo.memoryTypeIndex = memoryTypeIndex(
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memoryReqs.memoryRequirements.memoryTypeBits
    );

    VkImportMemoryFdInfoKHR importMemInfo = {};
    importMemInfo.sType = VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR;
    importMemInfo.fd = drm.fd;
    importMemInfo.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
    memoryAllocInfo.pNext = &importMemInfo;

    VkMemoryDedicatedAllocateInfo dedicatedMemInfo = {};
    dedicatedMemInfo.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO;
    dedicatedMemInfo.image = m_output.image;
    importMemInfo.pNext = &dedicatedMemInfo;

    VK_CHECK(vkAllocateMemory(m_dev, &memoryAllocInfo, NULL, &m_output.memory));

    VkBindImageMemoryInfo bindInfo = {};
    bindInfo.sType = VK_STRUCTURE_TYPE_BIND_IMAGE_MEMORY_INFO;
    bindInfo.image = m_output.image;
    bindInfo.memory = m_output.memory;
    bindInfo.memoryOffset = 0;
    VK_CHECK(vkBindImageMemory2(m_dev, 1, &bindInfo));

    VkImageViewCreateInfo viewInfo = {};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = m_output.imageInfo.format;
    viewInfo.image = m_output.image;
    viewInfo.subresourceRange = {};
    viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    viewInfo.subresourceRange.baseMipLevel = 0;
    viewInfo.subresourceRange.levelCount = 1;
    viewInfo.subresourceRange.baseArrayLayer = 0;
    viewInfo.subresourceRange.layerCount = 1;
    viewInfo.components.r = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.g = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.b = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.a = VK_COMPONENT_SWIZZLE_IDENTITY;
    VK_CHECK(vkCreateImageView(m_dev, &viewInfo, nullptr, &m_output.view));
}

void Renderer::Render(uint32_t index, uint64_t waitValue) {
    if (!m_inputImageCapture.empty()) {
        VkSemaphoreWaitInfo waitInfo = {};
        waitInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_WAIT_INFO;
        waitInfo.semaphoreCount = 1;
        waitInfo.pSemaphores = &m_images[index].semaphore;
        waitInfo.pValues = &waitValue;
        VK_CHECK(vkWaitSemaphores(m_dev, &waitInfo, UINT64_MAX));

        dumpImage(
            m_images[index].image,
            m_images[index].view,
            m_images[index].layout,
            m_imageSize.width,
            m_imageSize.height,
            m_inputImageCapture
        );
        m_inputImageCapture.clear();
    }

    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(m_commandBuffer, &commandBufferBegin));

    vkCmdResetQueryPool(m_commandBuffer, m_queryPool, 0, 2);
    vkCmdWriteTimestamp(m_commandBuffer, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, m_queryPool, 0);

    for (size_t i = 0; i < m_pipelines.size(); ++i) {
        VkRect2D rect = {};
        VkImage in = VK_NULL_HANDLE;
        VkImageView inView = VK_NULL_HANDLE;
        VkImageLayout* inLayout = nullptr;
        VkImage out = VK_NULL_HANDLE;
        VkImageView outView = VK_NULL_HANDLE;
        VkImageLayout* outLayout = nullptr;
        if (i == 0) {
            auto& img = m_images[index];
            in = img.image;
            inView = img.view;
            inLayout = &img.layout;
        } else {
            auto& img = m_stagingImages[(i - 1) % m_stagingImages.size()];
            in = img.image;
            inView = img.view;
            inLayout = &img.layout;
        }
        if (i == m_pipelines.size() - 1) {
            out = m_output.image;
            outView = m_output.view;
            outLayout = &m_output.layout;
            rect.extent.width = m_output.imageInfo.extent.width;
            rect.extent.height = m_output.imageInfo.extent.height;
        } else {
            auto& img = m_stagingImages[i % m_stagingImages.size()];
            out = img.image;
            outView = img.view;
            outLayout = &img.layout;
            rect.extent = m_imageSize;
        }
        VkImageMemoryBarrier imageBarrier = {};
        imageBarrier.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
        imageBarrier.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        imageBarrier.subresourceRange.layerCount = 1;
        imageBarrier.subresourceRange.levelCount = 1;
        std::vector<VkImageMemoryBarrier> imageBarriers;
        if (*inLayout != VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL) {
            imageBarrier.image = in;
            imageBarrier.oldLayout = *inLayout;
            *inLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
            imageBarrier.newLayout = *inLayout;
            imageBarrier.srcAccessMask = 0;
            imageBarrier.dstAccessMask = VK_ACCESS_SHADER_READ_BIT;
            imageBarriers.push_back(imageBarrier);
        }
        if (*outLayout != VK_IMAGE_LAYOUT_GENERAL) {
            imageBarrier.image = out;
            imageBarrier.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
            *outLayout = VK_IMAGE_LAYOUT_GENERAL;
            imageBarrier.newLayout = *outLayout;
            imageBarrier.srcAccessMask = 0;
            imageBarrier.dstAccessMask = VK_ACCESS_SHADER_WRITE_BIT;
            imageBarriers.push_back(imageBarrier);
        }
        if (imageBarriers.size()) {
            vkCmdPipelineBarrier(
                m_commandBuffer,
                VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
                0,
                0,
                nullptr,
                0,
                nullptr,
                imageBarriers.size(),
                imageBarriers.data()
            );
        }
        m_pipelines[i]->Render(inView, outView, rect);
    }

    vkCmdWriteTimestamp(m_commandBuffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, m_queryPool, 1);

    VK_CHECK(vkEndCommandBuffer(m_commandBuffer));

    VkTimelineSemaphoreSubmitInfo timelineInfo = {};
    timelineInfo.sType = VK_STRUCTURE_TYPE_TIMELINE_SEMAPHORE_SUBMIT_INFO;
    timelineInfo.waitSemaphoreValueCount = 1;
    timelineInfo.pWaitSemaphoreValues = &waitValue;

    VkPipelineStageFlags waitStage = VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT;

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.pNext = &timelineInfo;
    submitInfo.waitSemaphoreCount = 1;
    submitInfo.pWaitSemaphores = &m_images[index].semaphore;
    submitInfo.pWaitDstStageMask = &waitStage;
    submitInfo.signalSemaphoreCount = 1;
    submitInfo.pSignalSemaphores = &m_output.semaphore;
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &m_commandBuffer;
    VK_CHECK(vkQueueSubmit(m_queue, 1, &submitInfo, nullptr));
}

void Renderer::Sync() {
    VkPipelineStageFlags waitStage = VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT;

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.waitSemaphoreCount = 1;
    submitInfo.pWaitSemaphores = &m_output.semaphore;
    submitInfo.pWaitDstStageMask = &waitStage;
    VK_CHECK(vkQueueSubmit(m_queue, 1, &submitInfo, m_fence));

    VK_CHECK(vkWaitForFences(m_dev, 1, &m_fence, VK_TRUE, UINT64_MAX));
    VK_CHECK(vkResetFences(m_dev, 1, &m_fence));
}

Renderer::Output& Renderer::GetOutput() { return m_output; }

Renderer::Timestamps Renderer::GetTimestamps() {
    if (!d.haveCalibratedTimestamps) {
        return { 0, 0, 0 };
    }

    uint64_t queries[2];
    VK_CHECK(vkGetQueryPoolResults(
        m_dev,
        m_queryPool,
        0,
        2,
        2 * sizeof(uint64_t),
        queries,
        sizeof(uint64_t),
        VK_QUERY_RESULT_64_BIT
    ));
    queries[0] *= m_timestampPeriod;
    queries[1] *= m_timestampPeriod;

    VkCalibratedTimestampInfoEXT timestampInfo = {};
    timestampInfo.sType = VK_STRUCTURE_TYPE_CALIBRATED_TIMESTAMP_INFO_EXT;
    timestampInfo.timeDomain = VK_TIME_DOMAIN_DEVICE_EXT;
    uint64_t deviation;
    uint64_t timestamp;
    VK_CHECK(d.vkGetCalibratedTimestampsEXT(m_dev, 1, &timestampInfo, &timestamp, &deviation));
    timestamp *= m_timestampPeriod;

    if (!m_outputImageCapture.empty()) {
        dumpImage(
            m_output.image,
            m_output.view,
            m_output.layout,
            m_output.imageInfo.extent.width,
            m_output.imageInfo.extent.height,
            m_outputImageCapture
        );
        m_outputImageCapture.clear();
    }

    return { timestamp, queries[0], queries[1] };
}

void Renderer::CaptureInputFrame(const std::string& filename) { m_inputImageCapture = filename; }

void Renderer::CaptureOutputFrame(const std::string& filename) { m_outputImageCapture = filename; }

std::string Renderer::result_to_str(VkResult result) {
    switch (result) {
#define VAL(x)                                                                                     \
    case x:                                                                                        \
        return #x
        VAL(VK_SUCCESS);
        VAL(VK_NOT_READY);
        VAL(VK_TIMEOUT);
        VAL(VK_EVENT_SET);
        VAL(VK_EVENT_RESET);
        VAL(VK_INCOMPLETE);
        VAL(VK_ERROR_OUT_OF_HOST_MEMORY);
        VAL(VK_ERROR_OUT_OF_DEVICE_MEMORY);
        VAL(VK_ERROR_INITIALIZATION_FAILED);
        VAL(VK_ERROR_DEVICE_LOST);
        VAL(VK_ERROR_MEMORY_MAP_FAILED);
        VAL(VK_ERROR_LAYER_NOT_PRESENT);
        VAL(VK_ERROR_EXTENSION_NOT_PRESENT);
        VAL(VK_ERROR_FEATURE_NOT_PRESENT);
        VAL(VK_ERROR_INCOMPATIBLE_DRIVER);
        VAL(VK_ERROR_TOO_MANY_OBJECTS);
        VAL(VK_ERROR_FORMAT_NOT_SUPPORTED);
        VAL(VK_ERROR_FRAGMENTED_POOL);
        VAL(VK_ERROR_OUT_OF_POOL_MEMORY);
        VAL(VK_ERROR_INVALID_EXTERNAL_HANDLE);
        VAL(VK_ERROR_SURFACE_LOST_KHR);
        VAL(VK_ERROR_NATIVE_WINDOW_IN_USE_KHR);
        VAL(VK_SUBOPTIMAL_KHR);
        VAL(VK_ERROR_OUT_OF_DATE_KHR);
        VAL(VK_ERROR_INCOMPATIBLE_DISPLAY_KHR);
        VAL(VK_ERROR_VALIDATION_FAILED_EXT);
        VAL(VK_ERROR_INVALID_SHADER_NV);
        VAL(VK_ERROR_INVALID_DRM_FORMAT_MODIFIER_PLANE_LAYOUT_EXT);
        VAL(VK_ERROR_NOT_PERMITTED_EXT);
        VAL(VK_RESULT_MAX_ENUM);
#undef VAL
    default:
        return "Unknown VkResult";
    }
}

void Renderer::commandBufferBegin() {
    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(m_commandBuffer, &commandBufferBegin));
}

void Renderer::commandBufferSubmit() {
    VK_CHECK(vkEndCommandBuffer(m_commandBuffer));

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &m_commandBuffer;
    VkFenceCreateInfo fenceInfo = {};
    fenceInfo.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
    VkFence fence;
    VK_CHECK(vkCreateFence(m_dev, &fenceInfo, nullptr, &fence));
    VK_CHECK(vkQueueSubmit(m_queue, 1, &submitInfo, fence));
    VK_CHECK(vkWaitForFences(m_dev, 1, &fence, VK_TRUE, UINT64_MAX));
    vkDestroyFence(m_dev, fence, nullptr);
}

void Renderer::addStagingImage(uint32_t width, uint32_t height) {
    VkImageCreateInfo imageInfo = {};
    imageInfo = {};
    imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    imageInfo.imageType = VK_IMAGE_TYPE_2D;
    imageInfo.format = m_format;
    imageInfo.extent.width = width;
    imageInfo.extent.height = height;
    imageInfo.extent.depth = 1;
    imageInfo.mipLevels = 1;
    imageInfo.arrayLayers = 1;
    imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
    imageInfo.tiling = VK_IMAGE_TILING_OPTIMAL;
    imageInfo.usage = VK_IMAGE_USAGE_STORAGE_BIT | VK_IMAGE_USAGE_SAMPLED_BIT;
    imageInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    VkImage image;
    VK_CHECK(vkCreateImage(m_dev, &imageInfo, nullptr, &image));

    VkMemoryRequirements memoryReqs;
    vkGetImageMemoryRequirements(m_dev, image, &memoryReqs);
    VkMemoryAllocateInfo memoryAllocInfo = {};
    memoryAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    memoryAllocInfo.allocationSize = memoryReqs.size;
    memoryAllocInfo.memoryTypeIndex
        = memoryTypeIndex(VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memoryReqs.memoryTypeBits);
    VkDeviceMemory memory;
    VK_CHECK(vkAllocateMemory(m_dev, &memoryAllocInfo, nullptr, &memory));
    VK_CHECK(vkBindImageMemory(m_dev, image, memory, 0));

    VkImageViewCreateInfo viewInfo = {};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = imageInfo.format;
    viewInfo.image = image;
    viewInfo.subresourceRange = {};
    viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    viewInfo.subresourceRange.baseMipLevel = 0;
    viewInfo.subresourceRange.levelCount = 1;
    viewInfo.subresourceRange.baseArrayLayer = 0;
    viewInfo.subresourceRange.layerCount = 1;
    viewInfo.components.r = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.g = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.b = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.a = VK_COMPONENT_SWIZZLE_IDENTITY;
    VkImageView view;
    VK_CHECK(vkCreateImageView(m_dev, &viewInfo, nullptr, &view));

    m_stagingImages.push_back({ image, VK_IMAGE_LAYOUT_UNDEFINED, memory, view });
}

void Renderer::dumpImage(
    VkImage image,
    VkImageView imageView,
    VkImageLayout imageLayout,
    uint32_t width,
    uint32_t height,
    const std::string& filename
) {
    VkImageCreateInfo imageInfo = {};
    imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    imageInfo.imageType = VK_IMAGE_TYPE_2D;
    imageInfo.format = VK_FORMAT_R8G8B8A8_UNORM;
    imageInfo.extent.width = width;
    imageInfo.extent.height = height;
    imageInfo.extent.depth = 1;
    imageInfo.arrayLayers = 1;
    imageInfo.mipLevels = 1;
    imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
    imageInfo.tiling = VK_IMAGE_TILING_LINEAR;
    imageInfo.usage = VK_IMAGE_USAGE_STORAGE_BIT;
    VkImage dstImage;
    VK_CHECK(vkCreateImage(m_dev, &imageInfo, nullptr, &dstImage));

    VkMemoryRequirements memReqs;
    VkMemoryAllocateInfo memAllocInfo {};
    memAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    vkGetImageMemoryRequirements(m_dev, dstImage, &memReqs);
    memAllocInfo.allocationSize = memReqs.size;
    memAllocInfo.memoryTypeIndex = memoryTypeIndex(
        VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_CACHED_BIT
            | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
        memReqs.memoryTypeBits
    );
    VkDeviceMemory dstMemory;
    VK_CHECK(vkAllocateMemory(m_dev, &memAllocInfo, nullptr, &dstMemory));
    VK_CHECK(vkBindImageMemory(m_dev, dstImage, dstMemory, 0));

    VkImageViewCreateInfo viewInfo = {};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = imageInfo.format;
    viewInfo.image = dstImage;
    viewInfo.subresourceRange = {};
    viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    viewInfo.subresourceRange.baseMipLevel = 0;
    viewInfo.subresourceRange.levelCount = 1;
    viewInfo.subresourceRange.baseArrayLayer = 0;
    viewInfo.subresourceRange.layerCount = 1;
    viewInfo.components.r = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.g = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.b = VK_COMPONENT_SWIZZLE_IDENTITY;
    viewInfo.components.a = VK_COMPONENT_SWIZZLE_IDENTITY;
    VkImageView dstView;
    VK_CHECK(vkCreateImageView(m_dev, &viewInfo, nullptr, &dstView));

    std::array<VkImageMemoryBarrier, 2> imageBarrierIn;
    imageBarrierIn[0] = {};
    imageBarrierIn[0].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrierIn[0].oldLayout = imageLayout;
    imageBarrierIn[0].newLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
    imageBarrierIn[0].image = image;
    imageBarrierIn[0].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrierIn[0].subresourceRange.layerCount = 1;
    imageBarrierIn[0].subresourceRange.levelCount = 1;
    imageBarrierIn[0].srcAccessMask = 0;
    imageBarrierIn[0].dstAccessMask = VK_ACCESS_SHADER_READ_BIT;
    imageBarrierIn[1] = {};
    imageBarrierIn[1].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrierIn[1].oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    imageBarrierIn[1].newLayout = VK_IMAGE_LAYOUT_GENERAL;
    imageBarrierIn[1].image = dstImage;
    imageBarrierIn[1].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrierIn[1].subresourceRange.layerCount = 1;
    imageBarrierIn[1].subresourceRange.levelCount = 1;
    imageBarrierIn[1].srcAccessMask = 0;
    imageBarrierIn[1].dstAccessMask = VK_ACCESS_SHADER_WRITE_BIT;

    // Shader
    VkShaderModuleCreateInfo moduleInfo = {};
    moduleInfo.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    moduleInfo.codeSize = m_quadShaderSize;
    moduleInfo.pCode = m_quadShaderCode;
    VkShaderModule shader;
    VK_CHECK(vkCreateShaderModule(m_dev, &moduleInfo, nullptr, &shader));

    // Pipeline
    VkPipelineLayoutCreateInfo pipelineLayoutInfo = {};
    pipelineLayoutInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    pipelineLayoutInfo.setLayoutCount = 1;
    pipelineLayoutInfo.pSetLayouts = &m_descriptorLayout;
    VkPipelineLayout pipelineLayout;
    VK_CHECK(vkCreatePipelineLayout(m_dev, &pipelineLayoutInfo, nullptr, &pipelineLayout));

    VkPipelineShaderStageCreateInfo stageInfo = {};
    stageInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stageInfo.stage = VK_SHADER_STAGE_COMPUTE_BIT;
    stageInfo.pName = "main";
    stageInfo.module = shader;

    VkComputePipelineCreateInfo pipelineInfo = {};
    pipelineInfo.sType = VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO;
    pipelineInfo.layout = pipelineLayout;
    pipelineInfo.stage = stageInfo;
    VkPipeline pipeline;
    VK_CHECK(vkCreateComputePipelines(m_dev, nullptr, 1, &pipelineInfo, nullptr, &pipeline));

    std::array<VkImageMemoryBarrier, 2> imageBarrierOut;
    imageBarrierOut[0] = {};
    imageBarrierOut[0].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrierOut[0].oldLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
    imageBarrierOut[0].newLayout = imageLayout;
    imageBarrierOut[0].image = image;
    imageBarrierOut[0].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrierOut[0].subresourceRange.layerCount = 1;
    imageBarrierOut[0].subresourceRange.levelCount = 1;
    imageBarrierOut[0].srcAccessMask = 0;
    imageBarrierOut[0].dstAccessMask = 0;
    imageBarrierOut[1] = {};
    imageBarrierOut[1].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrierOut[1].oldLayout = VK_IMAGE_LAYOUT_GENERAL;
    imageBarrierOut[1].newLayout = VK_IMAGE_LAYOUT_GENERAL;
    imageBarrierOut[1].image = dstImage;
    imageBarrierOut[1].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrierOut[1].subresourceRange.layerCount = 1;
    imageBarrierOut[1].subresourceRange.levelCount = 1;
    imageBarrierOut[1].srcAccessMask = 0;
    imageBarrierOut[1].dstAccessMask = VK_ACCESS_MEMORY_READ_BIT;

    std::vector<VkWriteDescriptorSet> descriptorWriteSets;

    VkDescriptorImageInfo descriptorImageInfoIn = {};
    descriptorImageInfoIn.imageView = imageView;
    descriptorImageInfoIn.imageLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;

    VkDescriptorImageInfo descriptorImageInfoOut = {};
    descriptorImageInfoOut.imageView = dstView;
    descriptorImageInfoOut.imageLayout = VK_IMAGE_LAYOUT_GENERAL;

    VkWriteDescriptorSet descriptorWriteSet = {};
    descriptorWriteSet.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
    descriptorWriteSet.descriptorCount = 1;
    descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorWriteSet.pImageInfo = &descriptorImageInfoIn;
    descriptorWriteSet.dstBinding = 0;
    descriptorWriteSets.push_back(descriptorWriteSet);

    descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
    descriptorWriteSet.pImageInfo = &descriptorImageInfoOut;
    descriptorWriteSet.dstBinding = 1;
    descriptorWriteSets.push_back(descriptorWriteSet);

    commandBufferBegin();
    vkCmdPipelineBarrier(
        m_commandBuffer,
        VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
        VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
        0,
        0,
        nullptr,
        0,
        nullptr,
        imageBarrierIn.size(),
        imageBarrierIn.data()
    );
    vkCmdBindPipeline(m_commandBuffer, VK_PIPELINE_BIND_POINT_COMPUTE, pipeline);
    d.vkCmdPushDescriptorSetKHR(
        m_commandBuffer,
        VK_PIPELINE_BIND_POINT_COMPUTE,
        pipelineLayout,
        0,
        descriptorWriteSets.size(),
        descriptorWriteSets.data()
    );
    vkCmdDispatch(
        m_commandBuffer, (imageInfo.extent.width + 7) / 8, (imageInfo.extent.height + 7) / 8, 1
    );
    vkCmdPipelineBarrier(
        m_commandBuffer,
        VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
        VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
        0,
        0,
        nullptr,
        0,
        nullptr,
        imageBarrierOut.size(),
        imageBarrierOut.data()
    );
    commandBufferSubmit();

    VkImageSubresource subresource = {};
    subresource.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    VkSubresourceLayout layout;
    vkGetImageSubresourceLayout(m_dev, dstImage, &subresource, &layout);

    const char* imageData;
    VK_CHECK(vkMapMemory(m_dev, dstMemory, 0, VK_WHOLE_SIZE, 0, (void**)&imageData));
    imageData += layout.offset;

    std::ofstream file(filename, std::ios::out | std::ios::binary);

    // PPM header
    file << "P6\n" << width << "\n" << height << "\n" << 255 << "\n";

    // PPM binary pixel data
    for (uint32_t y = 0; y < height; y++) {
        uint32_t* row = (uint32_t*)imageData;
        for (uint32_t x = 0; x < width; x++) {
            file.write((char*)row++, 3);
        }
        imageData += layout.rowPitch;
    }
    file.close();

    std::cout << "Image saved to \"" << filename << "\"" << std::endl;

    vkUnmapMemory(m_dev, dstMemory);
    vkFreeMemory(m_dev, dstMemory, nullptr);
    vkDestroyImage(m_dev, dstImage, nullptr);
    vkDestroyImageView(m_dev, dstView, nullptr);
    vkDestroyShaderModule(m_dev, shader, nullptr);
    vkDestroyPipeline(m_dev, pipeline, nullptr);
    vkDestroyPipelineLayout(m_dev, pipelineLayout, nullptr);
}

uint32_t Renderer::memoryTypeIndex(VkMemoryPropertyFlags properties, uint32_t typeBits) const {
    VkPhysicalDeviceMemoryProperties prop;
    vkGetPhysicalDeviceMemoryProperties(m_physDev, &prop);
    for (uint32_t i = 0; i < prop.memoryTypeCount; i++) {
        if ((prop.memoryTypes[i].propertyFlags & properties) == properties && typeBits & (1 << i)) {
            return i;
        }
    }
    return 0xFFFFFFFF;
}

// RenderPipeline
RenderPipeline::RenderPipeline(Renderer* render)
    : r(render) { }

RenderPipeline::~RenderPipeline() {
    vkDestroyShaderModule(r->m_dev, m_shader, nullptr);
    vkDestroyPipeline(r->m_dev, m_pipeline, nullptr);
    vkDestroyPipelineLayout(r->m_dev, m_pipelineLayout, nullptr);
}

void RenderPipeline::SetShader(const char* filename) {
    std::ifstream is(filename, std::ios::binary | std::ios::in | std::ios::ate);
    if (!is.is_open()) {
        std::cerr << "Failed to open shader file: " << filename << std::endl;
        return;
    }
    size_t size = is.tellg();
    is.seekg(0, std::ios::beg);
    std::vector<char> data(size);
    is.read(data.data(), size);
    SetShader((unsigned char*)data.data(), size);
}

void RenderPipeline::SetShader(const unsigned char* data, unsigned len) {
    VkShaderModuleCreateInfo moduleInfo = {};
    moduleInfo.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    moduleInfo.codeSize = len;
    moduleInfo.pCode = (uint32_t*)data;
    VK_CHECK(vkCreateShaderModule(r->m_dev, &moduleInfo, nullptr, &m_shader));
}

void RenderPipeline::Build() {
    VkPipelineLayoutCreateInfo pipelineLayoutInfo = {};
    pipelineLayoutInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    pipelineLayoutInfo.setLayoutCount = 1;
    pipelineLayoutInfo.pSetLayouts = &r->m_descriptorLayout;
    VK_CHECK(vkCreatePipelineLayout(r->m_dev, &pipelineLayoutInfo, nullptr, &m_pipelineLayout));

    VkSpecializationInfo specInfo = {};
    specInfo.mapEntryCount = m_constantEntries.size();
    specInfo.pMapEntries = m_constantEntries.data();
    specInfo.dataSize = m_constantSize;
    specInfo.pData = m_constant;

    VkPipelineShaderStageCreateInfo stageInfo = {};
    stageInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stageInfo.stage = VK_SHADER_STAGE_COMPUTE_BIT;
    stageInfo.pName = "main";
    stageInfo.module = m_shader;
    if (m_constant) {
        stageInfo.pSpecializationInfo = &specInfo;
    }

    VkComputePipelineCreateInfo pipelineInfo = {};
    pipelineInfo.sType = VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO;
    pipelineInfo.layout = m_pipelineLayout;
    pipelineInfo.stage = stageInfo;
    VK_CHECK(vkCreateComputePipelines(r->m_dev, nullptr, 1, &pipelineInfo, nullptr, &m_pipeline));
}

void RenderPipeline::Render(VkImageView in, VkImageView out, VkRect2D outSize) {
    vkCmdBindPipeline(r->m_commandBuffer, VK_PIPELINE_BIND_POINT_COMPUTE, m_pipeline);

    VkDescriptorImageInfo descriptorImageInfoIn = {};
    descriptorImageInfoIn.imageView = in;
    descriptorImageInfoIn.imageLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;

    VkDescriptorImageInfo descriptorImageInfoOut = {};
    descriptorImageInfoOut.imageView = out;
    descriptorImageInfoOut.imageLayout = VK_IMAGE_LAYOUT_GENERAL;

    VkWriteDescriptorSet descriptorWriteSets[2] = {};
    descriptorWriteSets[0].sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
    descriptorWriteSets[0].descriptorCount = 1;
    descriptorWriteSets[0].descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorWriteSets[0].pImageInfo = &descriptorImageInfoIn;
    descriptorWriteSets[0].dstBinding = 0;
    descriptorWriteSets[1].sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
    descriptorWriteSets[1].descriptorCount = 1;
    descriptorWriteSets[1].descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
    descriptorWriteSets[1].pImageInfo = &descriptorImageInfoOut;
    descriptorWriteSets[1].dstBinding = 1;
    r->d.vkCmdPushDescriptorSetKHR(
        r->m_commandBuffer,
        VK_PIPELINE_BIND_POINT_COMPUTE,
        m_pipelineLayout,
        0,
        2,
        descriptorWriteSets
    );

    vkCmdDispatch(
        r->m_commandBuffer, (outSize.extent.width + 7) / 8, (outSize.extent.height + 7) / 8, 1
    );
}
