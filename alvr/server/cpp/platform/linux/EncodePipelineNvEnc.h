#pragma once

#include <functional>
#include "EncodePipeline.h"

extern "C" struct AVBufferRef;
extern "C" struct AVCodecContext;
extern "C" struct AVFrame;

namespace alvr
{

class EncodePipelineNvEnc: public EncodePipeline
{
public:
  ~EncodePipelineNvEnc();
  EncodePipelineNvEnc(std::vector<VkFrame> &input_frames, VkFrameCtx& vk_frame_ctx);

  void PushFrame(uint32_t frame_index, uint64_t targetTimestampNs, bool idr) override;

private:
  AVBufferRef *hw_ctx = nullptr;
  std::vector<std::unique_ptr<AVFrame, std::function<void(AVFrame*)>>> vk_frames;
  AVFrame * hw_frame = nullptr;
};
}
