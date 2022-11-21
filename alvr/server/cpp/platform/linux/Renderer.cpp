#include "Renderer.h"

#include <array>
#include <fstream>
#include <iostream>
#include <cassert>
#include <cstring>
#include <algorithm>

struct Vertex {
    float position[2];
};

static const char *result_to_str(VkResult result)
{
    switch (result) {
#define VAL(x) case x: return #x
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
    default: return "Unknown VkResult";
    }
}

#define VK_CHECK(f) \
{ \
    VkResult res = (f); \
    if (res != VK_SUCCESS) { \
        std::cout << "Fatal : VkResult is \"" << result_to_str(res) << "\" in " << __FILE__ << " at line " << __LINE__ << std::endl; \
        assert(res == VK_SUCCESS); \
    } \
}

Renderer::Renderer(const VkInstance &inst, const VkDevice &dev, const VkPhysicalDevice &physDev, uint32_t queueFamilyIndex, const std::vector<const char*> &devExtensions)
    : m_inst(inst)
    , m_dev(dev)
    , m_physDev(physDev)
    , m_queueFamilyIndex(queueFamilyIndex)
{
    auto checkExtension = [devExtensions](const char *name) {
        return std::find(devExtensions.begin(), devExtensions.end(), name) != devExtensions.end();
    };
    d.haveDrmModifiers = checkExtension(VK_EXT_IMAGE_DRM_FORMAT_MODIFIER_EXTENSION_NAME);

#define VK_LOAD_PFN(inst, name) (PFN_##name) vkGetInstanceProcAddr(inst, #name)
    d.vkImportSemaphoreFdKHR = VK_LOAD_PFN(m_inst, vkImportSemaphoreFdKHR);
#undef VK_LOAD_PFN
}

Renderer::~Renderer()
{
    for (const InputImage &image : m_images) {
        vkDestroyImageView(m_dev, image.view, nullptr);
        vkDestroyImage(m_dev, image.image, nullptr);
        vkFreeMemory(m_dev, image.memory, nullptr);
        vkDestroySemaphore(m_dev, image.semaphore, nullptr);
    }

    for (const StagingImage &image : m_stagingImages) {
        vkDestroyImageView(m_dev, image.view, nullptr);
        vkDestroyImage(m_dev, image.image, nullptr);
        vkFreeMemory(m_dev, image.memory, nullptr);
        vkDestroyFramebuffer(m_dev, image.framebuffer, nullptr);
    }

    vkDestroyImageView(m_dev, m_output.view, nullptr);
    vkDestroyImage(m_dev, m_output.image, nullptr);
    vkFreeMemory(m_dev, m_output.memory, nullptr);
    vkDestroyFramebuffer(m_dev, m_output.framebuffer, nullptr);

    vkDestroyCommandPool(m_dev, m_commandPool, nullptr);
    vkDestroySampler(m_dev, m_sampler, nullptr);
    vkDestroyBuffer(m_dev, m_vertexBuffer, nullptr);
    vkFreeMemory(m_dev, m_vertexMemory, nullptr);
    vkDestroyRenderPass(m_dev, m_renderPass, nullptr);
    vkDestroyDescriptorPool(m_dev, m_descriptorPool, nullptr);
    vkDestroyDescriptorSetLayout(m_dev, m_descriptorLayout, nullptr);
    vkDestroyFence(m_dev, m_fence, nullptr);
}

