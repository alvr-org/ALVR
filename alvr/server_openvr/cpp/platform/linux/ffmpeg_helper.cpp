#include "ffmpeg_helper.h"

#include <chrono>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>

#include "alvr_server/Logger.h"
#include "alvr_server/bindings.h"

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavutil/avutil.h>
}

namespace {
// it seems that ffmpeg does not provide this mapping
AVPixelFormat vk_format_to_av_format(vk::Format vk_fmt) {
    for (int f = AV_PIX_FMT_NONE; f < AV_PIX_FMT_NB; ++f) {
        auto current_fmt = av_vkfmt_from_pixfmt(AVPixelFormat(f));
        if (current_fmt and *current_fmt == (VkFormat)vk_fmt)
            return AVPixelFormat(f);
    }
    throw std::runtime_error("unsupported vulkan pixel format " + std::to_string((VkFormat)vk_fmt));
}
}

std::string alvr::AvException::makemsg(const std::string& msg, int averror) {
    char av_msg[AV_ERROR_MAX_STRING_SIZE];
    av_strerror(averror, av_msg, sizeof(av_msg));
    return msg + " " + av_msg;
}

alvr::VkContext::VkContext(
    const uint8_t* deviceUUID, const std::vector<const char*>& requiredDeviceExtensions
) {
    std::vector<const char*> instance_extensions = {
        VK_KHR_GET_PHYSICAL_DEVICE_PROPERTIES_2_EXTENSION_NAME,
        VK_KHR_SURFACE_EXTENSION_NAME,
    };

    std::vector<const char*> device_extensions = {
        VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME,
        VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME,
        VK_EXT_EXTERNAL_MEMORY_DMA_BUF_EXTENSION_NAME,
        VK_EXT_IMAGE_DRM_FORMAT_MODIFIER_EXTENSION_NAME,
        VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME,
        VK_EXT_EXTERNAL_MEMORY_HOST_EXTENSION_NAME,
        VK_KHR_PUSH_DESCRIPTOR_EXTENSION_NAME,
        VK_KHR_SAMPLER_YCBCR_CONVERSION_EXTENSION_NAME,
        VK_EXT_PHYSICAL_DEVICE_DRM_EXTENSION_NAME,
        VK_EXT_CALIBRATED_TIMESTAMPS_EXTENSION_NAME,
    };
    device_extensions.insert(
        device_extensions.end(), requiredDeviceExtensions.begin(), requiredDeviceExtensions.end()
    );

    uint32_t instanceExtensionCount = 0;
    vkEnumerateInstanceExtensionProperties(nullptr, &instanceExtensionCount, nullptr);
    std::vector<VkExtensionProperties> instanceExts(instanceExtensionCount);
    vkEnumerateInstanceExtensionProperties(nullptr, &instanceExtensionCount, instanceExts.data());
    for (const char* name : instance_extensions) {
        auto it = std::find_if(
            instanceExts.begin(), instanceExts.end(), [name](VkExtensionProperties e) {
                return strcmp(e.extensionName, name) == 0;
            }
        );
        if (it != instanceExts.end()) {
            instanceExtensions.push_back(name);
        }
    }

    VkApplicationInfo appInfo = {};
    appInfo.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    appInfo.pApplicationName = "ALVR";
    appInfo.apiVersion = VK_API_VERSION_1_2;

    VkInstanceCreateInfo instanceInfo = {};
    instanceInfo.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    instanceInfo.pApplicationInfo = &appInfo;

#ifdef DEBUG
    const char* validationLayers[] = { "VK_LAYER_KHRONOS_validation" };
    instanceInfo.ppEnabledLayerNames = validationLayers;
    instanceInfo.enabledLayerCount = 1;
#endif

    instanceInfo.enabledExtensionCount = instanceExtensions.size();
    instanceInfo.ppEnabledExtensionNames = instanceExtensions.data();
    VK_CHECK(vkCreateInstance(&instanceInfo, nullptr, &instance));

    uint32_t deviceCount = 0;
    VK_CHECK(vkEnumeratePhysicalDevices(instance, &deviceCount, nullptr));
    std::vector<VkPhysicalDevice> physicalDevices(deviceCount);
    VK_CHECK(vkEnumeratePhysicalDevices(instance, &deviceCount, physicalDevices.data()));
    for (VkPhysicalDevice dev : physicalDevices) {
        VkPhysicalDeviceVulkan11Properties props11 = {};
        props11.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_1_PROPERTIES;

        VkPhysicalDeviceProperties2 props = {};
        props.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2;
        props.pNext = &props11;
        vkGetPhysicalDeviceProperties2(dev, &props);
        if (memcmp(props11.deviceUUID, deviceUUID, VK_UUID_SIZE) == 0) {
            physicalDevice = dev;
            break;
        }
    }
    if (!physicalDevice && !physicalDevices.empty()) {
        Warn("Falling back to first device");
        physicalDevice = physicalDevices[0];
    }
    if (!physicalDevice) {
        throw std::runtime_error("Failed to find vulkan device.");
    }

    VkPhysicalDeviceDrmPropertiesEXT drmProps = {};
    drmProps.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_DRM_PROPERTIES_EXT;

    VkPhysicalDeviceProperties2 deviceProps = {};
    deviceProps.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2;
    deviceProps.pNext = &drmProps;
    vkGetPhysicalDeviceProperties2(physicalDevice, &deviceProps);

    amd = deviceProps.properties.vendorID == 0x1002;
    intel = deviceProps.properties.vendorID == 0x8086;
    nvidia = deviceProps.properties.vendorID == 0x10de;
    Info("Using Vulkan device %s", deviceProps.properties.deviceName);

    uint32_t deviceExtensionCount = 0;
    VK_CHECK(vkEnumerateDeviceExtensionProperties(
        physicalDevice, nullptr, &deviceExtensionCount, nullptr
    ));
    std::vector<VkExtensionProperties> deviceExts(deviceExtensionCount);
    VK_CHECK(vkEnumerateDeviceExtensionProperties(
        physicalDevice, nullptr, &deviceExtensionCount, deviceExts.data()
    ));
    for (const char* name : device_extensions) {
        auto it
            = std::find_if(deviceExts.begin(), deviceExts.end(), [name](VkExtensionProperties e) {
                  return strcmp(e.extensionName, name) == 0;
              });
        if (it != deviceExts.end()) {
            deviceExtensions.push_back(name);
        }
    }

    float queuePriority = 1.0;
    std::vector<VkDeviceQueueCreateInfo> queueInfos;

    uint32_t queueFamilyCount;
    vkGetPhysicalDeviceQueueFamilyProperties(physicalDevice, &queueFamilyCount, nullptr);
    std::vector<VkQueueFamilyProperties> queueFamilyProperties(queueFamilyCount);
    vkGetPhysicalDeviceQueueFamilyProperties(
        physicalDevice, &queueFamilyCount, queueFamilyProperties.data()
    );
    for (uint32_t i = 0; i < queueFamilyProperties.size(); ++i) {
        const bool graphics = queueFamilyProperties[i].queueFlags & VK_QUEUE_GRAPHICS_BIT;
        const bool compute = queueFamilyProperties[i].queueFlags & VK_QUEUE_COMPUTE_BIT;
        if (compute && (queueFamilyIndex == VK_QUEUE_FAMILY_IGNORED || !graphics)) {
            queueFamilyIndex = i;
        }
        VkDeviceQueueCreateInfo queueInfo = {};
        queueInfo.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
        queueInfo.queueFamilyIndex = i;
        queueInfo.queueCount = 1;
        queueInfo.pQueuePriorities = &queuePriority;
        queueInfos.push_back(queueInfo);
    }

    VkPhysicalDeviceVulkan12Features features12 = {};
    features12.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_2_FEATURES;
    features12.timelineSemaphore = true;

    VkPhysicalDeviceFeatures2 features = {};
    features.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_FEATURES_2;
    features.pNext = &features12;
    features.features.samplerAnisotropy = VK_TRUE;

    VkDeviceCreateInfo deviceInfo = {};
    deviceInfo.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
    deviceInfo.pNext = &features;
    deviceInfo.queueCreateInfoCount = queueInfos.size();
    deviceInfo.pQueueCreateInfos = queueInfos.data();
    deviceInfo.enabledExtensionCount = deviceExtensions.size();
    deviceInfo.ppEnabledExtensionNames = deviceExtensions.data();
    VK_CHECK(vkCreateDevice(physicalDevice, &deviceInfo, nullptr, &device));

    for (int i = 128; i < 136; ++i) {
        auto path = "/dev/dri/renderD" + std::to_string(i);
        int fd = open(path.c_str(), O_RDONLY);
        if (fd == -1) {
            continue;
        }
        struct stat s = {};
        int ret = fstat(fd, &s);
        close(fd);
        if (ret != 0) {
            continue;
        }
        dev_t primaryDev = makedev(drmProps.primaryMajor, drmProps.primaryMinor);
        dev_t renderDev = makedev(drmProps.renderMajor, drmProps.renderMinor);
        if (primaryDev == s.st_rdev || renderDev == s.st_rdev) {
            devicePath = path;
            break;
        }
    }
    if (devicePath.empty()) {
        devicePath = "/dev/dri/renderD128";
    }
    Info("Using device path %s", devicePath.c_str());

    ctx = av_hwdevice_ctx_alloc(AV_HWDEVICE_TYPE_VULKAN);
    AVHWDeviceContext* hwctx = (AVHWDeviceContext*)ctx->data;
    AVVulkanDeviceContext* vkctx = (AVVulkanDeviceContext*)hwctx->hwctx;

    vkctx->alloc = nullptr;
    vkctx->inst = instance;
    vkctx->phys_dev = physicalDevice;
    vkctx->act_dev = device;
    vkctx->device_features = features;
    vkctx->queue_family_index = queueFamilyIndex;
    vkctx->nb_graphics_queues = 1;
    vkctx->queue_family_tx_index = queueFamilyIndex;
    vkctx->nb_tx_queues = 1;
    vkctx->queue_family_comp_index = queueFamilyIndex;
    vkctx->nb_comp_queues = 1;
    vkctx->get_proc_addr = vkGetInstanceProcAddr;
    vkctx->queue_family_encode_index = -1;
    vkctx->nb_encode_queues = 0;
    vkctx->queue_family_decode_index = -1;
    vkctx->nb_decode_queues = 0;

    char** inst_extensions = (char**)malloc(sizeof(char*) * instanceExtensions.size());
    for (uint32_t i = 0; i < instanceExtensions.size(); ++i) {
        inst_extensions[i] = strdup(instanceExtensions[i]);
    }
    vkctx->enabled_inst_extensions = inst_extensions;
    vkctx->nb_enabled_inst_extensions = instanceExtensions.size();

    char** dev_extensions = (char**)malloc(sizeof(char*) * deviceExtensions.size());
    for (uint32_t i = 0; i < deviceExtensions.size(); ++i) {
        dev_extensions[i] = strdup(deviceExtensions[i]);
    }
    vkctx->enabled_dev_extensions = dev_extensions;
    vkctx->nb_enabled_dev_extensions = deviceExtensions.size();

    int ret = av_hwdevice_ctx_init(ctx);
    if (ret)
        throw AvException("failed to initialize ffmpeg", ret);
}

