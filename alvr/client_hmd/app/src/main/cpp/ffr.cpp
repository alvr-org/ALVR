#include "ffr.h"

#include <cmath>
#include <memory>

#include "utils.h"

using namespace gl_render_utils;

namespace {
    const std::string FFR_COMMON_SHADER_FORMAT = R"glsl(
        #version 300 es
        #extension GL_OES_EGL_image_external_essl3 : enable
        precision highp float;

        // https://www.shadertoy.com/view/3l2GRR

        // MIT License
        //
        // Copyright (c) 2019 Riccardo Zaglia
        //
        // Permission is hereby granted, free of charge, to any person obtaining a copy
        // of this software and associated documentation files (the "Software"), to deal
        // in the Software without restriction, including without limitation the rights
        // to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
        // copies of the Software, and to permit persons to whom the Software is
        // furnished to do so, subject to the following conditions:
        //
        // The above copyright notice and this permission notice shall be included in all
        // copies or substantial portions of the Software.
        //
        // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
        // IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
        // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
        // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
        // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
        // OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
        // SOFTWARE.

        const uvec2 TARGET_RESOLUTION = uvec2(%u, %u);
        const uvec2 OPTIMIZED_RESOLUTION = uvec2(%u, %u);
        const vec2 FOCUS_POSITION = vec2(%f, %f);
        const vec2 FOVEATION_SCALE = vec2(%f, %f);
        const vec2 BOUND_START = vec2(%f, %f);
        const vec2 DISTORTED_SIZE = vec2(%f, %f);
        const vec2 RESOLUTION_SCALE = vec2(TARGET_RESOLUTION) / vec2(OPTIMIZED_RESOLUTION);


        //Choose one distortion function:

        // ARCTANGENT: good for fixed foveated rendering
        const float EPS = 0.000001;
        #define INVERSE_DISTORTION_FN(a)   atan(a)
        #define INV_DIST_DERIVATIVE(a)     atanDerivative(a)
        float atanDerivative(float a) {
            return 1. / (a * a + 1.);
        }

        // HYPERBOLIC TANGENT: good compression but the periphery is too squished
        //const float EPS = 0.000001;
        //#define INVERSE_DISTORTION_FN(a)   tanh(a)
        //#define INV_DIST_DERIVATIVE(a)     tanhDerivative(a)
        //float tanhDerivative(float a) {
        //    float tanh_a = tanh(a);
        //    return 1. - tanh_a * tanh_a;
        //}

        // POW: good for tracked foveated rendering
        //const float POWER = 4. * sqrt(FOVEATION_SCALE.x * FOVEATION_SCALE.y);
        //const float EPS = 0.01;
        //#define INVERSE_DISTORTION_FN(a)   pow(a, 1. / POWER)
        //#define INV_DIST_DERIVATIVE(a)     (pow(a, 1. / POWER - 1.) / POWER)

        // Other functions for distortion:
        // https://en.wikipedia.org/wiki/Sigmoid_function


        vec2 InverseRadialDistortion(vec2 xy) {
            vec2 scaledXY = xy * FOVEATION_SCALE;
            float scaledRadius = length(scaledXY);
            return INVERSE_DISTORTION_FN(scaledRadius) * scaledXY / scaledRadius;
        }

        // Inverse radial distortion derivative wrt length(xy)
        vec2 InverseRadialDistortionDerivative(vec2 xy) {
            vec2 scaledXY = xy * FOVEATION_SCALE;
            float scaledRadius = length(scaledXY);
            return (INV_DIST_DERIVATIVE(scaledRadius) * FOVEATION_SCALE) * scaledXY / scaledRadius;
        }

        vec2 Undistort(vec2 uv) {
            return (InverseRadialDistortion(uv - FOCUS_POSITION) - BOUND_START) / DISTORTED_SIZE;
        }

        vec2 UndistortRadialDerivative(vec2 uv) {
            return InverseRadialDistortionDerivative(uv - FOCUS_POSITION) / DISTORTED_SIZE;
        }

        vec2 GetFilteringWeight2D(vec2 uv) {
            float radialExpansion = length(UndistortRadialDerivative(uv));
            vec2 contraction = 1. / (radialExpansion * RESOLUTION_SCALE);

            vec2 modifiedContraction = contraction - 1. / contraction; // -> ?

            return max(modifiedContraction, EPS);
        }

