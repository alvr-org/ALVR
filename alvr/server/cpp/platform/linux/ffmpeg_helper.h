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
  VkContext(const char* device);
  ~VkContext();
  VkDevice get_vk_device() const { return device;}
  VkInstance get_vk_instance() const { return instance;}
  VkPhysicalDevice get_vk_phys_device() const { return physicalDevice;}
  uint32_t get_vk_queue_family_index() const { return queueFamilyIndex;}
  std::vector<const char*> get_vk_instance_extensions() const { return instanceExtensions;}
  std::vector<const char*> get_vk_device_extensions() const { return deviceExtensions;}

  AVBufferRef *ctx;
  VkInstance instance;
  VkPhysicalDevice physicalDevice;
  VkDevice device;
  uint32_t queueFamilyIndex;
  std::vector<const char*> instanceExtensions;
  std::vector<const char*> deviceExtensions;
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
      VkImage image,
      VkImageCreateInfo image_info,
      VkDeviceSize size,
      VkDeviceMemory memory);
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