alvr::VkContext::~VkContext() {
    av_buffer_unref(&ctx);
    vkDestroyDevice(device, nullptr);
    vkDestroyInstance(instance, nullptr);
}

alvr::VkFrameCtx::VkFrameCtx(VkContext& vkContext, vk::ImageCreateInfo image_create_info) {
    AVHWFramesContext* frames_ctx = NULL;
    int err = 0;

    if (!(ctx = av_hwframe_ctx_alloc(vkContext.ctx))) {
        throw std::runtime_error("Failed to create vulkan frame context.");
    }
    frames_ctx = (AVHWFramesContext*)(ctx->data);
    frames_ctx->format = AV_PIX_FMT_VULKAN;
    frames_ctx->sw_format = vk_format_to_av_format(image_create_info.format);
    frames_ctx->width = image_create_info.extent.width;
    frames_ctx->height = image_create_info.extent.height;
    frames_ctx->initial_pool_size = 0;
    if ((err = av_hwframe_ctx_init(ctx)) < 0) {
        av_buffer_unref(&ctx);
        throw alvr::AvException("Failed to initialize vulkan frame context:", err);
    }
}

alvr::VkFrameCtx::~VkFrameCtx() { av_buffer_unref(&ctx); }

alvr::VkFrame::VkFrame(
    const VkContext& vk_ctx,
    VkImage image,
    VkImageCreateInfo image_info,
    VkDeviceSize size,
    VkDeviceMemory memory,
    DrmImage drm
)
    : vkimage(image)
    , vkimageinfo(image_info) {
    device = vk_ctx.get_vk_device();
    avformat = vk_format_to_av_format(vk::Format(image_info.format));

    av_drmframe = (AVDRMFrameDescriptor*)malloc(sizeof(AVDRMFrameDescriptor));
    av_drmframe->nb_objects = 1;
    av_drmframe->objects[0].fd = drm.fd;
    av_drmframe->objects[0].size = size;
    av_drmframe->objects[0].format_modifier = drm.modifier;
    av_drmframe->nb_layers = 1;
    av_drmframe->layers[0].format = drm.format;
    av_drmframe->layers[0].nb_planes = drm.planes;
    for (uint32_t i = 0; i < drm.planes; ++i) {
        av_drmframe->layers[0].planes[i].object_index = 0;
        av_drmframe->layers[0].planes[i].pitch = drm.strides[i];
        av_drmframe->layers[0].planes[i].offset = drm.offsets[i];
    }

    av_vkframe = av_vk_frame_alloc();
    av_vkframe->img[0] = image;
    av_vkframe->tiling = image_info.tiling;
    av_vkframe->mem[0] = memory;
    av_vkframe->size[0] = size;
    av_vkframe->layout[0] = VK_IMAGE_LAYOUT_UNDEFINED;

    VkExportSemaphoreCreateInfo exportInfo = {};
    exportInfo.sType = VK_STRUCTURE_TYPE_EXPORT_SEMAPHORE_CREATE_INFO;
    exportInfo.handleTypes = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT;

    VkSemaphoreTypeCreateInfo timelineInfo = {};
    timelineInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO;
    timelineInfo.pNext = &exportInfo;
    timelineInfo.semaphoreType = VK_SEMAPHORE_TYPE_TIMELINE;

    VkSemaphoreCreateInfo semInfo = {};
    semInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    semInfo.pNext = &timelineInfo;
    vkCreateSemaphore(device, &semInfo, nullptr, &av_vkframe->sem[0]);
}

alvr::VkFrame::~VkFrame() {
    free(av_drmframe);
    if (av_vkframe) {
        vkDestroySemaphore(device, av_vkframe->sem[0], nullptr);
        av_free(av_vkframe);
    }
}

std::unique_ptr<AVFrame, std::function<void(AVFrame*)>>
alvr::VkFrame::make_av_frame(VkFrameCtx& frame_ctx) {
    std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> frame {
        av_frame_alloc(), [](AVFrame* p) { av_frame_free(&p); }
    };
    frame->width = vkimageinfo.extent.width;
    frame->height = vkimageinfo.extent.height;
    frame->hw_frames_ctx = av_buffer_ref(frame_ctx.ctx);
    frame->data[0] = (uint8_t*)av_vkframe;
    frame->format = AV_PIX_FMT_VULKAN;
    frame->buf[0] = av_buffer_alloc(1);
    frame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

    return frame;
}
