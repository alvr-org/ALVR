#pragma once

#include <vulkan/vulkan.hpp>
#include <functional>
#include <memory>

#include "generated/avutil_loader.h"
#include "generated/avcodec_loader.h"
#include "generated/avfilter_loader.h"
#include "generated/swscale_loader.h"

namespace alvr
{

class libav
{
public:
	static libav& instance();
	avutil m_avutil;
	avcodec m_avcodec;
	swscale m_swscale;
	avfilter m_avfilter;
private:
	libav();
};

#define AVUTIL ::alvr::libav::instance().m_avutil
#define AVCODEC ::alvr::libav::instance().m_avcodec
#define SWSCALE ::alvr::libav::instance().m_swscale
#define AVFILTER ::alvr::libav::instance().m_avfilter

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
