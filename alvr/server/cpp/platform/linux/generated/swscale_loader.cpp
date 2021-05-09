// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#include "swscale_loader.h"

#include <dlfcn.h>

swscale::swscale() : loaded_(false) {
}

swscale::~swscale() {
  CleanUp(loaded_);
}

bool swscale::Load(const std::string& library_name) {
  if (loaded_)
    return false;

#if defined(LIBRARY_LOADER_SWSCALE_LOADER_H_DLOPEN)
  library_ = dlopen(library_name.c_str(), RTLD_LAZY);
  if (!library_)
    return false;
#else
  (void)library_name;
#endif


#if defined(LIBRARY_LOADER_SWSCALE_LOADER_H_DLOPEN)
  sws_getContext =
      reinterpret_cast<decltype(this->sws_getContext)>(
          dlsym(library_, "sws_getContext"));
#else
  sws_getContext = &::sws_getContext;
#endif
  if (!sws_getContext) {
    CleanUp(true);
    return false;
  }

#if defined(LIBRARY_LOADER_SWSCALE_LOADER_H_DLOPEN)
  sws_scale =
      reinterpret_cast<decltype(this->sws_scale)>(
          dlsym(library_, "sws_scale"));
#else
  sws_scale = &::sws_scale;
#endif
  if (!sws_scale) {
    CleanUp(true);
    return false;
  }


  loaded_ = true;
  return true;
}

void swscale::CleanUp(bool unload) {
#if defined(LIBRARY_LOADER_SWSCALE_LOADER_H_DLOPEN)
  if (unload) {
    dlclose(library_);
    library_ = NULL;
  }
#else
  (void)unload;
#endif
  loaded_ = false;
  sws_getContext = NULL;
  sws_scale = NULL;

}
