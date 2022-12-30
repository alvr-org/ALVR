#include "FormatConverter.h"
#include "alvr_server/bindings.h"

struct FormatInfo {
    int planes;
    VkFormat planeFormats[3];
    int planeDiv[3];
};

static FormatInfo formatInfo(VkFormat format)
{
    switch (format) {
    case VK_FORMAT_G8_B8_R8_3PLANE_420_UNORM:
        return { 3, { VK_FORMAT_R8_UNORM, VK_FORMAT_R8_UNORM, VK_FORMAT_R8_UNORM }, { 1, 2, 2 } };
    case VK_FORMAT_G8_B8R8_2PLANE_420_UNORM:
        return { 2, { VK_FORMAT_R8_UNORM, VK_FORMAT_R8G8_UNORM }, { 1, 2 } };
    default:
        throw std::runtime_error("Unsupported format " + std::to_string(format));
    }
}

FormatConverter::FormatConverter(Renderer *render)
    : r(render)
{
}

FormatConverter::~FormatConverter()
{
    for (const OutputImage &image : m_images) {
        if (image.mapped) {
            vkUnmapMemory(r->m_dev, image.memory);
        }
        vkDestroyImageView(r->m_dev, image.view, nullptr);
        vkDestroyImage(r->m_dev, image.image, nullptr);
        vkFreeMemory(r->m_dev, image.memory, nullptr);
    }

    vkDestroyImage(r->m_dev, m_output.image, nullptr);
    vkDestroySemaphore(r->m_dev, m_output.semaphore, nullptr);

    vkDestroySampler(r->m_dev, m_sampler, nullptr);
    vkDestroyQueryPool(r->m_dev, m_queryPool, nullptr);
    vkDestroyDescriptorSetLayout(r->m_dev, m_descriptorLayout, nullptr);
    vkDestroyImageView(r->m_dev, m_view, nullptr);
    vkDestroyShaderModule(r->m_dev, m_shader, nullptr);
    vkDestroyPipeline(r->m_dev, m_pipeline, nullptr);
    vkDestroyPipelineLayout(r->m_dev, m_pipelineLayout, nullptr);
}

