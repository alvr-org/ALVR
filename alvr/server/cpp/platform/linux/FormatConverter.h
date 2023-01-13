#pragma once

#include "Renderer.h"

class FormatConverter
{
public:
    struct Output {
        VkSemaphore semaphore;
    };

    virtual ~FormatConverter();

    Output GetOutput();

    void Convert(uint8_t **data, int *linesize);

    void Sync();

    uint64_t GetTimestamp();

protected:
    struct OutputImage {
        VkImage image;
        VkDeviceMemory memory;
        VkImageView view;
        VkSemaphore semaphore;
        VkDeviceSize linesize;
        uint8_t *mapped;
    };

    explicit FormatConverter(Renderer *render);
    void init(VkImage image, VkImageCreateInfo imageCreateInfo, VkSemaphore semaphore, int count, const unsigned char *shaderData, unsigned shaderLen);

    Renderer *r;
    VkSampler m_sampler;
    VkQueryPool m_queryPool;
    VkCommandBuffer m_commandBuffer;
    VkDescriptorSet m_descriptor;
    VkDescriptorSetLayout m_descriptorLayout;
    VkImageView m_view;
    VkSemaphore m_semaphore;
    VkShaderModule m_shader;
    VkPipelineLayout m_pipelineLayout;
    VkPipeline m_pipeline;
    uint32_t m_groupCountX;
    uint32_t m_groupCountY;
    std::vector<OutputImage> m_images;
    Output m_output;
};

class RgbToYuv420 : public FormatConverter
{
public:
    explicit RgbToYuv420(Renderer *render, VkImage image, VkImageCreateInfo imageInfo, VkSemaphore semaphore);
};
