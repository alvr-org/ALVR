// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#ifndef LIBRARY_LOADER_AVCODEC_LOADER_H
#define LIBRARY_LOADER_AVCODEC_LOADER_H

extern "C" {
#include <libavcodec/avcodec.h>

}


#include <string>

class avcodec {
 public:
  avcodec();
  ~avcodec();

  bool Load(const std::string& library_name)
      __attribute__((warn_unused_result));

  bool loaded() const { return loaded_; }

  decltype(&::avcodec_alloc_context3) avcodec_alloc_context3;
  decltype(&::avcodec_find_encoder_by_name) avcodec_find_encoder_by_name;
  decltype(&::avcodec_free_context) avcodec_free_context;
  decltype(&::avcodec_open2) avcodec_open2;
  decltype(&::avcodec_receive_packet) avcodec_receive_packet;
  decltype(&::avcodec_send_frame) avcodec_send_frame;
  decltype(&::av_packet_alloc) av_packet_alloc;
  decltype(&::av_packet_free) av_packet_free;


 private:
  void CleanUp(bool unload);

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  void* library_;
#endif

  bool loaded_;

  // Disallow copy constructor and assignment operator.
  avcodec(const avcodec&);
  void operator=(const avcodec&);
};

#endif  // LIBRARY_LOADER_AVCODEC_LOADER_H
