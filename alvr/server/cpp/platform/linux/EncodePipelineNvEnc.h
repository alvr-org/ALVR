#pragma once

#include "EncodePipeline.h"
#include "VkContext.hpp"
#include "ffmpeg_helper.h"
#include <functional>

extern "C" struct AVBufferRef;
extern "C" struct AVCodecContext;
extern "C" struct AVFrame;

class Renderer;

namespace alvr {

class EncodePipelineNvEnc : public EncodePipeline {
public:
    ~EncodePipelineNvEnc();
    EncodePipelineNvEnc(
        Renderer* render,
        HWContext& vk_ctx,
        VkContext& v_ctx,
        VkFrame& input_frame,
        VkFrameCtx& vk_frame_ctx,
        uint32_t width,
        uint32_t height
    );

    void PushFrame(uint64_t targetTimestampNs, bool idr) override;

private:
    VkContext& v_ctx;
    AVBufferRef* hw_ctx = nullptr;
    std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> vk_frame;
    AVFrame* hw_frame = nullptr;
};
}
