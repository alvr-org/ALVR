// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#include "avcodec_loader.h"

#include <dlfcn.h>

avcodec::avcodec() : loaded_(false) {
}

avcodec::~avcodec() {
  CleanUp(loaded_);
}

bool avcodec::Load(const std::string& library_name) {
  if (loaded_)
    return false;

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  library_ = dlopen(library_name.c_str(), RTLD_LAZY);
  if (!library_)
    return false;
#else
  (void)library_name;
#endif


#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  avcodec_alloc_context3 =
      reinterpret_cast<decltype(this->avcodec_alloc_context3)>(
          dlsym(library_, "avcodec_alloc_context3"));
#else
  avcodec_alloc_context3 = &::avcodec_alloc_context3;
#endif
  if (!avcodec_alloc_context3) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  avcodec_find_encoder_by_name =
      reinterpret_cast<decltype(this->avcodec_find_encoder_by_name)>(
          dlsym(library_, "avcodec_find_encoder_by_name"));
#else
  avcodec_find_encoder_by_name = &::avcodec_find_encoder_by_name;
#endif
  if (!avcodec_find_encoder_by_name) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  avcodec_free_context =
      reinterpret_cast<decltype(this->avcodec_free_context)>(
          dlsym(library_, "avcodec_free_context"));
#else
  avcodec_free_context = &::avcodec_free_context;
#endif
  if (!avcodec_free_context) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  avcodec_open2 =
      reinterpret_cast<decltype(this->avcodec_open2)>(
          dlsym(library_, "avcodec_open2"));
#else
  avcodec_open2 = &::avcodec_open2;
#endif
  if (!avcodec_open2) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  avcodec_receive_packet =
      reinterpret_cast<decltype(this->avcodec_receive_packet)>(
          dlsym(library_, "avcodec_receive_packet"));
#else
  avcodec_receive_packet = &::avcodec_receive_packet;
#endif
  if (!avcodec_receive_packet) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  avcodec_send_frame =
      reinterpret_cast<decltype(this->avcodec_send_frame)>(
          dlsym(library_, "avcodec_send_frame"));
#else
  avcodec_send_frame = &::avcodec_send_frame;
#endif
  if (!avcodec_send_frame) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  av_packet_alloc =
      reinterpret_cast<decltype(this->av_packet_alloc)>(
          dlsym(library_, "av_packet_alloc"));
#else
  av_packet_alloc = &::av_packet_alloc;
#endif
  if (!av_packet_alloc) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  av_packet_free =
      reinterpret_cast<decltype(this->av_packet_free)>(
          dlsym(library_, "av_packet_free"));
#else
  av_packet_free = &::av_packet_free;
#endif
  if (!av_packet_free) {
    CleanUp(true);
    return false;
  }


  loaded_ = true;
  return true;
}

void avcodec::CleanUp(bool unload) {
#if defined(LIBRARY_LOADER_AVCODEC_LOADER_H_DLOPEN)
  if (unload) {
    dlclose(library_);
    library_ = NULL;
  }
#else
  (void)unload;
#endif
  loaded_ = false;
  avcodec_alloc_context3 = NULL;
  avcodec_find_encoder_by_name = NULL;
  avcodec_free_context = NULL;
  avcodec_open2 = NULL;
  avcodec_receive_packet = NULL;
  avcodec_send_frame = NULL;
  av_packet_alloc = NULL;
  av_packet_free = NULL;

}
