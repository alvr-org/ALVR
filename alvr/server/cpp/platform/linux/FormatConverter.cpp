#include "FormatConverter.h"
#include "alvr_server/bindings.h"

FormatConverter::FormatConverter(Renderer *render)
    : r(render)
{
}

FormatConverter::~FormatConverter()
{
    for (const OutputImage &image : m_images) {
        vkUnmapMemory(r->m_dev, image.memory);
        vkDestroyImageView(r->m_dev, image.view, nullptr);
        vkDestroyImage(r->m_dev, image.image, nullptr);
        vkFreeMemory(r->m_dev, image.memory, nullptr);
    }

    vkDestroySemaphore(r->m_dev, m_output.semaphore, nullptr);

    vkDestroyQueryPool(r->m_dev, m_queryPool, nullptr);
    vkDestroyDescriptorSetLayout(r->m_dev, m_descriptorLayout, nullptr);
    vkDestroyImageView(r->m_dev, m_view, nullptr);
    vkDestroyShaderModule(r->m_dev, m_shader, nullptr);
    vkDestroyPipeline(r->m_dev, m_pipeline, nullptr);
    vkDestroyPipelineLayout(r->m_dev, m_pipelineLayout, nullptr);
}

void FormatConverter::init(VkImage image, VkImageCreateInfo imageCreateInfo, VkSemaphore semaphore, int count, const unsigned char *shaderData, unsigned shaderLen)
{
    m_images.resize(count);
    m_semaphore = semaphore;

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
    descriptorBindings[0].descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
    descriptorBindings[0].descriptorCount = 1;
    descriptorBindings[1] = {};
    descriptorBindings[1].binding = 1;
    descriptorBindings[1].stageFlags = VK_SHADER_STAGE_COMPUTE_BIT;
    descriptorBindings[1].descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
    descriptorBindings[1].descriptorCount = count;

    VkDescriptorSetLayoutCreateInfo descriptorSetLayoutInfo = {};
    descriptorSetLayoutInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO;
    descriptorSetLayoutInfo.flags = VK_DESCRIPTOR_SET_LAYOUT_CREATE_PUSH_DESCRIPTOR_BIT_KHR;
    descriptorSetLayoutInfo.bindingCount = 2;
    descriptorSetLayoutInfo.pBindings = descriptorBindings;
    VK_CHECK(vkCreateDescriptorSetLayout(r->m_dev, &descriptorSetLayoutInfo, nullptr, &m_descriptorLayout));

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

    // Output images
    for (int i = 0; i < count; ++i) {
        VkImageCreateInfo imageInfo = {};
        imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
        imageInfo.imageType = VK_IMAGE_TYPE_2D;
        imageInfo.format = VK_FORMAT_R8_UNORM;
        imageInfo.extent.width = imageCreateInfo.extent.width;
        imageInfo.extent.height = imageCreateInfo.extent.height;
        imageInfo.extent.depth = 1;
        imageInfo.arrayLayers = 1;
        imageInfo.mipLevels = 1;
        imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
        imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
        imageInfo.tiling = VK_IMAGE_TILING_LINEAR;
        imageInfo.usage = VK_IMAGE_USAGE_STORAGE_BIT;
        VK_CHECK(vkCreateImage(r->m_dev, &imageInfo, nullptr, &m_images[i].image));

        VkMemoryRequirements memReqs;
        VkMemoryAllocateInfo memAllocInfo {};
        memAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
        vkGetImageMemoryRequirements(r->m_dev, m_images[i].image, &memReqs);
        memAllocInfo.allocationSize = memReqs.size;

        VkMemoryPropertyFlags memType = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_CACHED_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
        memAllocInfo.memoryTypeIndex = r->memoryTypeIndex(memType, memReqs.memoryTypeBits);
        VK_CHECK(vkAllocateMemory(r->m_dev, &memAllocInfo, nullptr, &m_images[i].memory));
        VK_CHECK(vkBindImageMemory(r->m_dev, m_images[i].image, m_images[i].memory, 0));

        VkImageViewCreateInfo viewInfo = {};
        viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
        viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
        viewInfo.format = imageInfo.format;
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

        r->commandBufferBegin();
        vkCmdPipelineBarrier(r->m_commandBuffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT, 0, 0, nullptr, 0, nullptr, 1, &imageBarrier);
        r->commandBufferSubmit();

        VkImageSubresource subresource = {};
        subresource.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        VkSubresourceLayout layout;
        vkGetImageSubresourceLayout(r->m_dev, m_images[i].image, &subresource, &layout);

        m_images[i].linesize = layout.rowPitch;
        VK_CHECK(vkMapMemory(r->m_dev, m_images[i].memory, 0, VK_WHOLE_SIZE, 0, reinterpret_cast<void**>(&m_images[i].mapped)));
    }

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

    m_groupCountX = (imageCreateInfo.extent.width + 7) / 8;
    m_groupCountY = (imageCreateInfo.extent.height + 7) / 8;
}

void FormatConverter::Convert(uint8_t **data, int *linesize)
{
    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(m_commandBuffer, &commandBufferBegin));

    vkCmdResetQueryPool(m_commandBuffer, m_queryPool, 0, 1);

    vkCmdBindPipeline(m_commandBuffer, VK_PIPELINE_BIND_POINT_COMPUTE, m_pipeline);

    std::vector<VkWriteDescriptorSet> descriptorWriteSets;

    VkDescriptorImageInfo descriptorImageInfoIn = {};
    descriptorImageInfoIn.imageView = m_view;
    descriptorImageInfoIn.imageLayout = VK_IMAGE_LAYOUT_GENERAL;

    VkWriteDescriptorSet descriptorWriteSet = {};
    descriptorWriteSet.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
    descriptorWriteSet.descriptorCount = 1;
    descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
    descriptorWriteSet.pImageInfo = &descriptorImageInfoIn;
    descriptorWriteSet.dstBinding = 0;
    descriptorWriteSets.push_back(descriptorWriteSet);

    VkDescriptorImageInfo descriptorImageInfoOuts[3] = {};
    for (size_t i = 0; i < m_images.size(); ++i) {
        descriptorImageInfoOuts[i].imageView = m_images[i].view;
        descriptorImageInfoOuts[i].imageLayout = VK_IMAGE_LAYOUT_GENERAL;

        descriptorWriteSet.descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_IMAGE;
        descriptorWriteSet.pImageInfo = &descriptorImageInfoOuts[i];
        descriptorWriteSet.dstBinding = 1;
        descriptorWriteSet.dstArrayElement = i;
        descriptorWriteSets.push_back(descriptorWriteSet);
    }

    r->d.vkCmdPushDescriptorSetKHR(m_commandBuffer, VK_PIPELINE_BIND_POINT_COMPUTE, m_pipelineLayout, 0, descriptorWriteSets.size(), descriptorWriteSets.data());

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

    for (size_t i = 0; i < m_images.size(); ++i) {
        data[i] = m_images[i].mapped;
        linesize[i] = m_images[i].linesize;
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

RgbToYuv420::RgbToYuv420(Renderer *render, VkImage image, VkImageCreateInfo imageInfo, VkSemaphore semaphore)
    : FormatConverter(render)
{
    init(image, imageInfo, semaphore, 3, RGBTOYUV420_SHADER_COMP_SPV_PTR, RGBTOYUV420_SHADER_COMP_SPV_LEN);
}
