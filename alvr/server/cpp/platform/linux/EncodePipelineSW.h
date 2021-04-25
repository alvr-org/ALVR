#pragma once

#include "EncodePipeline.h"

extern "C" struct AVCodecContext;
extern "C" struct AVFrame;
extern "C" struct SwsContext;

namespace alvr
{

class EncodePipelineSW: public EncodePipeline
{
public:
  ~EncodePipelineSW();
  EncodePipelineSW(std::vector<VkFrame> &input_frames, VkFrameCtx& vk_frame_ctx);

  void EncodeFrame(uint32_t frame_index, bool idr, std::vector<uint8_t>& out) override;
private:
  AVCodecContext *encoder_ctx = nullptr;
  std::vector<AVFrame *> vk_frames;
  AVFrame * transferred_frame = nullptr;
  AVFrame * encoder_frame = nullptr;
  SwsContext *scaler_ctx = nullptr;
};
}
