// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#ifndef LIBRARY_LOADER_AVUTIL_LOADER_H
#define LIBRARY_LOADER_AVUTIL_LOADER_H

extern "C" {
#include <stdint.h>
#include <libavutil/avutil.h>
#include <libavutil/dict.h>
#include <libavutil/opt.h>
#include <libavutil/hwcontext.h>
#include <libavutil/hwcontext_vulkan.h>

}


#include <string>

class avutil {
 public:
  avutil();
  ~avutil();

  bool Load(const std::string& library_name)
      __attribute__((warn_unused_result));

  bool loaded() const { return loaded_; }

  decltype(&::av_buffer_alloc) av_buffer_alloc;
  decltype(&::av_buffer_ref) av_buffer_ref;
  decltype(&::av_buffer_unref) av_buffer_unref;
  decltype(&::av_dict_set) av_dict_set;
  decltype(&::av_frame_alloc) av_frame_alloc;
  decltype(&::av_frame_free) av_frame_free;
  decltype(&::av_frame_get_buffer) av_frame_get_buffer;
  decltype(&::av_frame_unref) av_frame_unref;
  decltype(&::av_free) av_free;
  decltype(&::av_hwdevice_ctx_create) av_hwdevice_ctx_create;
  decltype(&::av_hwframe_ctx_alloc) av_hwframe_ctx_alloc;
  decltype(&::av_hwframe_ctx_init) av_hwframe_ctx_init;
  decltype(&::av_hwframe_get_buffer) av_hwframe_get_buffer;
  decltype(&::av_hwframe_map) av_hwframe_map;
  decltype(&::av_hwframe_transfer_data) av_hwframe_transfer_data;
  decltype(&::av_log_set_callback) av_log_set_callback;
  decltype(&::av_log_set_level) av_log_set_level;
  decltype(&::av_opt_set) av_opt_set;
  decltype(&::av_strdup) av_strdup;
  decltype(&::av_strerror) av_strerror;
  decltype(&::av_vkfmt_from_pixfmt) av_vkfmt_from_pixfmt;
  decltype(&::av_vk_frame_alloc) av_vk_frame_alloc;


 private:
  void CleanUp(bool unload);

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  void* library_;
#endif

  bool loaded_;

  // Disallow copy constructor and assignment operator.
  avutil(const avutil&);
  void operator=(const avutil&);
};

#endif  // LIBRARY_LOADER_AVUTIL_LOADER_H
