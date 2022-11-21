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

  virtual void PushFrame(uint64_t targetTimestampNs, bool idr) = 0;
  bool GetEncoded(std::vector<uint8_t> & out, uint64_t *pts);

  void SetBitrate(int64_t bitrate);
  static std::unique_ptr<EncodePipeline> Create(VkFrame &input_frame, VkFrameCtx &vk_frame_ctx, uint32_t width, uint32_t height);
protected:
  AVCodecContext *encoder_ctx = nullptr; //shall be initialized by child class
};

}
