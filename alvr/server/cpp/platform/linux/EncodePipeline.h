#pragma once
#include <cstdint>
#include <memory>
#include <vector>

extern "C" struct AVCodecContext;

class Renderer;

namespace alvr
{

class VkFrame;
class VkFrameCtx;
class VkContext;

class EncodePipeline
{
public:
  virtual ~EncodePipeline();

  virtual void PushFrame(uint64_t targetTimestampNs, bool idr) = 0;
  virtual bool GetEncoded(std::vector<uint8_t> & out, uint64_t *pts);
  virtual uint64_t GetTimestamp();

  virtual void SetBitrate(int64_t bitrate);
  static std::unique_ptr<EncodePipeline> Create(Renderer *render, VkContext &vk_ctx, VkFrame &input_frame, VkFrameCtx &vk_frame_ctx, uint32_t width, uint32_t height);
protected:
  AVCodecContext *encoder_ctx = nullptr; //shall be initialized by child class
  uint64_t gpu_timestamp = 0;
};

}
