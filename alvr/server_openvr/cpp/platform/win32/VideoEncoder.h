#pragma once

#include "NvEncoderD3D11.h"
#include "shared/d3drender.h"
#include <functional>
#include <memory>

class VideoEncoder {
public:
    virtual void Initialize() = 0;
    virtual void Shutdown() = 0;

    virtual void Transmit(
        ID3D11Texture2D* pTexture,
        uint64_t presentationTime,
        uint64_t targetTimestampNs,
        bool insertIDR
    ) = 0;

    /// Marks this instance as encoding the monochrome alpha companion stream, so its output is
    /// routed to the alpha NAL path and its bitrate comes from the fixed alpha setting instead of
    /// the dynamic bitrate manager (which tracks the color stream only).
    void SetAlphaStream(bool isAlpha) { m_isAlphaStream = isAlpha; }
    bool IsAlphaStream() const { return m_isAlphaStream; }

protected:
    bool m_isAlphaStream = false;
};
