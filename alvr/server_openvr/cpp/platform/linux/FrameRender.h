#pragma once

#include "Renderer.h"
#include "ffmpeg_helper.h"
#include "protocol.h"

class FrameRender : public Renderer {
public:
    explicit FrameRender(alvr::VkContext& ctx, init_packet& init, int fds[]);
    ~FrameRender();

    Output CreateOutput();
    uint32_t GetEncodingWidth() const;
    uint32_t GetEncodingHeight() const;
    void Render(uint32_t index, uint64_t waitValue, uint64_t targetTimestampNs);

private:
    struct ColorCorrection {
        float renderWidth;
        float renderHeight;
        float brightness;
        float contrast;
        float saturation;
        float gamma;
        float sharpening;
    };

    struct FoveationVars {
        float eyeWidthRatio;
        float eyeHeightRatio;
        float centerSizeX;
        float centerSizeY;
        float edgeRatioX;
        float edgeRatioY;
    };

    struct FoveationCenterShifts {
        float leftX;
        float leftY;
        float rightX;
        float rightY;
    };

    void setupColorCorrection();
    void setupFoveatedRendering();
    void setupCustomShaders(const std::string& stage);

    uint32_t m_width;
    uint32_t m_height;
    ExternalHandle m_handle = ExternalHandle::None;
    ColorCorrection m_colorCorrectionConstants;
    FoveationVars m_foveatedRenderingConstants;
    FoveationCenterShifts m_foveationCenterShifts;
    std::vector<RenderPipeline*> m_pipelines;
};