        vec2 TextureToEyeUV(vec2 textureUV, bool isRightEye) {
            // flip distortion horizontally for right eye
            // left: x * 2; right: (1 - x) * 2
            return vec2((textureUV.x + float(isRightEye) * (1. - 2. * textureUV.x)) * 2., textureUV.y);
        }

        vec2 EyeToTextureUV(vec2 eyeUV, bool isRightEye) {
            // left: x / 2; right 1 - (x / 2)
            return vec2(eyeUV.x / 2. + float(isRightEye) * (1. - eyeUV.x), eyeUV.y);
        }
    )glsl";

    const std::string UNDISTORT_FRAGMENT_SHADER = R"glsl(
        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;
        void main() {
            bool isRightEye = uv.x > 0.5;
            vec2 undistortedUV = Undistort(TextureToEyeUV(uv, isRightEye));
            color = texture(tex0, EyeToTextureUV(undistortedUV, isRightEye));
        }
    )glsl";

    const std::string SHARPENING_FRAGMENT_SHADER = R"glsl(
        const float SHARPEN_STRENGTH = 0.5;
        const vec2 SHARPEN_SCALE = SHARPEN_STRENGTH / vec2(TARGET_RESOLUTION);

        uniform sampler2D tex0;
        in vec2 uv;
        out vec4 color;
        void main() {
            //vec2 sharpenWeight = GetFilteringWeight2D(TextureToEyeUV(uv, uv.x > 0.5));
            //vec2 delta = SHARPEN_SCALE * sharpenWeight;

            //vec3 currentColor = texture(tex0, uv).rgb;
            //vec3 leftColor = texture(tex0, uv - vec2(delta.x / 2., 0.)).rgb;
            //vec3 rightColor = texture(tex0, uv + vec2(delta.x / 2., 0.)).rgb;
            //vec3 downColor = texture(tex0, uv - vec2(0., delta.y)).rgb;
            //vec3 upColor = texture(tex0, uv + vec2(0., delta.y)).rgb;

            //vec3 finalColor = 5. * currentColor + -1. * (leftColor + rightColor + downColor + upColor);
            //color = vec4(finalColor, 1.);

            color = texture(tex0, uv);
        }
    )glsl";

    const std::string DECOMPRESS_SLICES_FRAGMENT_SHADER = R"glsl(
        const vec2 PADDING = 1. / vec2(TARGET_RESOLUTION);

        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;
        void main() {
            bool isRightEye = uv.x > 0.5;

            vec2 centeredUV = TextureToEyeUV(uv, isRightEye) - FOCUS_POSITION;

            float underLeftEdge = float(centeredUV.x < -FOVEATION_SCALE.x / 2.);
            float underBottomEdge = float(centeredUV.y < -FOVEATION_SCALE.y / 2.);
            float overRightEdge = float(centeredUV.x > FOVEATION_SCALE.x / 2.);
            float overTopEdge = float(centeredUV.y > FOVEATION_SCALE.y / 2.);

            vec2 shiftedAbsCornerUV = abs(mod(centeredUV, 1.) - 0.5) - 0.5 + FOVEATION_SCALE / 2.;
            float isCorner = float(shiftedAbsCornerUV.x < 0. && shiftedAbsCornerUV.y < 0.);
            float isCenterLeftOrRightmost =  (1. - overTopEdge) * (1. - underBottomEdge) *
                (float(centeredUV.x > -0.5) * underLeftEdge + float(centeredUV.x > +0.5));
            float isCenterBottomOrTopmost = (1. - overRightEdge) * (1. - underLeftEdge) *
                (float(centeredUV.y > -0.5) * underBottomEdge + float(centeredUV.y > +0.5));

            vec2 compressedOffset =
                vec2(underLeftEdge, underBottomEdge) +
                -1. / 2. * vec2(isCenterLeftOrRightmost, isCenterBottomOrTopmost);

            vec2 foveationRescale =
                1. / 2. +
                underLeftEdge + underBottomEdge + overRightEdge + overTopEdge +
                vec2(1. / 2., -1) * isCenterLeftOrRightmost + vec2(-1, 1. / 2.) * isCenterBottomOrTopmost +
                isCorner;

            float uncompressedScale =
                (1. + underLeftEdge) * (1. + underBottomEdge) * (1. + overRightEdge) * (1. + overTopEdge);

            vec2 paddingCount =
                2. +
                vec2(3, 1) * (underLeftEdge + overRightEdge) + vec2(1, 3) * (underBottomEdge + overTopEdge) +
                -2. * vec2(isCenterBottomOrTopmost, isCenterLeftOrRightmost) +
                -isCorner;

            vec2 uncompressedUV = (centeredUV + FOVEATION_SCALE * foveationRescale +
                                   compressedOffset) / uncompressedScale + paddingCount * PADDING;

            color = texture(tex0, EyeToTextureUV(uncompressedUV * RESOLUTION_SCALE, isRightEye));
        }
    )glsl";

    const float DEG_TO_RAD = (float) M_PI / 180;

