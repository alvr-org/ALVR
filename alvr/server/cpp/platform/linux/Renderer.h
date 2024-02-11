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
    enum class ExternalHandle {
        None,
        DmaBuf,
        OpaqueFd
    };

    struct Output {
        VkImage image = VK_NULL_HANDLE;
        VkImageLayout layout = VK_IMAGE_LAYOUT_UNDEFINED;
        VkImageCreateInfo imageInfo;
        VkDeviceSize size = 0;
        VkDeviceMemory memory = VK_NULL_HANDLE;
        VkSemaphore semaphore = VK_NULL_HANDLE;
        // ---
        VkImageView view = VK_NULL_HANDLE;
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

    void CreateOutput(uint32_t width, uint32_t height, ExternalHandle handle);
    void ImportOutput(const DrmImage &drm);

    void Render(uint32_t index, uint64_t waitValue);

    void Sync();

    Output &GetOutput();
    Timestamps GetTimestamps();

    void CaptureInputFrame(const std::string &filename);
    void CaptureOutputFrame(const std::string &filename);

    static std::string result_to_str(VkResult result);

// private:
    struct InputImage {
        VkImage image = VK_NULL_HANDLE;
        VkImageLayout layout = VK_IMAGE_LAYOUT_UNDEFINED;
        VkDeviceMemory memory = VK_NULL_HANDLE;
        VkSemaphore semaphore = VK_NULL_HANDLE;
        VkImageView view = VK_NULL_HANDLE;
    };

    struct StagingImage {
        VkImage image = VK_NULL_HANDLE;
        VkImageLayout layout = VK_IMAGE_LAYOUT_UNDEFINED;
        VkDeviceMemory memory = VK_NULL_HANDLE;
        VkImageView view = VK_NULL_HANDLE;
    };

    void commandBufferBegin();
    void commandBufferSubmit();
    void addStagingImage(uint32_t width, uint32_t height);
    void dumpImage(VkImage image, VkImageView imageView, VkImageLayout imageLayout, uint32_t width, uint32_t height, const std::string &filename);
    uint32_t memoryTypeIndex(VkMemoryPropertyFlags properties, uint32_t typeBits) const;

    struct {
        PFN_vkImportSemaphoreFdKHR vkImportSemaphoreFdKHR = nullptr;
        PFN_vkGetMemoryFdKHR vkGetMemoryFdKHR = nullptr;
        PFN_vkGetMemoryFdPropertiesKHR vkGetMemoryFdPropertiesKHR = nullptr;
        PFN_vkGetImageDrmFormatModifierPropertiesEXT vkGetImageDrmFormatModifierPropertiesEXT = nullptr;
        PFN_vkGetCalibratedTimestampsEXT vkGetCalibratedTimestampsEXT = nullptr;
        PFN_vkCmdPushDescriptorSetKHR vkCmdPushDescriptorSetKHR = nullptr;
        bool haveDmaBuf = false;
        bool haveDrmModifiers = false;
        bool haveCalibratedTimestamps = false;
    } d;

    Output m_output;
    std::vector<InputImage> m_images;
    std::vector<StagingImage> m_stagingImages;
    std::vector<RenderPipeline*> m_pipelines;

    VkInstance m_inst = VK_NULL_HANDLE;
    VkDevice m_dev = VK_NULL_HANDLE;
    VkPhysicalDevice m_physDev = VK_NULL_HANDLE;
    VkQueue m_queue = VK_NULL_HANDLE;
    uint32_t m_queueFamilyIndex = 0;
    VkFormat m_format = VK_FORMAT_UNDEFINED;
    VkExtent2D m_imageSize = {0, 0};
    VkQueryPool m_queryPool = VK_NULL_HANDLE;
    VkCommandPool m_commandPool = VK_NULL_HANDLE;
    VkSampler m_sampler = VK_NULL_HANDLE;
    VkDescriptorSetLayout m_descriptorLayout = VK_NULL_HANDLE;
    VkCommandBuffer m_commandBuffer = VK_NULL_HANDLE;
    VkFence m_fence = VK_NULL_HANDLE;
    double m_timestampPeriod = 0;

    size_t m_quadShaderSize = 0;
    const uint32_t *m_quadShaderCode = nullptr;

    std::string m_inputImageCapture;
    std::string m_outputImageCapture;
};

class RenderPipeline
{
public:
    explicit RenderPipeline(Renderer *render);
    virtual ~RenderPipeline();

    void SetShader(const char *filename);
    void SetShader(const unsigned char *data, unsigned len);

    template <typename T>
    void SetConstants(const T *data, std::vector<VkSpecializationMapEntry> &&entries) {
        m_constant = static_cast<const void*>(data);
        m_constantSize = sizeof(T);
        m_constantEntries = std::move(entries);
    }

private:
    void Build();
    void Render(VkImageView in, VkImageView out, VkRect2D outSize);

    Renderer *r;
    VkShaderModule m_shader = VK_NULL_HANDLE;
    const void *m_constant = nullptr;
    uint32_t m_constantSize = 0;
    std::vector<VkSpecializationMapEntry> m_constantEntries;
    VkPipeline m_pipeline = VK_NULL_HANDLE;
    VkPipelineLayout m_pipelineLayout = VK_NULL_HANDLE;

    friend class Renderer;
};
