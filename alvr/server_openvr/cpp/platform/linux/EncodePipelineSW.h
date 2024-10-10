#pragma once

#include "EncodePipeline.h"

#include <wels/codec_api.h>

class FormatConverter;

namespace alvr {

class EncodePipelineSW : public EncodePipeline {
public:
    ~EncodePipelineSW();
    EncodePipelineSW(Renderer* render, uint32_t width, uint32_t height);

    void PushFrame(uint64_t targetTimestampNs, bool idr) override;
    bool GetEncoded(FramePacket& packet) override;
    void SetParams(FfiDynamicEncoderParams params) override;
    int GetCodec() override;

private:
    ISVCEncoder* encoder_ = nullptr;
    uint8_t* buf = nullptr;
    std::vector<uint8_t>* buf_out = nullptr;
    SFrameBSInfo info;
    SSourcePicture pic;

    int64_t pts = 0;
    bool is_idr = false;
    FormatConverter* rgbtoyuv = nullptr;
};
}