void Renderer::Startup(uint32_t width, uint32_t height, VkFormat format)
{
    m_format = format;
    m_imageSize.width = width;
    m_imageSize.height = height;

    vkGetDeviceQueue(m_dev, m_queueFamilyIndex, 0, &m_queue);

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

    // Vertex buffer
    std::vector<Vertex> vertices = {
        { { -1.0f,  1.0f } },
        { {  1.0f,  1.0f } },
        { { -1.0f, -1.0f } },
        { {  1.0f,  1.0f } },
        { {  1.0f, -1.0f } },
        { { -1.0f, -1.0f } }
    };
    const VkDeviceSize vertexBufferSize = vertices.size() * sizeof(Vertex);
    VkBuffer stagingBuffer;
    VkDeviceMemory stagingMemory;

    VkBufferCreateInfo bufferInfo = {};
    bufferInfo.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
    bufferInfo.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
    bufferInfo.size = vertexBufferSize;
    bufferInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    VK_CHECK(vkCreateBuffer(m_dev, &bufferInfo, nullptr, &stagingBuffer));

    VkMemoryRequirements memReqs;
    VkMemoryAllocateInfo memAllocInfo {};
    memAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    vkGetBufferMemoryRequirements(m_dev, stagingBuffer, &memReqs);
    memAllocInfo.allocationSize = memReqs.size;
    memAllocInfo.memoryTypeIndex = memoryTypeIndex(VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT, memReqs.memoryTypeBits);
    VK_CHECK(vkAllocateMemory(m_dev, &memAllocInfo, nullptr, &stagingMemory));

    void *mapped;
    VK_CHECK(vkMapMemory(m_dev, stagingMemory, 0, vertexBufferSize, 0, &mapped));
    memcpy(mapped, vertices.data(), vertexBufferSize);
    vkUnmapMemory(m_dev, stagingMemory);
    VK_CHECK(vkBindBufferMemory(m_dev, stagingBuffer, stagingMemory, 0));

    bufferInfo.usage = VK_BUFFER_USAGE_VERTEX_BUFFER_BIT | VK_BUFFER_USAGE_TRANSFER_DST_BIT;
    VK_CHECK(vkCreateBuffer(m_dev, &bufferInfo, nullptr, &m_vertexBuffer));

    vkGetBufferMemoryRequirements(m_dev, m_vertexBuffer, &memReqs);
    memAllocInfo.allocationSize = memReqs.size;
    memAllocInfo.memoryTypeIndex = memoryTypeIndex(VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memReqs.memoryTypeBits);
    VK_CHECK(vkAllocateMemory(m_dev, &memAllocInfo, nullptr, &m_vertexMemory));
    VK_CHECK(vkBindBufferMemory(m_dev, m_vertexBuffer, m_vertexMemory, 0));

    VkBufferCopy copyRegion = {};
    copyRegion.size = vertexBufferSize;

    commandBufferBegin();
    vkCmdCopyBuffer(m_commandBuffer, stagingBuffer, m_vertexBuffer, 1, &copyRegion);
    commandBufferSubmit();

    vkDestroyBuffer(m_dev, stagingBuffer, nullptr);
    vkFreeMemory(m_dev, stagingMemory, nullptr);

    // Sampler
    VkSamplerCreateInfo samplerInfo = {};
    samplerInfo.sType = VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO;
    samplerInfo.magFilter = VK_FILTER_LINEAR;
    samplerInfo.minFilter = VK_FILTER_LINEAR;
    samplerInfo.mipmapMode = VK_SAMPLER_MIPMAP_MODE_NEAREST; // VK_SAMPLER_MIPMAP_MODE_LINEAR ?
    samplerInfo.addressModeU = VK_SAMPLER_ADDRESS_MODE_REPEAT;
    samplerInfo.addressModeV = VK_SAMPLER_ADDRESS_MODE_REPEAT;
    samplerInfo.addressModeW = VK_SAMPLER_ADDRESS_MODE_REPEAT;
    samplerInfo.maxAnisotropy = 1.0f;
    samplerInfo.minLod = 0.0f;
    samplerInfo.maxLod = 0.25f;
    samplerInfo.borderColor = VK_BORDER_COLOR_FLOAT_TRANSPARENT_BLACK;
    VK_CHECK(vkCreateSampler(m_dev, &samplerInfo, nullptr, &m_sampler));

    // Renderpass
    VkAttachmentDescription attachDesc = {};
    attachDesc.format = m_format;
    attachDesc.samples = VK_SAMPLE_COUNT_1_BIT;
    attachDesc.loadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
    attachDesc.storeOp = VK_ATTACHMENT_STORE_OP_STORE;
    attachDesc.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
    attachDesc.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
    attachDesc.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    attachDesc.finalLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;

    VkAttachmentReference ref = { 0, VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL };

    VkSubpassDescription subDesc = {};
    subDesc.pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS;
    subDesc.colorAttachmentCount = 1;
    subDesc.pColorAttachments = &ref;

    std::array<VkSubpassDependency, 2> dependencies = {};
    dependencies[0].srcSubpass = VK_SUBPASS_EXTERNAL;
    dependencies[0].srcStageMask = VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT;
    dependencies[0].srcAccessMask = VK_ACCESS_MEMORY_READ_BIT;
    dependencies[0].dstSubpass = 0;
    dependencies[0].dstStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
    dependencies[0].dstAccessMask = VK_ACCESS_COLOR_ATTACHMENT_READ_BIT | VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
    dependencies[0].dependencyFlags = VK_DEPENDENCY_BY_REGION_BIT;
    dependencies[1].srcSubpass = 0;
    dependencies[1].srcStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
    dependencies[1].srcAccessMask = VK_ACCESS_COLOR_ATTACHMENT_READ_BIT | VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
    dependencies[1].dstSubpass = VK_SUBPASS_EXTERNAL;
    dependencies[1].dstStageMask = VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT;
    dependencies[1].dstAccessMask = VK_ACCESS_MEMORY_READ_BIT;
    dependencies[1].dependencyFlags = VK_DEPENDENCY_BY_REGION_BIT;

    VkRenderPassCreateInfo renderPassInfo = {};
    renderPassInfo.sType = VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;
    renderPassInfo.attachmentCount = 1;
    renderPassInfo.pAttachments = &attachDesc;
    renderPassInfo.subpassCount = 1;
    renderPassInfo.pSubpasses = &subDesc;
    renderPassInfo.dependencyCount = dependencies.size();
    renderPassInfo.pDependencies = dependencies.data();
    VK_CHECK(vkCreateRenderPass(m_dev, &renderPassInfo, nullptr, &m_renderPass));

    // Descriptors
    VkDescriptorSetLayoutBinding descriptorBinding = {};
    descriptorBinding.descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorBinding.stageFlags = VK_SHADER_STAGE_FRAGMENT_BIT;
    descriptorBinding.descriptorCount = 1;
    descriptorBinding.pImmutableSamplers = &m_sampler;

    VkDescriptorSetLayoutCreateInfo descriptorSetLayoutInfo = {};
    descriptorSetLayoutInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO;
    descriptorSetLayoutInfo.bindingCount = 1;
    descriptorSetLayoutInfo.pBindings = &descriptorBinding;
    VK_CHECK(vkCreateDescriptorSetLayout(m_dev, &descriptorSetLayoutInfo, nullptr, &m_descriptorLayout));

    VkDescriptorPoolSize descriptorPoolSize = {};
    descriptorPoolSize.descriptorCount = 128;
    descriptorPoolSize.type = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;

    VkDescriptorPoolCreateInfo descriptorPoolInfo = {};
    descriptorPoolInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO;
    descriptorPoolInfo.maxSets = descriptorPoolSize.descriptorCount;
    descriptorPoolInfo.poolSizeCount = 1;
    descriptorPoolInfo.pPoolSizes = &descriptorPoolSize;
    descriptorPoolInfo.flags = VK_DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT;
    VK_CHECK(vkCreateDescriptorPool(m_dev, &descriptorPoolInfo, nullptr, &m_descriptorPool));

    // Fence
    VkFenceCreateInfo fenceInfo = {};
    fenceInfo.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
    VK_CHECK(vkCreateFence(m_dev, &fenceInfo, nullptr, &m_fence));
}

