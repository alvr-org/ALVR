#include "EncodePipelineVulkan.h"
#include "FormatConverter.h"
#include "ALVR-common/packet_types.h"
#include "alvr_server/Settings.h"
#include "ffmpeg_helper.h"

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/hwcontext.h>
#include <libavutil/opt.h>
}

namespace alvr
{

static const char *encoderName(ALVR_CODEC codec)
{
    switch (codec) {
    case ALVR_CODEC_H264:
        return "h264_vulkan";
    case ALVR_CODEC_H265:
        return "hevc_vulkan";
    }
    throw std::runtime_error("invalid codec " + std::to_string(codec));
}

EncodePipelineVulkan::EncodePipelineVulkan(Renderer *render, VkContext &vk_ctx, uint32_t width, uint32_t height)
    : EncodePipeline()
    , r(render)
{
    const auto &settings = Settings::Instance();

    auto codec_id = ALVR_CODEC(settings.m_codec);
    const char *encoder_name = encoderName(codec_id);
    const AVCodec *codec = avcodec_find_encoder_by_name(encoder_name);
    if (!codec) {
        throw std::runtime_error(std::string("Failed to find encoder ") + encoder_name);
    }

    encoder_ctx = avcodec_alloc_context3(codec);
    if (!encoder_ctx) {
        throw std::runtime_error("failed to allocate Vulkan encoder");
    }

    switch (codec_id) {
    case ALVR_CODEC_H264:
        encoder_ctx->profile = FF_PROFILE_H264_MAIN;
        switch (settings.m_entropyCoding) {
        case ALVR_CABAC:
            av_opt_set(encoder_ctx->priv_data, "coder", "cabac", 0);
            break;
        case ALVR_CAVLC:
            av_opt_set(encoder_ctx->priv_data, "coder", "vlc", 0);
            break;
        }
        break;
    case ALVR_CODEC_H265:
        encoder_ctx->profile = settings.m_use10bitEncoder ? FF_PROFILE_HEVC_MAIN_10 : FF_PROFILE_HEVC_MAIN;
        break;
    }

    encoder_ctx->width = width;
    encoder_ctx->height = height;
    encoder_ctx->time_base = {1, (int)1e9};
    encoder_ctx->framerate = AVRational{settings.m_refreshRate, 1};
    encoder_ctx->sample_aspect_ratio = AVRational{1, 1};
    encoder_ctx->pix_fmt = AV_PIX_FMT_VULKAN;
    encoder_ctx->max_b_frames = 0;
    encoder_ctx->gop_size = INT16_MAX;

    av_opt_set(encoder_ctx->priv_data, "tune", "ull", 0);
    av_opt_set(encoder_ctx->priv_data, "usage", "stream", 0);
    av_opt_set(encoder_ctx->priv_data, "content", "rendered", 0);
    av_opt_set_int(encoder_ctx->priv_data, "units", 0, 0);
    av_opt_set_int(encoder_ctx->priv_data, "async_depth", 1, 0);

    auto params = FfiDynamicEncoderParams {};
    params.updated = true;
    params.bitrate_bps = 30'000'000;
    params.framerate = 60.0;
    SetParams(params);

    int err = 0;
    if (!(frames_ctx = av_hwframe_ctx_alloc(vk_ctx.ctx))) {
        throw std::runtime_error("Failed to create vulkan frame context.");
    }
    AVHWFramesContext *hwframes_ctx = (AVHWFramesContext *)(frames_ctx->data);
    hwframes_ctx->format = AV_PIX_FMT_VULKAN;
    hwframes_ctx->sw_format = AV_PIX_FMT_NV12;
    hwframes_ctx->width = width;
    hwframes_ctx->height = height;
    hwframes_ctx->initial_pool_size = 0;
    if ((err = av_hwframe_ctx_init(frames_ctx)) < 0) {
        av_buffer_unref(&frames_ctx);
        throw alvr::AvException("Failed to initialize vulkan frame context:", err);
    }
    encoder_ctx->hw_frames_ctx = av_buffer_ref(frames_ctx);

    converter = new RgbToNv12(render, r->GetOutput().image, r->GetOutput().imageInfo, r->GetOutput().semaphore);

    for (size_t i = 0; i < encoder_frames.size(); ++i) {
        encoder_frames[i] = createFrame();
    }

    VkQueryPoolCreateInfo queryPoolInfo = {};
    queryPoolInfo.sType = VK_STRUCTURE_TYPE_QUERY_POOL_CREATE_INFO;
    queryPoolInfo.queryType = VK_QUERY_TYPE_TIMESTAMP;
    queryPoolInfo.queryCount = 1;
    VK_CHECK(vkCreateQueryPool(r->m_dev, &queryPoolInfo, nullptr, &query_pool));

    VkCommandBufferAllocateInfo commandBufferInfo = {};
    commandBufferInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
    commandBufferInfo.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    commandBufferInfo.commandPool = r->m_commandPool;
    commandBufferInfo.commandBufferCount = 1;
    VK_CHECK(vkAllocateCommandBuffers(r->m_dev, &commandBufferInfo, &cmd_buffer));

    err = avcodec_open2(encoder_ctx, codec, NULL);
    if (err < 0) {
        throw alvr::AvException("Cannot open video encoder codec:", err);
    }
}

EncodePipelineVulkan::~EncodePipelineVulkan()
{
    if (converter) {
        delete converter;
    }
    for (AVFrame *frame : encoder_frames) {
        if (!frame) {
            continue;
        }
        AVVkFrame *vk_frame = (AVVkFrame*)frame->data[0];
        vkDestroyImage(r->m_dev, vk_frame->img[0], nullptr);
        vkFreeMemory(r->m_dev, vk_frame->mem[0], nullptr);
        vkDestroySemaphore(r->m_dev, vk_frame->sem[0], nullptr);
        av_frame_free(&frame);
    }
    vkDestroyQueryPool(r->m_dev, query_pool, nullptr);
    av_buffer_unref(&frames_ctx);
}

void EncodePipelineVulkan::PushFrame(uint64_t targetTimestampNs, bool idr)
{
    current_frame = (current_frame + 1) % encoder_frames.size();
    AVFrame *frame = encoder_frames[current_frame];

    convertFrame(frame);

    frame->pict_type = idr ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
    frame->pts = targetTimestampNs;

    int err = avcodec_send_frame(encoder_ctx, frame);
    if (err < 0) {
        throw alvr::AvException("avcodec_send_frame failed: ", err);
    }
}

bool EncodePipelineVulkan::GetEncoded(FramePacket &data)
{
    if (!EncodePipeline::GetEncoded(data)) {
        return false;
    }

    uint64_t query;
    VK_CHECK(vkGetQueryPoolResults(r->m_dev, query_pool, 0, 1, sizeof(uint64_t), &query, sizeof(uint64_t), VK_QUERY_RESULT_64_BIT));
    timestamp.gpu = query * r->m_timestampPeriod;

    return true;
}

void EncodePipelineVulkan::convertFrame(AVFrame *frame)
{
    AVVkFrame *vk_frame = (AVVkFrame*)frame->data[0];

    converter->Convert();

    std::array<VkImageMemoryBarrier, 2> imageBarrierIn;
    imageBarrierIn[0] = {};
    imageBarrierIn[0].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrierIn[0].oldLayout = VK_IMAGE_LAYOUT_GENERAL;
    imageBarrierIn[0].newLayout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
    imageBarrierIn[0].image = converter->GetOutput().image;
    imageBarrierIn[0].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrierIn[0].subresourceRange.layerCount = 1;
    imageBarrierIn[0].subresourceRange.levelCount = 1;
    imageBarrierIn[0].srcAccessMask = 0;
    imageBarrierIn[0].dstAccessMask = VK_ACCESS_TRANSFER_READ_BIT;
    imageBarrierIn[0].srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    imageBarrierIn[0].dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    imageBarrierIn[1] = {};
    imageBarrierIn[1].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrierIn[1].oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    imageBarrierIn[1].newLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
    imageBarrierIn[1].image = vk_frame->img[0];
    imageBarrierIn[1].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrierIn[1].subresourceRange.layerCount = 1;
    imageBarrierIn[1].subresourceRange.levelCount = 1;
    imageBarrierIn[1].srcAccessMask = vk_frame->access[0];
    imageBarrierIn[1].dstAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
    imageBarrierIn[1].srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    imageBarrierIn[1].dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;

    std::array<VkImageCopy, 2> imageCopies;
    imageCopies[0].srcSubresource.aspectMask = VK_IMAGE_ASPECT_PLANE_0_BIT;
    imageCopies[0].srcSubresource.mipLevel = 0;
    imageCopies[0].srcSubresource.baseArrayLayer = 0;
    imageCopies[0].srcSubresource.layerCount = 1;
    imageCopies[0].srcOffset.x = 0;
    imageCopies[0].srcOffset.y = 0;
    imageCopies[0].srcOffset.z = 0;
    imageCopies[0].dstSubresource.aspectMask = VK_IMAGE_ASPECT_PLANE_0_BIT;;
    imageCopies[0].dstSubresource.mipLevel = 0;
    imageCopies[0].dstSubresource.baseArrayLayer = 0;
    imageCopies[0].dstSubresource.layerCount = 1;
    imageCopies[0].dstOffset.x = 0;
    imageCopies[0].dstOffset.y = 0;
    imageCopies[0].dstOffset.z = 0;
    imageCopies[0].extent.width = frame->width;
    imageCopies[0].extent.height = frame->height;
    imageCopies[0].extent.depth = 1;
    imageCopies[1].srcSubresource.aspectMask = VK_IMAGE_ASPECT_PLANE_1_BIT;
    imageCopies[1].srcSubresource.mipLevel = 0;
    imageCopies[1].srcSubresource.baseArrayLayer = 0;
    imageCopies[1].srcSubresource.layerCount = 1;
    imageCopies[1].srcOffset.x = 0;
    imageCopies[1].srcOffset.y = 0;
    imageCopies[1].srcOffset.z = 0;
    imageCopies[1].dstSubresource.aspectMask = VK_IMAGE_ASPECT_PLANE_1_BIT;;
    imageCopies[1].dstSubresource.mipLevel = 0;
    imageCopies[1].dstSubresource.baseArrayLayer = 0;
    imageCopies[1].dstSubresource.layerCount = 1;
    imageCopies[1].dstOffset.x = 0;
    imageCopies[1].dstOffset.y = 0;
    imageCopies[1].dstOffset.z = 0;
    imageCopies[1].extent.width = frame->width / 2;
    imageCopies[1].extent.height = frame->height / 2;
    imageCopies[1].extent.depth = 1;

    std::array<VkImageMemoryBarrier, 1> imageBarrierOut;
    imageBarrierOut[0] = {};
    imageBarrierOut[0].sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    imageBarrierOut[0].oldLayout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
    imageBarrierOut[0].newLayout = VK_IMAGE_LAYOUT_GENERAL;
    imageBarrierOut[0].image = converter->GetOutput().image;
    imageBarrierOut[0].subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    imageBarrierOut[0].subresourceRange.layerCount = 1;
    imageBarrierOut[0].subresourceRange.levelCount = 1;
    imageBarrierOut[0].srcAccessMask = VK_ACCESS_TRANSFER_READ_BIT;
    imageBarrierOut[0].dstAccessMask = 0;
    imageBarrierOut[0].srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    imageBarrierOut[0].dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;

    VkCommandBufferBeginInfo commandBufferBegin = {};
    commandBufferBegin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    VK_CHECK(vkBeginCommandBuffer(cmd_buffer, &commandBufferBegin));

    vkCmdResetQueryPool(cmd_buffer, query_pool, 0, 1);

    vkCmdPipelineBarrier(cmd_buffer, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0, 0, nullptr, 0, nullptr, imageBarrierIn.size(), imageBarrierIn.data());
    vkCmdCopyImage(cmd_buffer, converter->GetOutput().image, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, vk_frame->img[0], VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, imageCopies.size(), imageCopies.data());
    vkCmdPipelineBarrier(cmd_buffer, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0, 0, nullptr, 0, nullptr, imageBarrierOut.size(), imageBarrierOut.data());

    vkCmdWriteTimestamp(cmd_buffer, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, query_pool, 0);

    VK_CHECK(vkEndCommandBuffer(cmd_buffer));

    uint64_t signalValue = vk_frame->sem_value[0] + 1;

    VkTimelineSemaphoreSubmitInfo timelineInfo = {};
    timelineInfo.sType = VK_STRUCTURE_TYPE_TIMELINE_SEMAPHORE_SUBMIT_INFO;
    timelineInfo.signalSemaphoreValueCount = 1;
    timelineInfo.pSignalSemaphoreValues = &signalValue;

    VkSemaphore waitSemaphore = converter->GetOutput().semaphore;
    VkPipelineStageFlags waitStage = VK_PIPELINE_STAGE_TRANSFER_BIT;

    VkSubmitInfo submitInfo = {};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.pNext = &timelineInfo;
    submitInfo.waitSemaphoreCount = 1;
    submitInfo.pWaitSemaphores = &waitSemaphore;
    submitInfo.pWaitDstStageMask = &waitStage;
    submitInfo.signalSemaphoreCount = 1;
    submitInfo.pSignalSemaphores = &vk_frame->sem[0];
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &cmd_buffer;
    VK_CHECK(vkQueueSubmit(r->m_queue, 1, &submitInfo, nullptr));

    vk_frame->sem_value[0] = signalValue;
    vk_frame->layout[0] = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
    vk_frame->access[0] = VK_ACCESS_TRANSFER_WRITE_BIT;
}

AVFrame *EncodePipelineVulkan::createFrame() const
{
    AVVkFrame *vk_frame = av_vk_frame_alloc();

    typedef struct alvr_VkVideoEncodeH264ProfileInfoEXT {
    VkStructureType           sType;
    const void*               pNext;
    StdVideoH264ProfileIdc    stdProfileIdc;
    } alvr_VkVideoEncodeH264ProfileInfoEXT;

    alvr_VkVideoEncodeH264ProfileInfoEXT h264encProfileInfo = {};
    h264encProfileInfo.sType = static_cast<VkStructureType>(1000038007); // VK_STRUCTURE_TYPE_VIDEO_ENCODE_H264_PROFILE_INFO_EXT;
    h264encProfileInfo.stdProfileIdc = STD_VIDEO_H264_PROFILE_IDC_HIGH;

    VkVideoProfileInfoKHR encProfileInfo = {};
    encProfileInfo.sType = VK_STRUCTURE_TYPE_VIDEO_PROFILE_INFO_KHR;
    encProfileInfo.pNext = &h264encProfileInfo;
    encProfileInfo.videoCodecOperation = static_cast<VkVideoCodecOperationFlagBitsKHR>(0x00010000); // VK_VIDEO_CODEC_OPERATION_ENCODE_H264_BIT_EXT;
    encProfileInfo.chromaSubsampling = VK_VIDEO_CHROMA_SUBSAMPLING_420_BIT_KHR;
    encProfileInfo.lumaBitDepth = VK_VIDEO_COMPONENT_BIT_DEPTH_8_BIT_KHR;
    encProfileInfo.chromaBitDepth = VK_VIDEO_COMPONENT_BIT_DEPTH_8_BIT_KHR;

    VkVideoProfileListInfoKHR profilesInfo = {};
    profilesInfo.sType = VK_STRUCTURE_TYPE_VIDEO_PROFILE_LIST_INFO_KHR;
    profilesInfo.profileCount = 1;
    profilesInfo.pProfiles = &encProfileInfo;

    VkImageCreateInfo imageInfo = {};
    imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    imageInfo.pNext = &profilesInfo;
    imageInfo.imageType = VK_IMAGE_TYPE_2D;
    imageInfo.format = converter->GetOutput().imageInfo.format;
    imageInfo.extent = converter->GetOutput().imageInfo.extent;
    imageInfo.arrayLayers = 1;
    imageInfo.mipLevels = 1;
    imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
    imageInfo.tiling = VK_IMAGE_TILING_OPTIMAL;
    imageInfo.usage = VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT | 0x00004000; // VK_IMAGE_USAGE_VIDEO_ENCODE_SRC_BIT_KHR
    imageInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    VK_CHECK(vkCreateImage(r->m_dev, &imageInfo, nullptr, &vk_frame->img[0]));

    VkMemoryRequirements memoryReqs;
    vkGetImageMemoryRequirements(r->m_dev, vk_frame->img[0], &memoryReqs);
    VkMemoryAllocateInfo memoryAllocInfo = {};
    memoryAllocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    memoryAllocInfo.allocationSize = memoryReqs.size;
    memoryAllocInfo.memoryTypeIndex = r->memoryTypeIndex(VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT, memoryReqs.memoryTypeBits);
    VK_CHECK(vkAllocateMemory(r->m_dev, &memoryAllocInfo, nullptr, &vk_frame->mem[0]));
    VK_CHECK(vkBindImageMemory(r->m_dev, vk_frame->img[0], vk_frame->mem[0], 0));

    VkSemaphoreTypeCreateInfo timelineInfo = {};
    timelineInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO;
    timelineInfo.semaphoreType = VK_SEMAPHORE_TYPE_TIMELINE;
    VkSemaphoreCreateInfo semInfo = {};
    semInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    semInfo.pNext = &timelineInfo;
    vkCreateSemaphore(r->m_dev, &semInfo, nullptr, &vk_frame->sem[0]);

    vk_frame->layout[0] = VK_IMAGE_LAYOUT_UNDEFINED;
#if LIBAVUTIL_VERSION_MINOR >= 43 /// XXX
    vk_frame->queue_family[0] = VK_QUEUE_FAMILY_IGNORED;
#endif

    AVFrame *frame = av_frame_alloc();
    frame->width = imageInfo.extent.width;
    frame->height = imageInfo.extent.height;
    frame->hw_frames_ctx = av_buffer_ref(frames_ctx);
    frame->format = AV_PIX_FMT_VULKAN;
    frame->data[0] = (uint8_t*)vk_frame;
    frame->buf[0] = av_buffer_alloc(1);
    frame->buf[0]->data = (uint8_t*)vk_frame;
    return frame;
}

} // namespace alvr