#define INVERSE_DISTORTION_FN(a) atan(a);
    const float INVERSE_DISTORTION_DERIVATIVE_IN_0 = 1; // d(atan(0))/dx = 1

    float CalcBoundStart(float focusPos, float fovScale) {
        return INVERSE_DISTORTION_FN(-focusPos * fovScale);
    }

    float CalcBoundEnd(float focusPos, float fovScale) {
        return INVERSE_DISTORTION_FN((1.f - focusPos) * fovScale);
    }

    float CalcDistortedDimension(float focusPos, float fovScale) {
        float boundEnd = CalcBoundEnd(focusPos, fovScale);
        float boundStart = CalcBoundStart(focusPos, fovScale);
        return boundEnd - boundStart;
    }

    float CalcOptimalDimensionForWarp(float scale, float distortedDim, float originalDim) {
        float inverseDistortionDerivative = INVERSE_DISTORTION_DERIVATIVE_IN_0 * scale;
        float gradientOnFocus = inverseDistortionDerivative / distortedDim;
        return originalDim / gradientOnFocus;
    }

    float Align4Normalized(float scale, float originalDim) {
        return float(int(scale * originalDim / 4.f) * 4) / originalDim;
    }

    float CalcOptimalDimensionForSlicing(float scale, float originalDim) {
        return (1. + 3. * scale) / 4. * originalDim + 6;
    }

    struct FoveationVars {
        uint32_t targetEyeWidth;
        uint32_t targetEyeHeight;
        uint32_t optimizedEyeWidth;
        uint32_t optimizedEyeHeight;
        float focusPositionX;
        float focusPositionY;
        float foveationScaleX;
        float foveationScaleY;
        float boundStartX;
        float boundStartY;
        float distortedWidth;
        float distortedHeight;
    };

    FoveationVars CalculateFoveationVars(FFRData data) {
        float targetEyeWidth = data.eyeWidth;
        float targetEyeHeight = data.eyeHeight;

        // left and right side screen plane width with unit focal
        float leftHalfWidth = tan(data.leftEyeFov.left * DEG_TO_RAD);
        float rightHalfWidth = tan(data.leftEyeFov.right * DEG_TO_RAD);
        // foveated center X assuming screen plane with unit width
        float focusPositionX = leftHalfWidth / (leftHalfWidth + rightHalfWidth);
        // align focus position to a number of pixel multiple of 4 to avoid blur and artifacts
        if (data.mode == FOVEATION_MODE_SLICES) {
            focusPositionX = Align4Normalized(focusPositionX, targetEyeWidth);
        }

        // NB: swapping top/bottom fov
        float topHalfHeight = tan(data.leftEyeFov.bottom * DEG_TO_RAD);
        float bottomHalfHeight = tan(data.leftEyeFov.top * DEG_TO_RAD);
        float focusPositionY = topHalfHeight / (topHalfHeight + bottomHalfHeight);
        focusPositionY += data.foveationVerticalOffset;
        if (data.mode == FOVEATION_MODE_SLICES) {
            focusPositionY = Align4Normalized(focusPositionY, targetEyeHeight);
        }

        //calculate foveation scale such as the "area" of the foveation region remains equal to (mFoveationStrengthMean)^2
        // solve for {foveationScaleX, foveationScaleY}:
        // /{ foveationScaleX * foveationScaleY = (mFoveationStrengthMean)^2
        // \{ foveationScaleX / foveationScaleY = 1 / mFoveationShapeRatio
        // then foveationScaleX := foveationScaleX / (targetEyeWidth / targetEyeHeight) to compensate for non square frame.
        float foveationStrength = data.foveationStrength;
        float foveationShape = data.foveationShape;
        if (data.mode == FOVEATION_MODE_SLICES) {
            foveationStrength = 1.f / (foveationStrength / 2.f + 1.f);
            foveationShape = 1.f / foveationShape;
        }
        float scaleCoeff = foveationStrength * sqrt(foveationShape);
        float foveationScaleX = scaleCoeff / foveationShape / (targetEyeWidth / targetEyeHeight);
        float foveationScaleY = scaleCoeff;
        if (data.mode == FOVEATION_MODE_SLICES) {
            foveationScaleX = Align4Normalized(foveationScaleX, targetEyeWidth);
            foveationScaleY = Align4Normalized(foveationScaleY, targetEyeHeight);
        }

        float optimizedEyeWidth = 0;
        float optimizedEyeHeight = 0;
        float boundStartX = 0;
        float boundStartY = 0;
        float distortedWidth = 0;
        float distortedHeight = 0;

        if (data.mode == FOVEATION_MODE_SLICES) {
            optimizedEyeWidth = CalcOptimalDimensionForSlicing(foveationScaleX, targetEyeWidth);
            optimizedEyeHeight = CalcOptimalDimensionForSlicing(foveationScaleY, targetEyeHeight);

        } else if (data.mode == FOVEATION_MODE_WARP) {
            boundStartX = CalcBoundStart(focusPositionX, foveationScaleX);
            boundStartY = CalcBoundStart(focusPositionY, foveationScaleY);

            distortedWidth = CalcDistortedDimension(focusPositionX, foveationScaleX);
            distortedHeight = CalcDistortedDimension(focusPositionY, foveationScaleY);

            optimizedEyeWidth = CalcOptimalDimensionForWarp(
                    foveationScaleX, distortedWidth, targetEyeWidth);
            optimizedEyeHeight = CalcOptimalDimensionForWarp(
                    foveationScaleY, distortedHeight, targetEyeHeight);
        }

        // round the frame dimensions to a number of pixel multiple of 32 for the encoder
        auto optimizedEyeWidthAligned = (uint32_t)ceil(optimizedEyeWidth / 32.f) * 32;
        auto optimizedEyeHeightAligned = (uint32_t)ceil(optimizedEyeHeight / 32.f) * 32;

        return {data.eyeWidth, data.eyeHeight, optimizedEyeWidthAligned, optimizedEyeHeightAligned,
                focusPositionX, focusPositionY, foveationScaleX, foveationScaleY,
                boundStartX, boundStartY, distortedWidth, distortedHeight};
    }
}


