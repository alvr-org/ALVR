// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#ifndef LIBRARY_LOADER_AVFILTER_LOADER_H
#define LIBRARY_LOADER_AVFILTER_LOADER_H

extern "C" {
#include <stdint.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavfilter/avfilter.h>

}


#include <string>

class avfilter {
 public:
  avfilter();
  ~avfilter();

  bool Load(const std::string& library_name)
      __attribute__((warn_unused_result));

  bool loaded() const { return loaded_; }

  decltype(&::av_buffersink_get_frame) av_buffersink_get_frame;
  decltype(&::av_buffersrc_add_frame_flags) av_buffersrc_add_frame_flags;
  decltype(&::av_buffersrc_parameters_alloc) av_buffersrc_parameters_alloc;
  decltype(&::av_buffersrc_parameters_set) av_buffersrc_parameters_set;
  decltype(&::avfilter_get_by_name) avfilter_get_by_name;
  decltype(&::avfilter_graph_alloc) avfilter_graph_alloc;
  decltype(&::avfilter_graph_alloc_filter) avfilter_graph_alloc_filter;
  decltype(&::avfilter_graph_config) avfilter_graph_config;
  decltype(&::avfilter_graph_create_filter) avfilter_graph_create_filter;
  decltype(&::avfilter_graph_free) avfilter_graph_free;
  decltype(&::avfilter_graph_parse_ptr) avfilter_graph_parse_ptr;
  decltype(&::avfilter_inout_alloc) avfilter_inout_alloc;
  decltype(&::avfilter_inout_free) avfilter_inout_free;


 private:
  void CleanUp(bool unload);

#if defined(LIBRARY_LOADER_AVFILTER_LOADER_H_DLOPEN)
  void* library_;
#endif

  bool loaded_;

  // Disallow copy constructor and assignment operator.
  avfilter(const avfilter&);
  void operator=(const avfilter&);
};

#endif  // LIBRARY_LOADER_AVFILTER_LOADER_H
