#pragma once

#include <vulkan/vulkan.hpp>
#include <memory>

extern "C" struct AVBufferRef;
extern "C" struct AVDictionary;
extern "C" struct AVVkFrame;
extern "C" struct AVFrame;

namespace alvr
{

// Utility class to build an exception from an ffmpeg return code.
// Messages are rarely useful however.
class AvException: public std::runtime_error
{
public:
  AvException(std::string msg, int averror): std::runtime_error{makemsg(msg, averror)} {}
private:
  static std::string makemsg(const std::string & msg, int averror);
};

class VkContext
{
public:
  // structure that holds extensions methods
  struct dispatch
  {
    PFN_vkImportSemaphoreFdKHR vkImportSemaphoreFdKHR;
  };

  VkContext(const char* device, AVDictionary* opt = nullptr);
  ~VkContext();
  vk::Device get_vk_device() const;

  AVBufferRef *ctx;
  dispatch d;
};

class VkFrameCtx
{
public:
  VkFrameCtx(VkContext & vkContext, vk::ImageCreateInfo image_create_info);
  ~VkFrameCtx();

  AVBufferRef *ctx;
};

class VkFrame
{
public:
  VkFrame(
      const VkContext& vk_ctx,
      vk::ImageCreateInfo image_create_info,
      size_t memory_index,
      int image_fd,
      int semaphore_fd);
  ~VkFrame();
  operator AVVkFrame*() const { return av_vkframe;}
  std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> make_av_frame(VkFrameCtx & frame_ctx);
private:
  AVVkFrame* av_vkframe;
  const uint32_t width;
  const uint32_t height;
  vk::Device device;
};

}