void Renderer::AddImage(VkImageCreateInfo imageInfo, size_t memoryIndex, int imageFd, int semaphoreFd)
{
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

    VkDescriptorSetAllocateInfo descriptorAllocInfo = {};
    descriptorAllocInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO;
    descriptorAllocInfo.descriptorSetCount = 1;
    descriptorAllocInfo.pSetLayouts = &m_descriptorLayout;
    descriptorAllocInfo.descriptorPool = m_descriptorPool;
    VkDescriptorSet descriptor;
    VK_CHECK(vkAllocateDescriptorSets(m_dev, &descriptorAllocInfo, &descriptor));

    VkDescriptorImageInfo descriptorImageInfo = {};
    descriptorImageInfo.imageView = view;
    descriptorImageInfo.imageLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;

    VkWriteDescriptorSet descriptorWriteSet = {};
    descriptorWriteSet.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
    descriptorWriteSet.descriptorCount = 1;
    descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorWriteSet.dstSet = descriptor;
    descriptorWriteSet.pImageInfo = &descriptorImageInfo;
    vkUpdateDescriptorSets(m_dev, 1, &descriptorWriteSet, 0, nullptr);

    VkImageMemoryBarrier imageBarrier = {};
    imageBarrier.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrier.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    imageBarrier.newLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
    imageBarrier.image = image;
    imageBarrier.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrier.subresourceRange.layerCount = 1;
    imageBarrier.subresourceRange.levelCount = 1;
    imageBarrier.srcAccessMask = VK_ACCESS_NONE;
    imageBarrier.dstAccessMask = VK_ACCESS_SHADER_READ_BIT;

    commandBufferBegin();
    vkCmdPipelineBarrier(m_commandBuffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT, 0, 0, nullptr, 0, nullptr, 1, &imageBarrier);
    commandBufferSubmit();

    m_images.push_back({image, mem, semaphore, view, descriptor});
}