void FormatConverter::init(VkImage image, VkImageCreateInfo imageCreateInfo, VkSemaphore semaphore, VkFormat format, bool hostMapped, const unsigned char *shaderData, unsigned shaderLen)
{
    m_semaphore = semaphore;
    auto info = formatInfo(format);

    // Sampler
    VkSamplerCreateInfo samplerInfo = {};
    samplerInfo.sType = VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO;
    samplerInfo.magFilter = VK_FILTER_NEAREST;
    samplerInfo.minFilter = VK_FILTER_NEAREST;
    samplerInfo.mipmapMode = VK_SAMPLER_MIPMAP_MODE_NEAREST;
    samplerInfo.addressModeU = VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE;
    samplerInfo.addressModeV = VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE;
    samplerInfo.addressModeW = VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE;
    samplerInfo.borderColor = VK_BORDER_COLOR_FLOAT_TRANSPARENT_BLACK;
    VK_CHECK(vkCreateSampler(r->m_dev, &samplerInfo, nullptr, &m_sampler));

    // Timestamp query
    VkQueryPoolCreateInfo queryPoolInfo = {};
    queryPoolInfo.sType = VK_STRUCTURE_TYPE_QUERY_POOL_CREATE_INFO;
    queryPoolInfo.queryType = VK_QUERY_TYPE_TIMESTAMP;
    queryPoolInfo.queryCount = 1;
    VK_CHECK(vkCreateQueryPool(r->m_dev, &queryPoolInfo, nullptr, &m_queryPool));

    // Command buffer
    VkCommandBufferAllocateInfo commandBufferInfo = {};
    commandBufferInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
    commandBufferInfo.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    commandBufferInfo.commandPool = r->m_commandPool;
    commandBufferInfo.commandBufferCount = 1;
    VK_CHECK(vkAllocateCommandBuffers(r->m_dev, &commandBufferInfo, &m_commandBuffer));

    // Descriptors
    VkDescriptorSetLayoutBinding descriptorBindings[2];
    descriptorBindings[0] = {};
    descriptorBindings[0].binding = 0;
    descriptorBindings[0].stageFlags = VK_SHADER_STAGE_COMPUTE_BIT;
    descriptorBindings[0].descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorBindings[0].descriptorCount = 1;
    descriptorBindings[0].pImmutableSamplers = &m_sampler;
    descriptorBindings[1] = {};
    descriptorBindings[1].binding = 1;
    descriptorBindings[1].stageFlags = VK_SHADER_STAGE_COMPUTE_BIT;
    descriptorBindings[1].descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
    descriptorBindings[1].descriptorCount = info.planes;

    VkDescriptorSetLayoutCreateInfo descriptorSetLayoutInfo = {};
    descriptorSetLayoutInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO;
    descriptorSetLayoutInfo.bindingCount = 2;
    descriptorSetLayoutInfo.pBindings = descriptorBindings;
    VK_CHECK(vkCreateDescriptorSetLayout(r->m_dev, &descriptorSetLayoutInfo, nullptr, &m_descriptorLayout));

    VkDescriptorSetAllocateInfo descriptorAllocInfo = {};
    descriptorAllocInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO;
    descriptorAllocInfo.descriptorSetCount = 1;
    descriptorAllocInfo.pSetLayouts = &m_descriptorLayout;
    descriptorAllocInfo.descriptorPool = r->m_descriptorPool;
    VK_CHECK(vkAllocateDescriptorSets(r->m_dev, &descriptorAllocInfo, &m_descriptor));

    // Input image
    VkImageViewCreateInfo viewInfo = {};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = imageCreateInfo.format;
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
    VK_CHECK(vkCreateImageView(r->m_dev, &viewInfo, nullptr, &m_view));

    VkDescriptorImageInfo descriptorImageInfo = {};
    descriptorImageInfo.imageView = m_view;
    descriptorImageInfo.imageLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;

    VkWriteDescriptorSet descriptorWriteSet = {};
    descriptorWriteSet.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
    descriptorWriteSet.descriptorCount = 1;
    descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    descriptorWriteSet.dstSet = m_descriptor;
    descriptorWriteSet.dstBinding = 0;
    descriptorWriteSet.pImageInfo = &descriptorImageInfo;
    vkUpdateDescriptorSets(r->m_dev, 1, &descriptorWriteSet, 0, nullptr);

    // Output images
    m_images.resize(info.planes);

    m_output.imageInfo = {};
    m_output.imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    m_output.imageInfo.imageType = VK_IMAGE_TYPE_2D;
    m_output.imageInfo.format = format;
    m_output.imageInfo.extent.width = imageCreateInfo.extent.width;
    m_output.imageInfo.extent.height = imageCreateInfo.extent.height;
    m_output.imageInfo.extent.depth = 1;
    m_output.imageInfo.arrayLayers = 1;
    m_output.imageInfo.mipLevels = 1;
    m_output.imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    m_output.imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
    m_output.imageInfo.tiling = hostMapped ? VK_IMAGE_TILING_LINEAR : VK_IMAGE_TILING_OPTIMAL;
    m_output.imageInfo.usage = VK_IMAGE_USAGE_STORAGE_BIT | VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT;
    m_output.imageInfo.flags = VK_IMAGE_CREATE_EXTENDED_USAGE_BIT | VK_IMAGE_CREATE_ALIAS_BIT | VK_IMAGE_CREATE_DISJOINT_BIT | VK_IMAGE_CREATE_MUTABLE_FORMAT_BIT;
    m_output.imageInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    VK_CHECK(vkCreateImage(r->m_dev, &m_output.imageInfo, nullptr, &m_output.image));

    r->commandBufferBegin();

    for (int i = 0; i < info.planes; ++i) {
        VkImageCreateInfo imageInfo = m_output.imageInfo;
        imageInfo.format = info.planeFormats[i];
        imageInfo.extent.width /= info.planeDiv[i];
        imageInfo.extent.height /= info.planeDiv[i];
        VK_CHECK(vkCreateImage(r->m_dev, &imageInfo, nullptr, &m_images[i].image));

        VkImagePlaneMemoryRequirementsInfo planeReqs = {};
        planeReqs.sType = VK_STRUCTURE_TYPE_IMAGE_PLANE_MEMORY_REQUIREMENTS_INFO;
        planeReqs.planeAspect = static_cast<VkImageAspectFlagBits>(VK_IMAGE_ASPECT_PLANE_0_BIT << i);

        VkImageMemoryRequirementsInfo2 imageReqs = {};
        imageReqs.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_REQUIREMENTS_INFO_2;
        imageReqs.pNext = &planeReqs;
        imageReqs.image = m_output.image;

        VkMemoryRequirements2 memoryReqs2 = {};
        memoryReqs2.sType = VK_STRUCTURE_TYPE_MEMORY_REQUIREMENTS_2;
        vkGetImageMemoryRequirements2(r->m_dev, &imageReqs, &memoryReqs2);

        VkMemoryPropertyFlags memType = VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;
        if (hostMapped) {
            memType = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_CACHED_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
        }
        VkMemoryAllocateInfo memoryAllocInfo = {};
        memoryAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
        memoryAllocInfo.memoryTypeIndex = r->memoryTypeIndex(memType, memoryReqs2.memoryRequirements.memoryTypeBits);
        memoryAllocInfo.allocationSize = memoryReqs2.memoryRequirements.size;
        VK_CHECK(vkAllocateMemory(r->m_dev, &memoryAllocInfo, nullptr, &m_images[i].memory));

        VkBindImagePlaneMemoryInfo bindPlaneInfo = {};
        bindPlaneInfo.sType = VK_STRUCTURE_TYPE_BIND_IMAGE_PLANE_MEMORY_INFO;
        bindPlaneInfo.planeAspect = VK_IMAGE_ASPECT_COLOR_BIT;

        VkBindImageMemoryInfo bindInfo = {};
        bindInfo.sType = VK_STRUCTURE_TYPE_BIND_IMAGE_MEMORY_INFO;
        bindInfo.pNext = &bindPlaneInfo;
        bindInfo.image = m_images[i].image;
        bindInfo.memory = m_images[i].memory;
        bindInfo.memoryOffset = 0;
        VK_CHECK(vkBindImageMemory2(r->m_dev, 1, &bindInfo));

        VkImageViewUsageCreateInfo viewUsageInfo = {};
        viewUsageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_USAGE_CREATE_INFO;
        viewUsageInfo.usage = VK_IMAGE_USAGE_STORAGE_BIT;

        VkImageViewCreateInfo viewInfo = {};
        viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
        viewInfo.pNext = &viewUsageInfo;
        viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
        viewInfo.format = info.planeFormats[i];
        viewInfo.image = m_images[i].image;
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
        VK_CHECK(vkCreateImageView(r->m_dev, &viewInfo, nullptr, &m_images[i].view));

        VkDescriptorImageInfo descriptorImageInfo = {};
        descriptorImageInfo.imageView = m_images[i].view;
        descriptorImageInfo.imageLayout = VK_IMAGE_LAYOUT_GENERAL;

        VkWriteDescriptorSet descriptorWriteSet = {};
        descriptorWriteSet.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
        descriptorWriteSet.descriptorCount = 1;
        descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
        descriptorWriteSet.dstSet = m_descriptor;
        descriptorWriteSet.dstBinding = 1;
        descriptorWriteSet.dstArrayElement = i;
        descriptorWriteSet.pImageInfo = &descriptorImageInfo;
        vkUpdateDescriptorSets(r->m_dev, 1, &descriptorWriteSet, 0, nullptr);

        VkImageMemoryBarrier imageBarrier = {};
        imageBarrier.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
        imageBarrier.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
        imageBarrier.newLayout = VK_IMAGE_LAYOUT_GENERAL;
        imageBarrier.image = m_images[i].image;
        imageBarrier.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        imageBarrier.subresourceRange.layerCount = 1;
        imageBarrier.subresourceRange.levelCount = 1;
        imageBarrier.srcAccessMask = 0;
        imageBarrier.dstAccessMask = VK_ACCESS_SHADER_WRITE_BIT;
        vkCmdPipelineBarrier(r->m_commandBuffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT, 0, 0, nullptr, 0, nullptr, 1, &imageBarrier);

        if (hostMapped) {
            VkImageSubresource subresource = {};
            subresource.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
            VkSubresourceLayout layout;
            vkGetImageSubresourceLayout(r->m_dev, m_images[i].image, &subresource, &layout);

            m_images[i].linesize = layout.rowPitch;
            VK_CHECK(vkMapMemory(r->m_dev, m_images[i].memory, 0, VK_WHOLE_SIZE, 0, reinterpret_cast<void**>(&m_images[i].mapped)));
        }
    }

    VkBindImageMemoryInfo bindInfos[3];
    VkBindImagePlaneMemoryInfo bindPlaneInfos[3];
    for (int i = 0; i < info.planes; ++i) {
        bindInfos[i] = {};
        bindInfos[i].sType = VK_STRUCTURE_TYPE_BIND_IMAGE_MEMORY_INFO;
        bindInfos[i].pNext = &bindPlaneInfos[i];
        bindInfos[i].image = m_output.image;
        bindInfos[i].memory = m_images[i].memory;
        bindInfos[i].memoryOffset = 0;

        bindPlaneInfos[i] = {};
        bindPlaneInfos[i].sType = VK_STRUCTURE_TYPE_BIND_IMAGE_PLANE_MEMORY_INFO;
        bindPlaneInfos[i].planeAspect = static_cast<VkImageAspectFlagBits>(VK_IMAGE_ASPECT_PLANE_0_BIT << i);
    }
    VK_CHECK(vkBindImageMemory2(r->m_dev, info.planes, bindInfos));

    VkImageMemoryBarrier imageBarrier = {};
    imageBarrier.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrier.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    imageBarrier.newLayout = VK_IMAGE_LAYOUT_GENERAL;
    imageBarrier.image = m_output.image;
    imageBarrier.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrier.subresourceRange.layerCount = 1;
    imageBarrier.subresourceRange.levelCount = 1;
    imageBarrier.srcAccessMask = 0;
    imageBarrier.dstAccessMask = 0;
    vkCmdPipelineBarrier(r->m_commandBuffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, 0, 0, nullptr, 0, nullptr, 1, &imageBarrier);

    r->commandBufferSubmit();

    VkSemaphoreCreateInfo semInfo = {};
    semInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    VK_CHECK(vkCreateSemaphore(r->m_dev, &semInfo, nullptr, &m_output.semaphore));

    // Shader
    VkShaderModuleCreateInfo moduleInfo = {};
    moduleInfo.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    moduleInfo.codeSize = shaderLen;
    moduleInfo.pCode = (uint32_t*)shaderData;
    VK_CHECK(vkCreateShaderModule(r->m_dev, &moduleInfo, nullptr, &m_shader));

    // Pipeline
    VkPipelineLayoutCreateInfo pipelineLayoutInfo = {};
    pipelineLayoutInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    pipelineLayoutInfo.setLayoutCount = 1;
    pipelineLayoutInfo.pSetLayouts = &m_descriptorLayout;
    VK_CHECK(vkCreatePipelineLayout(r->m_dev, &pipelineLayoutInfo, nullptr, &m_pipelineLayout));

    VkPipelineShaderStageCreateInfo stageInfo = {};
    stageInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stageInfo.stage = VK_SHADER_STAGE_COMPUTE_BIT;
    stageInfo.pName = "main";
    stageInfo.module = m_shader;

    VkComputePipelineCreateInfo pipelineInfo = {};
    pipelineInfo.sType = VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO;
    pipelineInfo.layout = m_pipelineLayout;
    pipelineInfo.stage = stageInfo;
    VK_CHECK(vkCreateComputePipelines(r->m_dev, nullptr, 1, &pipelineInfo, nullptr, &m_pipeline));

    m_groupCountX = (imageCreateInfo.extent.width + (imageCreateInfo.extent.width & 31)) / 32;
    m_groupCountY = (imageCreateInfo.extent.height + (imageCreateInfo.extent.height & 31)) / 32;
}

