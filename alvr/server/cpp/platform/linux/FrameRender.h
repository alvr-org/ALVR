#pragma once

#include <vector>
#include <string>
#include <vulkan/vulkan.h>

class FrameRender
{
public:
    struct Output {
        VkImage image;
        VkImageCreateInfo imageInfo;
        VkDeviceSize size;
        VkDeviceMemory memory;

        // ---
        VkImageView view;
        VkFramebuffer framebuffer;
    };

    FrameRender(const VkInstance &inst, const VkDevice &dev, const VkPhysicalDevice &physDev, const std::vector<std::string> &devExtensions);
    virtual ~FrameRender();

    void Startup(uint32_t width, uint32_t height, VkFormat format, std::vector<uint32_t> queueFamilies);

    void AddImage(VkImageCreateInfo imageInfo, size_t memoryIndex, int imageFd, int semaphoreFd);

    Output GetOutput() const { return m_output; }

    void Render(uint32_t index, uint64_t waitValue);

private:
    struct InImage {
        VkImage image;
        VkDeviceMemory memory;
        VkSemaphore semaphore;
        VkImageView view;
        VkDescriptorSet descriptor;
    };

    uint32_t memoryTypeIndex(VkMemoryPropertyFlags properties, uint32_t type_bits) const;
    VkResult createBuffer(VkBufferUsageFlags usageFlags, VkMemoryPropertyFlags memoryPropertyFlags, VkBuffer *buffer, VkDeviceMemory *memory, VkDeviceSize size, void *data = nullptr);
    void submitWork(VkCommandBuffer cmdBuffer);
    VkShaderModule loadShader(const unsigned char *data, unsigned len);

    struct {
        PFN_vkImportSemaphoreFdKHR vkImportSemaphoreFdKHR;
        bool haveDrmModifiers = false;
    } d;

    Output m_output;
    std::vector<InImage> m_images;

    VkInstance m_inst;
    VkDevice m_dev;
    VkPhysicalDevice m_physDev;
    VkQueue m_queue;
    VkFormat m_format;
    VkCommandPool m_commandPool;
    VkSampler m_sampler;
    VkBuffer m_vertexBuffer;
    VkDeviceMemory m_vertexMemory;
    VkRenderPass m_renderPass;
    VkDescriptorPool m_descriptorPool;
    VkDescriptorSetLayout m_descriptorLayout;
    VkPipeline m_pipeline;
    VkPipelineLayout m_pipelineLayout;
    VkCommandBuffer m_commandBuffer;
    VkFence m_fence;
};
