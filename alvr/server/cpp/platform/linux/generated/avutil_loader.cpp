// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#include "avutil_loader.h"

#include <dlfcn.h>

avutil::avutil() : loaded_(false) {
}

avutil::~avutil() {
  CleanUp(loaded_);
}

bool avutil::Load(const std::string& library_name) {
  if (loaded_)
    return false;

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  library_ = dlopen(library_name.c_str(), RTLD_LAZY);
  if (!library_)
    return false;
#else
  (void)library_name;
#endif


#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_buffer_alloc =
      reinterpret_cast<decltype(this->av_buffer_alloc)>(
          dlsym(library_, "av_buffer_alloc"));
#else
  av_buffer_alloc = &::av_buffer_alloc;
#endif
  if (!av_buffer_alloc) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_buffer_ref =
      reinterpret_cast<decltype(this->av_buffer_ref)>(
          dlsym(library_, "av_buffer_ref"));
#else
  av_buffer_ref = &::av_buffer_ref;
#endif
  if (!av_buffer_ref) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_buffer_unref =
      reinterpret_cast<decltype(this->av_buffer_unref)>(
          dlsym(library_, "av_buffer_unref"));
#else
  av_buffer_unref = &::av_buffer_unref;
#endif
  if (!av_buffer_unref) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_dict_set =
      reinterpret_cast<decltype(this->av_dict_set)>(
          dlsym(library_, "av_dict_set"));
#else
  av_dict_set = &::av_dict_set;
#endif
  if (!av_dict_set) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_frame_alloc =
      reinterpret_cast<decltype(this->av_frame_alloc)>(
          dlsym(library_, "av_frame_alloc"));
#else
  av_frame_alloc = &::av_frame_alloc;
#endif
  if (!av_frame_alloc) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_frame_free =
      reinterpret_cast<decltype(this->av_frame_free)>(
          dlsym(library_, "av_frame_free"));
#else
  av_frame_free = &::av_frame_free;
#endif
  if (!av_frame_free) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_frame_get_buffer =
      reinterpret_cast<decltype(this->av_frame_get_buffer)>(
          dlsym(library_, "av_frame_get_buffer"));
#else
  av_frame_get_buffer = &::av_frame_get_buffer;
#endif
  if (!av_frame_get_buffer) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_frame_unref =
      reinterpret_cast<decltype(this->av_frame_unref)>(
          dlsym(library_, "av_frame_unref"));
#else
  av_frame_unref = &::av_frame_unref;
#endif
  if (!av_frame_unref) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_free =
      reinterpret_cast<decltype(this->av_free)>(
          dlsym(library_, "av_free"));
#else
  av_free = &::av_free;
#endif
  if (!av_free) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_hwdevice_ctx_create =
      reinterpret_cast<decltype(this->av_hwdevice_ctx_create)>(
          dlsym(library_, "av_hwdevice_ctx_create"));
#else
  av_hwdevice_ctx_create = &::av_hwdevice_ctx_create;
#endif
  if (!av_hwdevice_ctx_create) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_hwframe_ctx_alloc =
      reinterpret_cast<decltype(this->av_hwframe_ctx_alloc)>(
          dlsym(library_, "av_hwframe_ctx_alloc"));
#else
  av_hwframe_ctx_alloc = &::av_hwframe_ctx_alloc;
#endif
  if (!av_hwframe_ctx_alloc) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_hwframe_ctx_init =
      reinterpret_cast<decltype(this->av_hwframe_ctx_init)>(
          dlsym(library_, "av_hwframe_ctx_init"));
#else
  av_hwframe_ctx_init = &::av_hwframe_ctx_init;
#endif
  if (!av_hwframe_ctx_init) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_hwframe_get_buffer =
      reinterpret_cast<decltype(this->av_hwframe_get_buffer)>(
          dlsym(library_, "av_hwframe_get_buffer"));
#else
  av_hwframe_get_buffer = &::av_hwframe_get_buffer;
#endif
  if (!av_hwframe_get_buffer) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_hwframe_map =
      reinterpret_cast<decltype(this->av_hwframe_map)>(
          dlsym(library_, "av_hwframe_map"));
#else
  av_hwframe_map = &::av_hwframe_map;
#endif
  if (!av_hwframe_map) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_hwframe_transfer_data =
      reinterpret_cast<decltype(this->av_hwframe_transfer_data)>(
          dlsym(library_, "av_hwframe_transfer_data"));
#else
  av_hwframe_transfer_data = &::av_hwframe_transfer_data;
#endif
  if (!av_hwframe_transfer_data) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_log_set_callback =
      reinterpret_cast<decltype(this->av_log_set_callback)>(
          dlsym(library_, "av_log_set_callback"));
#else
  av_log_set_callback = &::av_log_set_callback;
#endif
  if (!av_log_set_callback) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_log_set_level =
      reinterpret_cast<decltype(this->av_log_set_level)>(
          dlsym(library_, "av_log_set_level"));
#else
  av_log_set_level = &::av_log_set_level;
#endif
  if (!av_log_set_level) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_opt_set =
      reinterpret_cast<decltype(this->av_opt_set)>(
          dlsym(library_, "av_opt_set"));
#else
  av_opt_set = &::av_opt_set;
#endif
  if (!av_opt_set) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_strdup =
      reinterpret_cast<decltype(this->av_strdup)>(
          dlsym(library_, "av_strdup"));
#else
  av_strdup = &::av_strdup;
#endif
  if (!av_strdup) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_strerror =
      reinterpret_cast<decltype(this->av_strerror)>(
          dlsym(library_, "av_strerror"));
#else
  av_strerror = &::av_strerror;
#endif
  if (!av_strerror) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_vkfmt_from_pixfmt =
      reinterpret_cast<decltype(this->av_vkfmt_from_pixfmt)>(
          dlsym(library_, "av_vkfmt_from_pixfmt"));
#else
  av_vkfmt_from_pixfmt = &::av_vkfmt_from_pixfmt;
#endif
  if (!av_vkfmt_from_pixfmt) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  av_vk_frame_alloc =
      reinterpret_cast<decltype(this->av_vk_frame_alloc)>(
          dlsym(library_, "av_vk_frame_alloc"));
#else
  av_vk_frame_alloc = &::av_vk_frame_alloc;
#endif
  if (!av_vk_frame_alloc) {
    CleanUp(true);
    return false;
  }


  loaded_ = true;
  return true;
}

void avutil::CleanUp(bool unload) {
#if defined(LIBRARY_LOADER_AVUTIL_LOADER_H_DLOPEN)
  if (unload) {
    dlclose(library_);
    library_ = NULL;
  }
#else
  (void)unload;
#endif
  loaded_ = false;
  av_buffer_alloc = NULL;
  av_buffer_ref = NULL;
  av_buffer_unref = NULL;
  av_dict_set = NULL;
  av_frame_alloc = NULL;
  av_frame_free = NULL;
  av_frame_get_buffer = NULL;
  av_frame_unref = NULL;
  av_free = NULL;
  av_hwdevice_ctx_create = NULL;
  av_hwframe_ctx_alloc = NULL;
  av_hwframe_ctx_init = NULL;
  av_hwframe_get_buffer = NULL;
  av_hwframe_map = NULL;
  av_hwframe_transfer_data = NULL;
  av_log_set_callback = NULL;
  av_log_set_level = NULL;
  av_opt_set = NULL;
  av_strdup = NULL;
  av_strerror = NULL;
  av_vkfmt_from_pixfmt = NULL;
  av_vk_frame_alloc = NULL;

}