void Renderer::AddPipeline(RenderPipeline *pipeline)
{
    pipeline->Build();
    m_pipelines.push_back(pipeline);

    if (m_pipelines.size() > 1 && m_stagingImages.size() < 2) {
        addStagingImage(m_imageSize.width, m_imageSize.height);
    }
}

Renderer::Output Renderer::CreateOutput(uint32_t width, uint32_t height)
{
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
    m_output.imageInfo.usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT;
    m_output.imageInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    m_output.imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;

    if (d.haveDrmModifiers) {
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

        std::vector<VkDrmFormatModifierPropertiesEXT> modifierProps(modifierPropsList.drmFormatModifierCount);
        modifierPropsList.pDrmFormatModifierProperties = modifierProps.data();
        vkGetPhysicalDeviceFormatProperties2(m_physDev, m_output.imageInfo.format, &formatProps);

        std::vector<uint64_t> imageModifiers;
        std::cout << "Available modifiers:" << std::endl;
        for (const VkDrmFormatModifierPropertiesEXT &prop : modifierProps) {
            std::cout << "modifier: " << prop.drmFormatModifier << " planes: " << prop.drmFormatModifierPlaneCount << std::endl;

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

            VkResult r = vkGetPhysicalDeviceImageFormatProperties2(m_physDev, &formatInfo, &imageFormatProps);
            if (r == VK_SUCCESS) {
                imageModifiers.push_back(prop.drmFormatModifier);
            }
        }
        modifierListInfo.drmFormatModifierCount = imageModifiers.size();
        modifierListInfo.pDrmFormatModifiers = imageModifiers.data();

        VkExternalMemoryImageCreateInfo extMemImageInfo = {};
        extMemImageInfo.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO;
        extMemImageInfo.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
        modifierListInfo.pNext = &extMemImageInfo;

        VK_CHECK(vkCreateImage(m_dev, &m_output.imageInfo, nullptr, &m_output.image));

        VkMemoryDedicatedRequirements mdr = {};
        mdr.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_REQUIREMENTS;

        VkMemoryRequirements2 memoryReqs = {};
        memoryReqs.sType = VK_STRUCTURE_TYPE_MEMORY_REQUIREMENTS_2;
        memoryReqs.pNext = &mdr;

        VkImageMemoryRequirementsInfo2 memoryReqsInfo = {};
        memoryReqsInfo.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_REQUIREMENTS_INFO_2;
        memoryReqsInfo.image = m_output.image;
        vkGetImageMemoryRequirements2(m_dev, &memoryReqsInfo, &memoryReqs);

        VkExportMemoryAllocateInfo memory_export_info = {};
        memory_export_info.sType = VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO;
        memory_export_info.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;

        VkMemoryDedicatedAllocateInfo memory_dedicated_info = {};
        memory_dedicated_info.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO;
        memory_dedicated_info.pNext = &memory_export_info;
        memory_dedicated_info.image = m_output.image;

        VkMemoryAllocateInfo memi = {};
        memi.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
        memi.pNext = &memory_dedicated_info;
        memi.allocationSize = memoryReqs.memoryRequirements.size;
        memi.memoryTypeIndex = memoryTypeIndex(VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memoryReqs.memoryRequirements.memoryTypeBits);
        VK_CHECK(vkAllocateMemory(m_dev, &memi, nullptr, &m_output.memory));

        VkBindImageMemoryInfo bimi = {};
        bimi.sType = VK_STRUCTURE_TYPE_BIND_IMAGE_MEMORY_INFO;
        bimi.image = m_output.image;
        bimi.memory = m_output.memory;
        bimi.memoryOffset = 0;
        VK_CHECK(vkBindImageMemory2(m_dev, 1, &bimi));
    } else {
        m_output.imageInfo.tiling = VK_IMAGE_TILING_LINEAR; // Doesn't seem to work with optimal tiling
        VK_CHECK(vkCreateImage(m_dev, &m_output.imageInfo, nullptr, &m_output.image));

        VkMemoryRequirements memoryReqs;
        vkGetImageMemoryRequirements(m_dev, m_output.image, &memoryReqs);
        m_output.size = memoryReqs.size;
        VkMemoryAllocateInfo memoryAllocInfo = {};
        memoryAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
        memoryAllocInfo.allocationSize = m_output.size;
        memoryAllocInfo.memoryTypeIndex = memoryTypeIndex(VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memoryReqs.memoryTypeBits);
        VK_CHECK(vkAllocateMemory(m_dev, &memoryAllocInfo, nullptr, &m_output.memory));
        VK_CHECK(vkBindImageMemory(m_dev, m_output.image, m_output.memory, 0));
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

    VkFramebufferCreateInfo framebufferInfo = {};
    framebufferInfo.sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
    framebufferInfo.renderPass = m_renderPass;
    framebufferInfo.attachmentCount = 1;
    framebufferInfo.pAttachments = &m_output.view;
    framebufferInfo.layers = 1;
    framebufferInfo.width = m_output.imageInfo.extent.width;
    framebufferInfo.height = m_output.imageInfo.extent.height;
    VK_CHECK(vkCreateFramebuffer(m_dev, &framebufferInfo, nullptr, &m_output.framebuffer));

    return m_output;
}

void Renderer::Render(uint32_t index, uint64_t waitValue)
{
    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(m_commandBuffer, &commandBufferBegin));

    for (size_t i = 0; i < m_pipelines.size(); ++i) {
        VkRect2D rect = {};
        VkDescriptorSet in = VK_NULL_HANDLE;
        VkFramebuffer out = VK_NULL_HANDLE;
        if (i == 0) {
            in = m_images[index].descriptor;
        } else {
            in = m_stagingImages[(i - 1) % m_stagingImages.size()].descriptor;
        }
        if (i == m_pipelines.size() - 1) {
            out = m_output.framebuffer;
            rect.extent.width = m_output.imageInfo.extent.width;
            rect.extent.height = m_output.imageInfo.extent.height;
        } else {
            out = m_stagingImages[i % m_stagingImages.size()].framebuffer;
            rect.extent = m_imageSize;
        }
        m_pipelines[i]->Render(in, out, rect);
    }

    VK_CHECK(vkEndCommandBuffer(m_commandBuffer));

    VkTimelineSemaphoreSubmitInfo timelineInfo = {};
    timelineInfo.sType = VK_STRUCTURE_TYPE_TIMELINE_SEMAPHORE_SUBMIT_INFO;
    timelineInfo.waitSemaphoreValueCount = 1;
    timelineInfo.pWaitSemaphoreValues = &waitValue;

    VkPipelineStageFlags waitStage = VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT;

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.pNext = &timelineInfo;
    submitInfo.waitSemaphoreCount = 1;
    submitInfo.pWaitSemaphores = &m_images[index].semaphore;
    submitInfo.pWaitDstStageMask = &waitStage;
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &m_commandBuffer;
    VK_CHECK(vkQueueSubmit(m_queue, 1, &submitInfo, m_fence));

    VK_CHECK(vkWaitForFences(m_dev, 1, &m_fence, VK_TRUE, UINT64_MAX));
    VK_CHECK(vkResetFences(m_dev, 1, &m_fence));
}

