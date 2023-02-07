#pragma once

#include "NvEncoderD3D11.h"
#include "shared/d3drender.h"
#include <functional>
#include <memory>

class VideoEncoder {
  public:
    virtual void Initialize() = 0;
    virtual void Shutdown() = 0;

    virtual void Transmit(ID3D11Texture2D *pTexture,
                          uint64_t presentationTime,
                          uint64_t targetTimestampNs,
                          bool insertIDR) = 0;
};
