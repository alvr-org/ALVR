#pragma once

#include <vector>
#include <string>
#include <array>
#include <iostream>
#include <vulkan/vulkan.h>

#define VK_CHECK(f) \
{ \
    VkResult res = (f); \
    if (res != VK_SUCCESS) { \
        std::cerr << Renderer::result_to_str(res) << "at" << __FILE__ << ":" << __LINE__ << std::endl; \
        throw std::runtime_error("Vulkan: " + Renderer::result_to_str(res) + "at " __FILE__ ":" + std::to_string(__LINE__)); \
    } \
}

struct DrmImage {
    int fd = -1;
    uint32_t format = 0;
    uint64_t modifier = 0;
    uint32_t planes = 0;
    std::array<uint32_t, 4> strides;
    std::array<uint32_t, 4> offsets;
};

class RenderPipeline;

class Renderer
{
public:
    struct Output {
        VkImage image;
        VkImageCreateInfo imageInfo;
        VkDeviceSize size;
        VkDeviceMemory memory;
        VkSemaphore semaphore;
        // ---
        VkImageView view;
        VkFramebuffer framebuffer;
        // ---
        DrmImage drm;
    };

    struct Timestamps {
        uint64_t now;
        uint64_t renderBegin;
        uint64_t renderComplete;
    };

    explicit Renderer(const VkInstance &inst, const VkDevice &dev, const VkPhysicalDevice &physDev, uint32_t queueIdx, const std::vector<const char *> &devExtensions);
    virtual ~Renderer();

    void Startup(uint32_t width, uint32_t height, VkFormat format);

    void AddImage(VkImageCreateInfo imageInfo, size_t memoryIndex, int imageFd, int semaphoreFd);

    void AddPipeline(RenderPipeline *pipeline);

    void CreateOutput(uint32_t width, uint32_t height);

    void Render(uint32_t index, uint64_t waitValue);

    void Sync();

    Output GetOutput();
    Timestamps GetTimestamps();

    void CaptureInputFrame(const std::string &filename);
    void CaptureOutputFrame(const std::string &filename);

    static std::string result_to_str(VkResult result);

// private:
    struct InputImage {
        VkImage image;
        VkDeviceMemory memory;
        VkSemaphore semaphore;
        VkImageView view;
        VkDescriptorSet descriptor;
    };

    struct StagingImage {
        VkImage image;
        VkDeviceMemory memory;
        VkImageView view;
        VkFramebuffer framebuffer;
        VkDescriptorSet descriptor;
    };

    void commandBufferBegin();
    void commandBufferSubmit();
    void addStagingImage(uint32_t width, uint32_t height);
    void dumpImage(VkImage image, VkImageLayout imageLayout, uint32_t width, uint32_t height, const std::string &filename);
    uint32_t memoryTypeIndex(VkMemoryPropertyFlags properties, uint32_t typeBits) const;

    struct {
        PFN_vkImportSemaphoreFdKHR vkImportSemaphoreFdKHR;
        PFN_vkGetMemoryFdKHR vkGetMemoryFdKHR;
        PFN_vkGetImageDrmFormatModifierPropertiesEXT vkGetImageDrmFormatModifierPropertiesEXT;
        PFN_vkGetCalibratedTimestampsEXT vkGetCalibratedTimestampsEXT;
        bool haveDmaBuf = false;
        bool haveDrmModifiers = false;
    } d;

    Output m_output;
    std::vector<InputImage> m_images;
    std::vector<StagingImage> m_stagingImages;
    std::vector<RenderPipeline*> m_pipelines;

    VkInstance m_inst;
    VkDevice m_dev;
    VkPhysicalDevice m_physDev;
    VkQueue m_queue;
    uint32_t m_queueFamilyIndex;
    VkFormat m_format;
    VkExtent2D m_imageSize;
    VkQueryPool m_queryPool;
    VkCommandPool m_commandPool;
    VkSampler m_sampler;
    VkBuffer m_vertexBuffer;
    VkDeviceMemory m_vertexMemory;
    VkRenderPass m_renderPass;
    VkDescriptorPool m_descriptorPool;
    VkDescriptorSetLayout m_descriptorLayout;
    VkCommandBuffer m_commandBuffer;
    VkFence m_fence;
    double m_timestampPeriod;

    std::string m_inputImageCapture;
    std::string m_outputImageCapture;
};

class RenderPipeline
{
public:
    enum ShaderType {
        VertexShader,
        FragmentShader
    };

    explicit RenderPipeline(Renderer *render);
    virtual ~RenderPipeline();

    void SetShader(ShaderType type, const char *filename);
    void SetShader(ShaderType type, const unsigned char *data, unsigned len);
    void SetPushConstant(ShaderType type, const void *data, uint32_t size);

private:
    void Build();
    void Render(VkDescriptorSet in, VkFramebuffer out, VkRect2D outSize);

    Renderer *r;
    VkShaderModule m_vertexShader = VK_NULL_HANDLE;
    VkShaderModule m_fragmentShader = VK_NULL_HANDLE;
    const void *m_vertexConstant = nullptr;
    uint32_t m_vertexConstantSize = 0;
    const void *m_fragmentConstant = nullptr;
    uint32_t m_fragmentConstantSize = 0;
    VkPipeline m_pipeline = VK_NULL_HANDLE;
    VkPipelineLayout m_pipelineLayout = VK_NULL_HANDLE;

    friend class Renderer;
};