FFR::FFR(Texture *inputSurface)
        : mInputSurface(inputSurface) {
}

void FFR::Initialize(FFRData ffrData) {
    auto fv = CalculateFoveationVars(ffrData);
    auto ffrCommonShaderStr = string_format(FFR_COMMON_SHADER_FORMAT,
                                            fv.targetEyeWidth, fv.targetEyeHeight,
                                            fv.optimizedEyeWidth, fv.optimizedEyeHeight,
                                            fv.focusPositionX, fv.focusPositionY,
                                            fv.foveationScaleX, fv.foveationScaleY,
                                            fv.boundStartX, fv.boundStartY,
                                            fv.distortedWidth, fv.distortedHeight);
    mExpandedTexture.reset(new Texture(false, ffrData.eyeWidth * 2, ffrData.eyeHeight, GL_RGB8));


    switch (ffrData.mode) {
        case FOVEATION_MODE_DISABLED:
            mExpandedTexture.reset(mInputSurface);
            break;
        case FOVEATION_MODE_SLICES: {
            auto decompressSlicesShaderStr = ffrCommonShaderStr + DECOMPRESS_SLICES_FRAGMENT_SHADER;
            auto decompressSlicesPipeline = new RenderPipeline(
                    {mInputSurface}, decompressSlicesShaderStr, mExpandedTexture.get());

            mPipelines.push_back(std::unique_ptr<RenderPipeline>(decompressSlicesPipeline));
            break;
        }
        case FOVEATION_MODE_WARP:
            //mSharpenedTexture = std::make_unique<Texture>(false, ffrData.eyeWidth * 2, ffrData.eyeHeight,
            //                                              GL_RGB8);

            auto undistortShaderStr = ffrCommonShaderStr + UNDISTORT_FRAGMENT_SHADER;
            //auto sharpeningShaderStr = ffrCommonShaderStr + SHARPENING_FRAGMENT_SHADER;

            auto undistortPipeline = new RenderPipeline(
                    {mInputSurface}, undistortShaderStr, mExpandedTexture.get());
            //auto sharpeningPipeline = RenderPipeline({mDistortedTexture.get()}, sharpeningShaderStr,
            //                                              mSharpenedTexture.get()));

            mPipelines.push_back(std::unique_ptr<RenderPipeline>(undistortPipeline));
            break;
    }


}

void FFR::Render() {
    for (auto &p : mPipelines) {
        p->Render();
    }
}
