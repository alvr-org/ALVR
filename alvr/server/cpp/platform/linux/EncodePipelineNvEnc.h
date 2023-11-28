#pragma once

#include <functional>
#include "EncodePipeline.h"

extern "C" struct AVBufferRef;
extern "C" struct AVCodecContext;
extern "C" struct AVFrame;

class Renderer;

namespace alvr
{

class EncodePipelineNvEnc: public EncodePipeline
{
public:
  ~EncodePipelineNvEnc();
  EncodePipelineNvEnc(Renderer *render, VkContext &vk_ctx, VkFrame &input_frame, VkFrameCtx& vk_frame_ctx, uint32_t width, uint32_t height);

  void PushFrame(uint64_t targetTimestampNs, bool idr) override;

private:
  Renderer *r = nullptr;
  AVBufferRef *hw_ctx = nullptr;
  std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> vk_frame;
  AVFrame * hw_frame = nullptr;
};
}
