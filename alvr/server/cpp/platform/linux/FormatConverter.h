#pragma once

#include "Renderer.h"

class FormatConverter
{
public:
    struct Output {
        VkSemaphore semaphore = VK_NULL_HANDLE;
    };

    virtual ~FormatConverter();

    Output GetOutput();

    void Convert(uint8_t **data, int *linesize);

    void Sync();

    uint64_t GetTimestamp();

protected:
    struct OutputImage {
        VkImage image = VK_NULL_HANDLE;
        VkDeviceMemory memory = VK_NULL_HANDLE;
        VkImageView view = VK_NULL_HANDLE;
        VkSemaphore semaphore = VK_NULL_HANDLE;
        VkDeviceSize linesize = 0;
        uint8_t *mapped = nullptr;
    };

    explicit FormatConverter(Renderer *render);
    void init(VkImage image, VkImageCreateInfo imageCreateInfo, VkSemaphore semaphore, int count, const unsigned char *shaderData, unsigned shaderLen);

    Renderer *r;
    VkQueryPool m_queryPool = VK_NULL_HANDLE;
    VkCommandBuffer m_commandBuffer = VK_NULL_HANDLE;
    VkDescriptorSetLayout m_descriptorLayout = VK_NULL_HANDLE;
    VkImageView m_view = VK_NULL_HANDLE;
    VkSemaphore m_semaphore = VK_NULL_HANDLE;
    VkShaderModule m_shader = VK_NULL_HANDLE;
    VkPipelineLayout m_pipelineLayout = VK_NULL_HANDLE;
    VkPipeline m_pipeline = VK_NULL_HANDLE;
    uint32_t m_groupCountX = 0;
    uint32_t m_groupCountY = 0;
    std::vector<OutputImage> m_images;
    Output m_output;
};

class RgbToYuv420 : public FormatConverter
{
public:
    explicit RgbToYuv420(Renderer *render, VkImage image, VkImageCreateInfo imageInfo, VkSemaphore semaphore);
};
