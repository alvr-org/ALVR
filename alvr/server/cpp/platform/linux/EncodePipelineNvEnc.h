#pragma once

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

  void PushFrame(uint32_t frame_index, bool idr) override;

private:
  AVBufferRef *hw_ctx = nullptr;
  std::vector<AVFrame *> vk_frames;
  AVFrame * hw_frame = nullptr;
};
}
