#pragma once

#include "Renderer.h"

class FormatConverter
{
public:
    virtual ~FormatConverter();

    void Convert(uint8_t **data, int *linesize);

protected:
    struct OutputImage {
        VkImage image;
        VkDeviceMemory memory;
        VkImageView view;
        VkDeviceSize linesize;
        uint8_t *mapped;
    };

    explicit FormatConverter(Renderer *render);
    void init(VkImage image, VkImageCreateInfo imageCreateInfo, int count, const unsigned char *shaderData, unsigned shaderLen);

    Renderer *r;
    VkSampler m_sampler;
    VkCommandPool m_commandPool;
    VkCommandBuffer m_commandBuffer;
    VkDescriptorSet m_descriptor;
    VkDescriptorSetLayout m_descriptorLayout;
    VkImageView m_view;
    VkShaderModule m_shader;
    VkPipelineLayout m_pipelineLayout;
    VkPipeline m_pipeline;
    uint32_t m_groupCountX;
    uint32_t m_groupCountY;
    std::vector<OutputImage> m_images;
};

class RgbToYuv420 : public FormatConverter
{
public:
    explicit RgbToYuv420(Renderer *render, VkImage image, VkImageCreateInfo imageInfo);
};
