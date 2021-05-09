// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#include "avfilter_loader.h"

#include <dlfcn.h>

avfilter::avfilter() : loaded_(false) {
}

avfilter::~avfilter() {
  CleanUp(loaded_);
}

bool avfilter::Load(const std::string& library_name) {
  if (loaded_)
    return false;

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  library_ = dlopen(library_name.c_str(), RTLD_LAZY);
  if (!library_)
    return false;
#else
  (void)library_name;
#endif


#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  av_buffersink_get_frame =
      reinterpret_cast<decltype(this->av_buffersink_get_frame)>(
          dlsym(library_, "av_buffersink_get_frame"));
#else
  av_buffersink_get_frame = &::av_buffersink_get_frame;
#endif
  if (!av_buffersink_get_frame) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  av_buffersrc_add_frame_flags =
      reinterpret_cast<decltype(this->av_buffersrc_add_frame_flags)>(
          dlsym(library_, "av_buffersrc_add_frame_flags"));
#else
  av_buffersrc_add_frame_flags = &::av_buffersrc_add_frame_flags;
#endif
  if (!av_buffersrc_add_frame_flags) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  av_buffersrc_parameters_alloc =
      reinterpret_cast<decltype(this->av_buffersrc_parameters_alloc)>(
          dlsym(library_, "av_buffersrc_parameters_alloc"));
#else
  av_buffersrc_parameters_alloc = &::av_buffersrc_parameters_alloc;
#endif
  if (!av_buffersrc_parameters_alloc) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  av_buffersrc_parameters_set =
      reinterpret_cast<decltype(this->av_buffersrc_parameters_set)>(
          dlsym(library_, "av_buffersrc_parameters_set"));
#else
  av_buffersrc_parameters_set = &::av_buffersrc_parameters_set;
#endif
  if (!av_buffersrc_parameters_set) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_get_by_name =
      reinterpret_cast<decltype(this->avfilter_get_by_name)>(
          dlsym(library_, "avfilter_get_by_name"));
#else
  avfilter_get_by_name = &::avfilter_get_by_name;
#endif
  if (!avfilter_get_by_name) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_graph_alloc =
      reinterpret_cast<decltype(this->avfilter_graph_alloc)>(
          dlsym(library_, "avfilter_graph_alloc"));
#else
  avfilter_graph_alloc = &::avfilter_graph_alloc;
#endif
  if (!avfilter_graph_alloc) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_graph_alloc_filter =
      reinterpret_cast<decltype(this->avfilter_graph_alloc_filter)>(
          dlsym(library_, "avfilter_graph_alloc_filter"));
#else
  avfilter_graph_alloc_filter = &::avfilter_graph_alloc_filter;
#endif
  if (!avfilter_graph_alloc_filter) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_graph_config =
      reinterpret_cast<decltype(this->avfilter_graph_config)>(
          dlsym(library_, "avfilter_graph_config"));
#else
  avfilter_graph_config = &::avfilter_graph_config;
#endif
  if (!avfilter_graph_config) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_graph_create_filter =
      reinterpret_cast<decltype(this->avfilter_graph_create_filter)>(
          dlsym(library_, "avfilter_graph_create_filter"));
#else
  avfilter_graph_create_filter = &::avfilter_graph_create_filter;
#endif
  if (!avfilter_graph_create_filter) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_graph_free =
      reinterpret_cast<decltype(this->avfilter_graph_free)>(
          dlsym(library_, "avfilter_graph_free"));
#else
  avfilter_graph_free = &::avfilter_graph_free;
#endif
  if (!avfilter_graph_free) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_graph_parse_ptr =
      reinterpret_cast<decltype(this->avfilter_graph_parse_ptr)>(
          dlsym(library_, "avfilter_graph_parse_ptr"));
#else
  avfilter_graph_parse_ptr = &::avfilter_graph_parse_ptr;
#endif
  if (!avfilter_graph_parse_ptr) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_inout_alloc =
      reinterpret_cast<decltype(this->avfilter_inout_alloc)>(
          dlsym(library_, "avfilter_inout_alloc"));
#else
  avfilter_inout_alloc = &::avfilter_inout_alloc;
#endif
  if (!avfilter_inout_alloc) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  avfilter_inout_free =
      reinterpret_cast<decltype(this->avfilter_inout_free)>(
          dlsym(library_, "avfilter_inout_free"));
#else
  avfilter_inout_free = &::avfilter_inout_free;
#endif
  if (!avfilter_inout_free) {
    CleanUp(true);
    return false;
  }


  loaded_ = true;
  return true;
}

void avfilter::CleanUp(bool unload) {
#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  if (unload) {
    dlclose(library_);
    library_ = NULL;
  }
#else
  (void)unload;
#endif
  loaded_ = false;
  av_buffersink_get_frame = NULL;
  av_buffersrc_add_frame_flags = NULL;
  av_buffersrc_parameters_alloc = NULL;
  av_buffersrc_parameters_set = NULL;
  avfilter_get_by_name = NULL;
  avfilter_graph_alloc = NULL;
  avfilter_graph_alloc_filter = NULL;
  avfilter_graph_config = NULL;
  avfilter_graph_create_filter = NULL;
  avfilter_graph_free = NULL;
  avfilter_graph_parse_ptr = NULL;
  avfilter_inout_alloc = NULL;
  avfilter_inout_free = NULL;

}
