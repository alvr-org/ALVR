#pragma once

#include "Renderer.h"
#include "ffmpeg_helper.h"
#include "protocol.h"

class FrameRender : public Renderer
{
public:
    explicit FrameRender(alvr::VkContext &ctx, init_packet &init, int fds[]);
    ~FrameRender();

    Output CreateOutput();
    uint32_t GetEncodingWidth() const;
    uint32_t GetEncodingHeight() const;

private:
    struct ColorCorrection {
        float renderWidth;
        float renderHeight;
        float brightness;
        float contrast;
        float saturation;
        float gamma;
        float sharpening;
        float _align;
    };

    struct FoveationVars {
        int32_t targetEyeWidth;
        int32_t targetEyeHeight;
        int32_t optimizedEyeWidth;
        int32_t optimizedEyeHeight;

        float eyeWidthRatio;
        float eyeHeightRatio;

        float centerSizeX;
        float centerSizeY;
        float centerShiftX;
        float centerShiftY;
        float edgeRatioX;
        float edgeRatioY;
    };

    void setupColorCorrection();
    void setupFoveatedRendering();
    void setupCustomShaders(const std::string &stage);

    uint32_t m_width;
    uint32_t m_height;
    ColorCorrection m_colorCorrectionPushConstants;
    FoveationVars m_foveatedRenderingPushConstants;
    std::vector<RenderPipeline*> m_pipelines;
};
