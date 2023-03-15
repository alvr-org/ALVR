#include "ffr.h"

#include <cmath>
#include <memory>

#include "utils.h"

using namespace std;
using namespace gl_render_utils;

namespace {
const string FFR_COMMON_SHADER_FORMAT = R"glsl(#version 300 es
        precision highp float;

        const uvec2 TARGET_RESOLUTION = uvec2(%u, %u);
        const uvec2 OPTIMIZED_RESOLUTION = uvec2(%u, %u);
        const vec2 EYE_SIZE_RATIO = vec2(%f, %f);
        const vec2 CENTER_SIZE = vec2(%f, %f);
        const vec2 CENTER_SHIFT = vec2(%f, %f);
        const vec2 EDGE_RATIO = vec2(%f, %f);

        vec2 TextureToEyeUV(vec2 textureUV, bool isRightEye) {
            // flip distortion horizontally for right eye
            // left: x * 2; right: (1 - x) * 2
            return vec2((textureUV.x + float(isRightEye) * (1. - 2. * textureUV.x)) * 2., textureUV.y);
        }

        vec2 EyeToTextureUV(vec2 eyeUV, bool isRightEye) {
            // left: x / 2; right 1 - (x / 2)
            return vec2(eyeUV.x * 0.5 + float(isRightEye) * (1. - eyeUV.x), eyeUV.y);
        }
    )glsl";

const string DECOMPRESS_AXIS_ALIGNED_FRAGMENT_SHADER = R"glsl(
        uniform sampler2D tex0;
        in vec2 uv;
        out vec4 color;
        void main() {
            bool isRightEye = uv.x > 0.5;
            vec2 eyeUV = TextureToEyeUV(uv, isRightEye);

            vec2 c0 = (1. - CENTER_SIZE) * 0.5;
            vec2 c1 = (EDGE_RATIO - 1.) * c0 * (CENTER_SHIFT + 1.) / EDGE_RATIO;
            vec2 c2 = (EDGE_RATIO - 1.) * CENTER_SIZE + 1.;

            vec2 loBound = c0 * (CENTER_SHIFT + 1.);
            vec2 hiBound = c0 * (CENTER_SHIFT - 1.) + 1.;
            vec2 underBound = vec2(eyeUV.x < loBound.x, eyeUV.y < loBound.y);
            vec2 inBound = vec2(loBound.x < eyeUV.x && eyeUV.x < hiBound.x,
                                loBound.y < eyeUV.y && eyeUV.y < hiBound.y);
            vec2 overBound = vec2(eyeUV.x > hiBound.x, eyeUV.y > hiBound.y);

            vec2 center = (eyeUV - c1) * EDGE_RATIO / c2;

            vec2 loBoundC = c0 * (CENTER_SHIFT + 1.) / c2;
            vec2 hiBoundC = c0 * (CENTER_SHIFT - 1.) / c2 + 1.;

            vec2 leftEdge = (-(c1 + c2 * loBoundC) / loBoundC +
                            sqrt(((c1 + c2 * loBoundC) / loBoundC) * ((c1 + c2 * loBoundC) / loBoundC) +
                                4. * c2 * (1. - EDGE_RATIO) / (EDGE_RATIO * loBoundC) * eyeUV)) /
                            (2. * c2 * (1. - EDGE_RATIO)) * (EDGE_RATIO * loBoundC);
            vec2 rightEdge =
                (-(c2 - EDGE_RATIO * c1 - 2. * EDGE_RATIO * c2 + c2 * EDGE_RATIO * (1. - hiBoundC) +
                EDGE_RATIO) /
                    (EDGE_RATIO * (1. - hiBoundC)) +
                sqrt(((c2 - EDGE_RATIO * c1 - 2. * EDGE_RATIO * c2 + c2 * EDGE_RATIO * (1. - hiBoundC) +
                        EDGE_RATIO) /
                    (EDGE_RATIO * (1. - hiBoundC))) *
                        ((c2 - EDGE_RATIO * c1 - 2. * EDGE_RATIO * c2 +
                            c2 * EDGE_RATIO * (1. - hiBoundC) + EDGE_RATIO) /
                        (EDGE_RATIO * (1. - hiBoundC))) -
                    4. * ((c2 * EDGE_RATIO - c2) * (c1 - hiBoundC + hiBoundC * c2) /
                                (EDGE_RATIO * (1. - hiBoundC) * (1. - hiBoundC)) -
                            eyeUV * (c2 * EDGE_RATIO - c2) / (EDGE_RATIO * (1. - hiBoundC))))) /
                (2. * c2 * (EDGE_RATIO - 1.)) * (EDGE_RATIO * (1. - hiBoundC));

            vec2 uncompressedUV = underBound * leftEdge + inBound * center + overBound * rightEdge;

            color = texture(tex0, EyeToTextureUV(uncompressedUV * EYE_SIZE_RATIO, isRightEye));
        }
    )glsl";
} // namespace