FormatConverter::Output FormatConverter::GetOutput()
{
    return m_output;
}

void FormatConverter::Convert(uint8_t **data, int *linesize)
{
    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(m_commandBuffer, &commandBufferBegin));

    vkCmdResetQueryPool(m_commandBuffer, m_queryPool, 0, 1);

    vkCmdBindPipeline(m_commandBuffer, VK_PIPELINE_BIND_POINT_COMPUTE, m_pipeline);
    vkCmdBindDescriptorSets(m_commandBuffer, VK_PIPELINE_BIND_POINT_COMPUTE, m_pipelineLayout, 0, 1, &m_descriptor, 0, nullptr);
    vkCmdDispatch(m_commandBuffer, m_groupCountX, m_groupCountY, 1);

    vkCmdWriteTimestamp(m_commandBuffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, m_queryPool, 0);

    vkEndCommandBuffer(m_commandBuffer);

    VkPipelineStageFlags waitStage = VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT;

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.waitSemaphoreCount = 1;
    submitInfo.pWaitSemaphores = &m_semaphore;
    submitInfo.pWaitDstStageMask = &waitStage;
    submitInfo.signalSemaphoreCount = 1;
    submitInfo.pSignalSemaphores = &m_output.semaphore;
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &m_commandBuffer;
    VK_CHECK(vkQueueSubmit(r->m_queue, 1, &submitInfo, nullptr));

    if (data && linesize) {
        for (size_t i = 0; i < m_images.size(); ++i) {
            data[i] = m_images[i].mapped;
            linesize[i] = m_images[i].linesize;
        }
    }
}

