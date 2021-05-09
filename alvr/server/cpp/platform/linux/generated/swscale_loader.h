// This is generated file. Do not modify directly.
// Path to the code generator: alvr/server/generate_library_loader.py .

#ifndef LIBRARY_LOADER_SWSCALE_LOADER_H
#define LIBRARY_LOADER_SWSCALE_LOADER_H

extern "C" {
#include <libswscale/swscale.h>

}


#include <string>

class swscale {
 public:
  swscale();
  ~swscale();

  bool Load(const std::string& library_name)
      __attribute__((warn_unused_result));

  bool loaded() const { return loaded_; }

  decltype(&::sws_getContext) sws_getContext;
  decltype(&::sws_scale) sws_scale;


 private:
  void CleanUp(bool unload);

#if defined(LIBRARY_LOADER_SWSCALE_LOADER_H_DLOPEN)
  void* library_;
#endif

  bool loaded_;

  // Disallow copy constructor and assignment operator.
  swscale(const swscale&);
  void operator=(const swscale&);
};

#endif  // LIBRARY_LOADER_SWSCALE_LOADER_H