FoveationVars CalculateFoveationVars(FFRData data) {
    float targetEyeWidth = data.viewWidth;
    float targetEyeHeight = data.viewHeight;

    float centerSizeX = data.centerSizeX;
    float centerSizeY = data.centerSizeY;
    float centerShiftX = data.centerShiftX;
    float centerShiftY = data.centerShiftY;
    float edgeRatioX = data.edgeRatioX;
    float edgeRatioY = data.edgeRatioY;

    float edgeSizeX = targetEyeWidth - centerSizeX * targetEyeWidth;
    float edgeSizeY = targetEyeHeight - centerSizeY * targetEyeHeight;

    float centerSizeXAligned =
        1. - ceil(edgeSizeX / (edgeRatioX * 2.)) * (edgeRatioX * 2.) / targetEyeWidth;
    float centerSizeYAligned =
        1. - ceil(edgeSizeY / (edgeRatioY * 2.)) * (edgeRatioY * 2.) / targetEyeHeight;

    float edgeSizeXAligned = targetEyeWidth - centerSizeXAligned * targetEyeWidth;
    float edgeSizeYAligned = targetEyeHeight - centerSizeYAligned * targetEyeHeight;

    float centerShiftXAligned = ceil(centerShiftX * edgeSizeXAligned / (edgeRatioX * 2.)) *
                                (edgeRatioX * 2.) / edgeSizeXAligned;
    float centerShiftYAligned = ceil(centerShiftY * edgeSizeYAligned / (edgeRatioY * 2.)) *
                                (edgeRatioY * 2.) / edgeSizeYAligned;

    float foveationScaleX = (centerSizeXAligned + (1. - centerSizeXAligned) / edgeRatioX);
    float foveationScaleY = (centerSizeYAligned + (1. - centerSizeYAligned) / edgeRatioY);

    float optimizedEyeWidth = foveationScaleX * targetEyeWidth;
    float optimizedEyeHeight = foveationScaleY * targetEyeHeight;

    // round the frame dimensions to a number of pixel multiple of 32 for the encoder
    auto optimizedEyeWidthAligned = (uint32_t)ceil(optimizedEyeWidth / 32.f) * 32;
    auto optimizedEyeHeightAligned = (uint32_t)ceil(optimizedEyeHeight / 32.f) * 32;

    float eyeWidthRatioAligned = optimizedEyeWidth / optimizedEyeWidthAligned;
    float eyeHeightRatioAligned = optimizedEyeHeight / optimizedEyeHeightAligned;

    return {data.viewWidth,
            data.viewHeight,
            optimizedEyeWidthAligned,
            optimizedEyeHeightAligned,
            eyeWidthRatioAligned,
            eyeHeightRatioAligned,
            centerSizeXAligned,
            centerSizeYAligned,
            centerShiftXAligned,
            centerShiftYAligned,
            edgeRatioX,
            edgeRatioY};
}

FFR::FFR(Texture *inputSurface) : mInputSurface(inputSurface) {}

void FFR::Initialize(FoveationVars fv) {
    auto ffrCommonShaderStr = string_format(FFR_COMMON_SHADER_FORMAT,
                                            fv.targetEyeWidth,
                                            fv.targetEyeHeight,
                                            fv.optimizedEyeWidth,
                                            fv.optimizedEyeHeight,
                                            fv.eyeWidthRatio,
                                            fv.eyeHeightRatio,
                                            fv.centerSizeX,
                                            fv.centerSizeY,
                                            fv.centerShiftX,
                                            fv.centerShiftY,
                                            fv.edgeRatioX,
                                            fv.edgeRatioY);

    mExpandedTexture.reset(new Texture(false, 0, false, fv.targetEyeWidth * 2, fv.targetEyeHeight));
    mExpandedTextureState = make_unique<RenderState>(mExpandedTexture.get());

    auto decompressAxisAlignedShaderStr =
        ffrCommonShaderStr + DECOMPRESS_AXIS_ALIGNED_FRAGMENT_SHADER;
    mDecompressAxisAlignedPipeline = unique_ptr<RenderPipeline>(
        new RenderPipeline({mInputSurface}, QUAD_2D_VERTEX_SHADER, decompressAxisAlignedShaderStr));
}

void FFR::Render() const {
    mExpandedTextureState->ClearDepth();
    mDecompressAxisAlignedPipeline->Render(*mExpandedTextureState);
}
