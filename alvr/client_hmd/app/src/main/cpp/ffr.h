#pragma once

#include <memory>
#include <vector>

#include "gl_render_utils/render_pipeline.h"
#include "packet_types.h"


enum FOVEATION_MODE {
    FOVEATION_MODE_DISABLED = 0,
    FOVEATION_MODE_SLICES = 1,
    FOVEATION_MODE_WARP = 2,
};

struct FFRData {
    FOVEATION_MODE mode;
    uint32_t eyeWidth;
    uint32_t eyeHeight;
    EyeFov leftEyeFov;
    float foveationStrength;
    float foveationShape;
    float foveationVerticalOffset;
};

class FFR {
public:
    FFR(gl_render_utils::Texture *inputSurface);

    void Initialize(FFRData ffrData);

    void Render();

    gl_render_utils::Texture *GetOutputTexture() { return mExpandedTexture.get(); }

private:

    gl_render_utils::Texture *mInputSurface;
    std::unique_ptr<gl_render_utils::Texture> mExpandedTexture;

    std::vector<std::unique_ptr<gl_render_utils::RenderPipeline>> mPipelines;
};