void Renderer::commandBufferBegin()
{
    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(m_commandBuffer, &commandBufferBegin));
}

void Renderer::commandBufferSubmit()
{
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

void Renderer::addStagingImage(uint32_t width, uint32_t height)
{
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
    imageInfo.usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT;
    imageInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    VkImage image;
    VK_CHECK(vkCreateImage(m_dev, &imageInfo, nullptr, &image));

    VkMemoryRequirements memoryReqs;
    vkGetImageMemoryRequirements(m_dev, image, &memoryReqs);
    VkMemoryAllocateInfo memoryAllocInfo = {};
    memoryAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    memoryAllocInfo.allocationSize = memoryReqs.size;
    memoryAllocInfo.memoryTypeIndex = memoryTypeIndex(VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memoryReqs.memoryTypeBits);
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

    VkFramebufferCreateInfo framebufferInfo = {};
    framebufferInfo.sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
    framebufferInfo.renderPass = m_renderPass;
    framebufferInfo.attachmentCount = 1;
    framebufferInfo.pAttachments = &view;
    framebufferInfo.layers = 1;
    framebufferInfo.width = imageInfo.extent.width;
    framebufferInfo.height = imageInfo.extent.height;
    VkFramebuffer framebuffer;
    VK_CHECK(vkCreateFramebuffer(m_dev, &framebufferInfo, nullptr, &framebuffer));

    VkDescriptorSetAllocateInfo descriptorAllocInfo = {};
    descriptorAllocInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO;
    descriptorAllocInfo.descriptorSetCount = 1;
    descriptorAllocInfo.pSetLayouts = &m_descriptorLayout;
    descriptorAllocInfo.descriptorPool = m_descriptorPool;
    VkDescriptorSet descriptor;
    VK_CHECK(vkAllocateDescriptorSets(m_dev, &descriptorAllocInfo, &descriptor));

    VkDescriptorImageInfo descriptorImageInfo = {};
    descriptorImageInfo.imageView = view;
    descriptorImageInfo.imageLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;

    VkWriteDescriptorSet descriptorWriteSet = {};
    descriptorWriteSet.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
    descriptorWriteSet.descriptorCount = 1;
    descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorWriteSet.dstSet = descriptor;
    descriptorWriteSet.pImageInfo = &descriptorImageInfo;
    vkUpdateDescriptorSets(m_dev, 1, &descriptorWriteSet, 0, nullptr);

    m_stagingImages.push_back({image, memory, view, framebuffer, descriptor});
}

uint32_t Renderer::memoryTypeIndex(VkMemoryPropertyFlags properties, uint32_t typeBits) const
{
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
RenderPipeline::RenderPipeline(Renderer *render)
    : r(render)
{
}

RenderPipeline::~RenderPipeline()
{
    vkDestroyShaderModule(r->m_dev, m_vertexShader, nullptr);
    vkDestroyShaderModule(r->m_dev, m_fragmentShader, nullptr);
    vkDestroyPipeline(r->m_dev, m_pipeline, nullptr);
    vkDestroyPipelineLayout(r->m_dev, m_pipelineLayout, nullptr);
}

void RenderPipeline::SetShader(ShaderType type, const char *filename)
{
    std::ifstream is(filename, std::ios::binary | std::ios::in | std::ios::ate);
    if (!is.is_open()) {
        std::cerr << "Failed to open shader file: " << filename << std::endl;
        return;
    }
    size_t size = is.tellg();
    is.seekg(0, std::ios::beg);
    std::vector<char> data(size);
    is.read(data.data(), size);
    SetShader(type, (unsigned char*)data.data(), size);
}

void RenderPipeline::SetShader(ShaderType type, const unsigned char *data, unsigned len)
{
    VkShaderModuleCreateInfo moduleInfo = {};
    moduleInfo.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    moduleInfo.codeSize = len;
    moduleInfo.pCode = (uint32_t*)data;
    VK_CHECK(vkCreateShaderModule(r->m_dev, &moduleInfo, nullptr, type == VertexShader ? &m_vertexShader : &m_fragmentShader));
}

void RenderPipeline::SetPushConstant(ShaderType type, const void *data, uint32_t size)
{
    if (type == VertexShader) {
        m_vertexConstant = data;
        m_vertexConstantSize = size;
    } else {
        m_fragmentConstant = data;
        m_fragmentConstantSize = size;
    }
}

void RenderPipeline::Build()
{
    std::vector<VkPushConstantRange> pushConstantRanges;

    if (m_vertexConstant) {
        VkPushConstantRange rangeInfo = {};
        rangeInfo.stageFlags = VK_SHADER_STAGE_VERTEX_BIT;
        rangeInfo.offset = 0;
        rangeInfo.size = m_vertexConstantSize;
        pushConstantRanges.push_back(rangeInfo);
    }

    if (m_fragmentConstant) {
        VkPushConstantRange rangeInfo = {};
        rangeInfo.stageFlags = VK_SHADER_STAGE_FRAGMENT_BIT;
        rangeInfo.offset = m_vertexConstantSize;
        rangeInfo.size = m_fragmentConstantSize;
        pushConstantRanges.push_back(rangeInfo);
    }

    VkPipelineLayoutCreateInfo pipelineLayoutInfo = {};
    pipelineLayoutInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    pipelineLayoutInfo.setLayoutCount = 1;
    pipelineLayoutInfo.pSetLayouts = &r->m_descriptorLayout;
    pipelineLayoutInfo.pushConstantRangeCount = pushConstantRanges.size();
    pipelineLayoutInfo.pPushConstantRanges = pushConstantRanges.data();
    VK_CHECK(vkCreatePipelineLayout(r->m_dev, &pipelineLayoutInfo, nullptr, &m_pipelineLayout));

    std::vector<VkPipelineShaderStageCreateInfo> shaderStages;

    VkPipelineShaderStageCreateInfo stageInfo = {};
    stageInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stageInfo.stage = VK_SHADER_STAGE_VERTEX_BIT;
    stageInfo.pName = "main";
    stageInfo.module = m_vertexShader;
    shaderStages.push_back(stageInfo);

    stageInfo.stage = VK_SHADER_STAGE_FRAGMENT_BIT;
    stageInfo.module = m_fragmentShader;
    shaderStages.push_back(stageInfo);

    VkPipelineInputAssemblyStateCreateInfo inputAssemblyState = {};
    inputAssemblyState.sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
    inputAssemblyState.topology = VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;

    VkPipelineRasterizationStateCreateInfo rasterizationState = {};
    rasterizationState.sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
    rasterizationState.polygonMode = VK_POLYGON_MODE_FILL;
    rasterizationState.cullMode = VK_CULL_MODE_NONE;
    rasterizationState.frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE;
    rasterizationState.lineWidth = 1.0f;

    VkPipelineColorBlendAttachmentState blendAttachmentState = {};
    blendAttachmentState.blendEnable = false;
    blendAttachmentState.colorWriteMask = VK_COLOR_COMPONENT_R_BIT | VK_COLOR_COMPONENT_G_BIT | VK_COLOR_COMPONENT_B_BIT | VK_COLOR_COMPONENT_A_BIT;

    VkPipelineColorBlendStateCreateInfo colorBlendState = {};
    colorBlendState.sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
    colorBlendState.attachmentCount = 1;
    colorBlendState.pAttachments = &blendAttachmentState;

    VkPipelineViewportStateCreateInfo viewportState = {};
    viewportState.sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
    viewportState.viewportCount = 1;
    viewportState.scissorCount = 1;

    VkPipelineMultisampleStateCreateInfo multisampleState = {};
    multisampleState.sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
    multisampleState.rasterizationSamples = VK_SAMPLE_COUNT_1_BIT;

    std::vector<VkDynamicState> dynamicStateEnables = {
        VK_DYNAMIC_STATE_VIEWPORT,
        VK_DYNAMIC_STATE_SCISSOR
    };

    VkPipelineDynamicStateCreateInfo dynamicState = {};
    dynamicState.sType = VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO;
    dynamicState.pDynamicStates = dynamicStateEnables.data();
    dynamicState.dynamicStateCount = dynamicStateEnables.size();

    std::vector<VkVertexInputBindingDescription> vertexInputBindings;
    VkVertexInputBindingDescription inputBind1 = {};
    inputBind1.stride = sizeof(Vertex);
    inputBind1.inputRate = VK_VERTEX_INPUT_RATE_VERTEX;
    vertexInputBindings.push_back(inputBind1);

    std::vector<VkVertexInputAttributeDescription> vertexInputAttributes;
    VkVertexInputAttributeDescription inputAttrib1 = {};
    inputAttrib1.format = VK_FORMAT_R32G32_SFLOAT;
    inputAttrib1.offset = 0;
    vertexInputAttributes.push_back(inputAttrib1);

    VkPipelineVertexInputStateCreateInfo vertexInputState = {};
    vertexInputState.sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
    vertexInputState.vertexBindingDescriptionCount = static_cast<uint32_t>(vertexInputBindings.size());
    vertexInputState.pVertexBindingDescriptions = vertexInputBindings.data();
    vertexInputState.vertexAttributeDescriptionCount = static_cast<uint32_t>(vertexInputAttributes.size());
    vertexInputState.pVertexAttributeDescriptions = vertexInputAttributes.data();

    VkGraphicsPipelineCreateInfo pipelineInfo = {};
    pipelineInfo.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
    pipelineInfo.layout = m_pipelineLayout;
    pipelineInfo.renderPass = r->m_renderPass;
    pipelineInfo.subpass = 0;
    pipelineInfo.stageCount = shaderStages.size();
    pipelineInfo.pStages = shaderStages.data();
    pipelineInfo.pInputAssemblyState = &inputAssemblyState;
    pipelineInfo.pRasterizationState = &rasterizationState;
    pipelineInfo.pColorBlendState = &colorBlendState;
    pipelineInfo.pMultisampleState = &multisampleState;
    pipelineInfo.pViewportState = &viewportState;
    pipelineInfo.pDynamicState = &dynamicState;
    pipelineInfo.pVertexInputState = &vertexInputState;
    VK_CHECK(vkCreateGraphicsPipelines(r->m_dev, nullptr, 1, &pipelineInfo, nullptr, &m_pipeline));
}

void RenderPipeline::Render(VkDescriptorSet in, VkFramebuffer out, VkRect2D outSize)
{
    VkRenderPassBeginInfo renderPassBegin = {};
    renderPassBegin.sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO;
    renderPassBegin.renderArea = outSize;
    renderPassBegin.renderPass = r->m_renderPass;
    renderPassBegin.framebuffer = out;
    vkCmdBeginRenderPass(r->m_commandBuffer, &renderPassBegin, VK_SUBPASS_CONTENTS_INLINE);

    VkViewport viewport = {};
    viewport.width = outSize.extent.width;
    viewport.height = outSize.extent.height;
    viewport.minDepth = 0.0f;
    viewport.maxDepth = 1.0f;
    vkCmdSetViewport(r->m_commandBuffer, 0, 1, &viewport);
    vkCmdSetScissor(r->m_commandBuffer, 0, 1, &outSize);

    vkCmdBindPipeline(r->m_commandBuffer, VK_PIPELINE_BIND_POINT_GRAPHICS, m_pipeline);

    VkDeviceSize offset = 0;
    vkCmdBindVertexBuffers(r->m_commandBuffer, 0, 1, &r->m_vertexBuffer, &offset);

    vkCmdBindDescriptorSets(r->m_commandBuffer, VK_PIPELINE_BIND_POINT_GRAPHICS, m_pipelineLayout, 0, 1, &in, 0, nullptr);

    if (m_vertexConstant) {
        vkCmdPushConstants(r->m_commandBuffer, m_pipelineLayout, VK_SHADER_STAGE_VERTEX_BIT, 0, m_vertexConstantSize, m_vertexConstant);
    }
    if (m_fragmentConstant) {
        vkCmdPushConstants(r->m_commandBuffer, m_pipelineLayout, VK_SHADER_STAGE_FRAGMENT_BIT, m_vertexConstantSize, m_fragmentConstantSize, m_fragmentConstant);
    }

    vkCmdDraw(r->m_commandBuffer, 6, 1, 0, 0);

    vkCmdEndRenderPass(r->m_commandBuffer);
}
