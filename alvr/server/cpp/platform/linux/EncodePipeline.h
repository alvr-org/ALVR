#pragma once
#include <cstdint>
#include <memory>
#include <vector>

extern "C" struct AVCodecContext;

namespace alvr
{

class VkFrame;
class VkFrameCtx;

class EncodePipeline
{
public:
  virtual ~EncodePipeline();

  virtual void PushFrame(uint32_t frame_index, bool idr) = 0;
  bool GetEncoded(std::vector<uint8_t> & out);

  void SetBitrate(int64_t bitrate);
  static std::unique_ptr<EncodePipeline> Create(std::vector<VkFrame> &input_frames, VkFrameCtx &vk_frame_ctx);
protected:
  AVCodecContext *encoder_ctx = nullptr; //shall be initialized by child class
};

}
