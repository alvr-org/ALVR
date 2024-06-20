#pragma once

#include <functional>
#include <memory>

extern "C" {
#include <libavcodec/avcodec.h>

#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>

#include <libavutil/avutil.h>
#include <libavutil/buffer.h>
#include <libavutil/dict.h>
#include <libavutil/hwcontext.h>
#include <libavutil/hwcontext_drm.h>
#include <libavutil/hwcontext_vulkan.h>
#include <libavutil/opt.h>
}

#include "VkContext.hpp"

namespace alvr {

// Utility class to build an exception from an ffmpeg return code.
// Messages are rarely useful however.
class AvException : public std::runtime_error {
public:
    AvException(std::string msg, int averror)
        : std::runtime_error { makemsg(msg, averror) } { }

private:
    static std::string makemsg(const std::string& msg, int averror);
};

struct DrmImage {
    int fd = -1;
    uint32_t format = 0;
    uint64_t modifier = 0;
    uint32_t planes = 0;
    std::array<uint32_t, 4> strides;
    std::array<uint32_t, 4> offsets;
};

class HWContext {
public:
    AVBufferRef* avCtx;

    HWContext(VkContext const& vkCtx) {
        avCtx = av_hwdevice_ctx_alloc(AV_HWDEVICE_TYPE_VULKAN);
        auto hwCtx = (AVHWDeviceContext*)avCtx->data;
        auto avVk = (AVVulkanDeviceContext*)hwCtx->hwctx;

        avVk->alloc = nullptr;
        avVk->get_proc_addr = vkGetInstanceProcAddr;

        avVk->inst = vkCtx.instance;
        avVk->phys_dev = vkCtx.physDev;
        avVk->act_dev = vkCtx.dev;

        avVk->device_features = vkCtx.meta.feats;

        auto queueFam = vkCtx.meta.queueFamily;

        avVk->nb_graphics_queues = 1;
        avVk->queue_family_index = queueFam;

        avVk->nb_tx_queues = 1;
        avVk->queue_family_tx_index = queueFam;

        avVk->nb_comp_queues = 1;
        avVk->queue_family_comp_index = queueFam;

        avVk->nb_encode_queues = 0;
        avVk->queue_family_encode_index = -1;

        avVk->nb_decode_queues = 0;
        avVk->queue_family_decode_index = -1;

        avVk->nb_enabled_inst_extensions = vkCtx.meta.instExtensions.size();
        avVk->enabled_inst_extensions = vkCtx.meta.instExtensions.data();

        avVk->nb_enabled_dev_extensions = vkCtx.meta.devExtensions.size();
        avVk->enabled_dev_extensions = vkCtx.meta.devExtensions.data();

        int ret = av_hwdevice_ctx_init(avCtx);
        if (ret)
            throw AvException("failed to initialize ffmpeg", ret);
    }

    ~HWContext() { av_buffer_unref(&avCtx); }
};

class VkFrameCtx {
public:
    VkFrameCtx(VkContext& vkContext, vk::ImageCreateInfo image_create_info);
    ~VkFrameCtx();

    AVBufferRef* ctx;
};

class VkFrame {
public:
    VkFrame(
        const VkContext& vk_ctx,
        VkImage image,
        VkImageCreateInfo image_info,
        VkDeviceSize size,
        VkDeviceMemory memory,
        DrmImage drm
    );
    ~VkFrame();
    VkImage image() { return vkimage; }
    VkImageCreateInfo imageInfo() { return vkimageinfo; }
    VkFormat format() { return vkimageinfo.format; }
    AVPixelFormat avFormat() { return avformat; }
    operator AVVkFrame*() const { return av_vkframe; }
    operator AVDRMFrameDescriptor*() const { return av_drmframe; }
    std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> make_av_frame(VkFrameCtx& frame_ctx);

private:
    AVVkFrame* av_vkframe = nullptr;
    AVDRMFrameDescriptor* av_drmframe = nullptr;
    vk::Device device;
    VkImage vkimage;
    VkImageCreateInfo vkimageinfo;
    AVPixelFormat avformat;
};

}
