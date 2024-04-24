#include "ffr.h"

#include <cmath>
#include <memory>

#include "utils.h"

#include <glm/vec2.hpp>

using namespace std;
using namespace gl_render_utils;

namespace {
const string FFR_COMMON_SHADER_FORMAT = R"glsl(#version 300 es
        precision highp float;

        const vec2 EYE_SIZE_RATIO = vec2(%f, %f);
        const vec2 EDGE_RATIO = vec2(%f, %f);

        const vec2 c1 = vec2(%f, %f);
        const vec2 c2 = vec2(%f, %f);
        const vec2 loBound = vec2(%f, %f);
        const vec2 hiBound = vec2(%f, %f);
        const vec2 loBoundC = vec2(%f, %f);
        const vec2 hiBoundC = vec2(%f, %f);

        const vec2 aleft = vec2(%f, %f);
        const vec2 bleft = vec2(%f, %f);

        const vec2 aright = vec2(%f, %f);
        const vec2 bright = vec2(%f, %f);
        const vec2 cright = vec2(%f, %f);

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

// Fragment shader which reverses the FFE compressed image on client side.
// Essentially it implements this function, for both the X and Y axis (or U and V in texture):
//   https://www.desmos.com/calculator/cmvjr7ljje
//
// The function is the same for each axis. The code refers to the "left" and "right" edges, but
// that kinda only refers to the left/right sides of the function. The actual edge being calculated
// in the image might not be the left and right ones (might be top and bottom instead). Also for
// the right eye the UVs are mirrored so the "left" edge is actually the right one.
const string DECOMPRESS_AXIS_ALIGNED_FRAGMENT_SHADER = R"glsl(
        uniform sampler2D tex0;
        in vec2 uv;
        out vec4 color;
        void main() {
            // Source UV spans across both eyes, so first transform
            // the UV coordinates to be per-eye.
            bool isRightEye = uv.x > 0.5;
            vec2 eyeUV = TextureToEyeUV(uv, isRightEye);

            // Now calculate the uncompressed UVs for the various regions of the image.
            // There's three regions to consider: the "left", the middle and the "right"
            vec2 center = (eyeUV - c1) * EDGE_RATIO / c2;
            vec2 leftEdge = (-bleft + sqrt(bleft * bleft + 4. * aleft * eyeUV)) /
                            (2. * aleft);
            vec2 rightEdge = (-bright + sqrt(bright * bright - 4. * (cright - aright * eyeUV))) / (2. * aright);

            // Now figure out which UV coordinates to actually output depending on which
            // UV region is being processed. Do each axis separately to cover all the nine
            // possible combinations.
            vec2 uncompressedUV = vec2(0., 0.);

            if (eyeUV.x < loBound.x)
                uncompressedUV.x = leftEdge.x;
            else if (eyeUV.x > hiBound.x)
                uncompressedUV.x = rightEdge.x;
            else
                uncompressedUV.x = center.x;

            if (eyeUV.y < loBound.y)
                uncompressedUV.y = leftEdge.y;
            else if (eyeUV.y > hiBound.y)
                uncompressedUV.y = rightEdge.y;
            else
                uncompressedUV.y = center.y;

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
    using glm::vec2;

    // Precalculate a bunch of constants that will be used in fragment shader
    auto CENTER_SIZE = vec2(fv.centerSizeX, fv.centerSizeY); // Size of the center, non-distorted region
    auto CENTER_SHIFT = vec2(fv.centerShiftX, fv.centerShiftY); // How much to shift the center region
    auto EDGE_RATIO = vec2(fv.edgeRatioX, fv.edgeRatioY); // Ratio of edge region VS center region

    auto c0 = (vec2(1., 1.) - CENTER_SIZE) * vec2(0.5, 0.5);
    auto c1 = (EDGE_RATIO - vec2(1., 1.)) * c0 * (CENTER_SHIFT + vec2(1., 1.)) / EDGE_RATIO;
    auto c2 = (EDGE_RATIO - vec2(1., 1.)) * CENTER_SIZE + vec2(1., 1.);

    auto loBound = c0 * (CENTER_SHIFT + vec2(1., 1.)); // Lower bound bellow which "left" edge begins
    auto hiBound = c0 * (CENTER_SHIFT - vec2(1., 1.)) + vec2(1., 1.); // Upper bound above which "right" edge begins
    auto loBoundC = c0 * (CENTER_SHIFT + vec2(1., 1.)) / c2; // Same as loBound but rescaled for distorted image
    auto hiBoundC = c0 * (CENTER_SHIFT - vec2(1., 1.)) / c2 + vec2(1., 1.);  // Same as hiBound but rescaled for distorted image

    // Constants for function:
    //   leftEdge(x) = (-bleft + sqrt(bleft^2 + 4 * aleft * x)) / (2 * aleft)
    auto aleft = c2 * (vec2(1., 1.) - EDGE_RATIO) / (EDGE_RATIO * loBoundC);
    auto bleft = (c1 + c2 * loBoundC) / loBoundC;

    // Constants for function:
    //   rightEdge(x) = (-bright + sqrt(bright^2 + 4 * (cright - aright * x)) / (2 * aright)
    auto aright = c2 * (EDGE_RATIO - vec2(1., 1.)) / (EDGE_RATIO * (vec2(1., 1.) - hiBoundC));
    auto bright = (c2 - EDGE_RATIO * c1 - vec2(2., 2.) * EDGE_RATIO * c2 + c2 * EDGE_RATIO * (vec2(1., 1.) - hiBoundC) + EDGE_RATIO) / (EDGE_RATIO * (vec2(1., 1.) - hiBoundC));
    auto cright = ((c2 * EDGE_RATIO - c2) * (c1 - hiBoundC + c2 * hiBoundC)) / (EDGE_RATIO * (vec2(1., 1.) - hiBoundC) * (vec2(1., 1.) - hiBoundC));

    // Put all the constants into the shader
    auto ffrCommonShaderStr = string_format(FFR_COMMON_SHADER_FORMAT,
                                            fv.eyeWidthRatio, fv.eyeHeightRatio,
                                            fv.edgeRatioX, fv.edgeRatioY,
                                            c1.x, c1.y,
                                            c2.x, c2.y,
                                            loBound.x, loBound.y,
                                            hiBound.x, hiBound.y,
                                            loBoundC.x, loBoundC.y,
                                            hiBoundC.x, hiBoundC.y,
                                            aleft.x, aleft.y,
                                            bleft.x, bleft.y,
                                            aright.x, aright.y,
                                            bright.x, bright.y,
                                            cright.x, cright.y
                                            );

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
