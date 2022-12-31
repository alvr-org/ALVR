#pragma once

#include "EncodePipeline.h"
#include "FormatConverter.h"

extern "C" struct AVFrame;
extern "C" struct AVBufferRef;

namespace alvr
{

class EncodePipelineVulkan: public EncodePipeline
{
public:
    EncodePipelineVulkan(Renderer *render, VkContext &vk_ctx, uint32_t width, uint32_t height);
    ~EncodePipelineVulkan();

    void PushFrame(uint64_t targetTimestampNs, bool idr) override;
    bool GetEncoded(FramePacket &data) override;

private:
    AVFrame *createFrame() const;
    void convertFrame(AVFrame *frame);

    Renderer *r = nullptr;
    RgbToNv12 *converter = nullptr;
    AVBufferRef *frames_ctx = nullptr;
    uint8_t current_frame = 0;
    std::array<AVFrame*, 3> encoder_frames = {};
    VkQueryPool query_pool = VK_NULL_HANDLE;
    VkCommandBuffer cmd_buffer = VK_NULL_HANDLE;
};

}