void FormatConverter::Sync()
{
    VkPipelineStageFlags waitStage = VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT;

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.waitSemaphoreCount = 1;
    submitInfo.pWaitSemaphores = &m_output.semaphore;
    submitInfo.pWaitDstStageMask = &waitStage;
    VK_CHECK(vkQueueSubmit(r->m_queue, 1, &submitInfo, r->m_fence));

    VK_CHECK(vkWaitForFences(r->m_dev, 1, &r->m_fence, VK_TRUE, UINT64_MAX));
    VK_CHECK(vkResetFences(r->m_dev, 1, &r->m_fence));
}

uint64_t FormatConverter::GetTimestamp()
{
    uint64_t query;
    VK_CHECK(vkGetQueryPoolResults(r->m_dev, m_queryPool, 0, 1, sizeof(uint64_t), &query, sizeof(uint64_t), VK_QUERY_RESULT_64_BIT));
    return query * r->m_timestampPeriod;
}

RgbToYuv420::RgbToYuv420(Renderer *render, VkImage image, VkImageCreateInfo imageInfo, VkSemaphore semaphore, bool hostMapped)
    : FormatConverter(render)
{
    init(image, imageInfo, semaphore, VK_FORMAT_G8_B8_R8_3PLANE_420_UNORM, hostMapped, RGBTOYUV420_SHADER_COMP_SPV_PTR, RGBTOYUV420_SHADER_COMP_SPV_LEN);
}

RgbToNv12::RgbToNv12(Renderer *render, VkImage image, VkImageCreateInfo imageInfo, VkSemaphore semaphore, bool hostMapped)
    : FormatConverter(render)
{
    init(image, imageInfo, semaphore, VK_FORMAT_G8_B8R8_2PLANE_420_UNORM, hostMapped, RGBTONV12_SHADER_COMP_SPV_PTR, RGBTONV12_SHADER_COMP_SPV_LEN);
}
