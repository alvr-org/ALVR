#pragma once

#include <vulkan/vulkan.hpp>
#include <functional>
#include <memory>

extern "C" {
  #include <stdint.h>

  #include <libavcodec/avcodec.h>

  #include <libavfilter/avfilter.h>
  #include <libavfilter/buffersink.h>
  #include <libavfilter/buffersrc.h>

  #include <libavutil/avutil.h>
  #include <libavutil/dict.h>
  #include <libavutil/opt.h>
  #include <libavutil/hwcontext.h>
  #include <libavutil/hwcontext_vulkan.h>
  #include <libavutil/hwcontext_drm.h>
}

#include "Renderer.h"

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
  VkContext(const uint8_t* deviceUUID, const std::vector<const char*> &requiredDeviceExtensions);
  ~VkContext();
  VkDevice get_vk_device() const { return device;}
  VkInstance get_vk_instance() const { return instance;}
  VkPhysicalDevice get_vk_phys_device() const { return physicalDevice;}
  uint32_t get_vk_queue_family_index() const { return queueFamilyIndex;}
  std::vector<const char*> get_vk_instance_extensions() const { return instanceExtensions;}
  std::vector<const char*> get_vk_device_extensions() const { return deviceExtensions;}

  AVBufferRef *ctx = nullptr;
  VkInstance instance = VK_NULL_HANDLE;
  VkPhysicalDevice physicalDevice = VK_NULL_HANDLE;
  VkDevice device = VK_NULL_HANDLE;
  uint32_t queueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
  std::vector<const char*> instanceExtensions;
  std::vector<const char*> deviceExtensions;
  bool amd = false;
  bool intel = false;
  bool nvidia = false;
  std::string devicePath;
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
      VkDeviceMemory memory,
      DrmImage drm);
  ~VkFrame();
  VkImage image() { return vkimage;}
  VkImageCreateInfo imageInfo() { return vkimageinfo;}
  VkFormat format() { return vkimageinfo.format;}
  AVPixelFormat avFormat() { return avformat;}
  operator AVVkFrame*() const { return av_vkframe;}
  operator AVDRMFrameDescriptor*() const { return av_drmframe;}
  std::unique_ptr<AVFrame, std::function<void(AVFrame*)>> make_av_frame(VkFrameCtx & frame_ctx);
private:
  AVVkFrame* av_vkframe = nullptr;
  AVDRMFrameDescriptor* av_drmframe = nullptr;
  vk::Device device;
  VkImage vkimage;
  VkImageCreateInfo vkimageinfo;
  AVPixelFormat avformat;
};

}
