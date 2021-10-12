#include "ffr.h"

#include <cmath>
#include <memory>

#include "utils.h"

using namespace std;
using namespace gl_render_utils;

namespace {
    const string FFR_COMMON_SHADER_FORMAT = R"glsl(
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
        const vec2 EYE_SIZE_RATIO = vec2(%f, %f);
        const vec2 CENTER_SIZE = vec2(%f, %f);
        const vec2 CENTER_SHIFT = vec2(%f, %f);
        const vec2 EDGE_RATIO = vec2(%f, %f);


        //Choose one distortion function:

        // ARCTANGENT: good for fixed foveated rendering
        //const float EPS = 0.000001;
        //#define INVERSE_DISTORTION_FN(a)   atan(a)
        //#define INV_DIST_DERIVATIVE(a)     atanDerivative(a)
        //float atanDerivative(float a) {
        //    return 1. / (a * a + 1.);
        //}

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


        //vec2 InverseRadialDistortion(vec2 xy) {
        //    vec2 scaledXY = xy * FOVEATION_SCALE;
        //    float scaledRadius = length(scaledXY);
        //    return INVERSE_DISTORTION_FN(scaledRadius) * scaledXY / scaledRadius;
        //}

        //// Inverse radial distortion derivative wrt length(xy)
        //vec2 InverseRadialDistortionDerivative(vec2 xy) {
        //    vec2 scaledXY = xy * FOVEATION_SCALE;
        //    float scaledRadius = length(scaledXY);
        //    return (INV_DIST_DERIVATIVE(scaledRadius) * FOVEATION_SCALE) * scaledXY / scaledRadius;
        //}

        //vec2 Undistort(vec2 uv) {
        //    return (InverseRadialDistortion(uv - FOCUS_POSITION) - BOUND_START) / DISTORTED_SIZE;
        //}

        //vec2 UndistortRadialDerivative(vec2 uv) {
        //    return InverseRadialDistortionDerivative(uv - FOCUS_POSITION) / DISTORTED_SIZE;
        //}

        //vec2 GetFilteringWeight2D(vec2 uv) {
        //    float radialExpansion = length(UndistortRadialDerivative(uv));
        //    vec2 contraction = 1. / (radialExpansion * RESOLUTION_SCALE);
		//
        //    vec2 modifiedContraction = contraction - 1. / contraction; // -> ?
		//
        //    return max(modifiedContraction, EPS);
        //}

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

    const string UNDISTORT_FRAGMENT_SHADER = R"glsl(
        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;
        void main() {
            //bool isRightEye = uv.x > 0.5;
            //vec2 undistortedUV = Undistort(TextureToEyeUV(uv, isRightEye));
            //color = texture(tex0, EyeToTextureUV(undistortedUV, isRightEye));

            color = texture(tex0, uv);
        }
    )glsl";

    const string SHARPENING_FRAGMENT_SHADER = R"glsl(
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

    const string DECOMPRESS_SLICES_FRAGMENT_SHADER = R"glsl(
        const vec2 EDGE_COMPRESSED_SIZE = (1.-CENTER_SIZE)/(2.*EDGE_RATIO);

        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;
        void main() {
            bool isRightEye = uv.x > 0.5;
            vec2 eyeUV = TextureToEyeUV(uv, isRightEye);

            vec2 alignedUV = eyeUV;

            vec2 loBound = EDGE_RATIO*EDGE_COMPRESSED_SIZE*(CENTER_SHIFT+1.);
            vec2 hiBound = EDGE_RATIO*EDGE_COMPRESSED_SIZE*(CENTER_SHIFT-1.)+1.;
            vec2 underBound = vec2(alignedUV.x<loBound.x,alignedUV.y<loBound.y);
            vec2 inBound = vec2(loBound.x<alignedUV.x&&alignedUV.x<hiBound.x,loBound.y<alignedUV.y&&alignedUV.y<hiBound.y);
            vec2 overBound = vec2(alignedUV.x>hiBound.x,alignedUV.y>hiBound.y);

            vec2 center = EDGE_RATIO*(alignedUV+EDGE_COMPRESSED_SIZE*(1.-EDGE_RATIO)*(CENTER_SHIFT+1.))/((EDGE_RATIO-1.)*CENTER_SIZE+1.);
            vec2 leftEdge = alignedUV/((EDGE_RATIO-1.)*CENTER_SIZE+1.);
            vec2 rightEdge = (alignedUV-1.)/((EDGE_RATIO-1.)*CENTER_SIZE+1.)+1.;

            vec2 uncompressedUV = underBound*leftEdge+inBound*center+overBound*rightEdge;

            color = texture(tex0, EyeToTextureUV(uncompressedUV * EYE_SIZE_RATIO, isRightEye));
        }
    )glsl";

    const float DEG_TO_RAD = (float) M_PI / 180;

#define INVERSE_DISTORTION_FN(a) atan(a);

    float Align4Normalized(float scale, float originalDim) {
        return float(int(scale * originalDim / 4.f) * 4) / originalDim;
    }

    float CalcOptimalDimensionForSlicing(float scale, float originalDim) {
        return (1.f + 3.f * scale) / 4.f * originalDim + 6.f;
    }

    struct FoveationVars {
		uint32_t targetEyeWidth;
		uint32_t targetEyeHeight;
		uint32_t optimizedEyeWidth;
		uint32_t optimizedEyeHeight;

		float eyeWidthRatio;
		float eyeHeightRatio;

		float centerSizeX;
		float centerSizeY;
		float centerShiftX;
		float centerShiftY;
		float edgeRatioX;
		float edgeRatioY;
    };

    FoveationVars CalculateFoveationVars(FFRData data) {
        float targetEyeWidth = data.eyeWidth;
        float targetEyeHeight = data.eyeHeight;

		float centerSizeX = data.centerSizeX;
		float centerSizeY = data.centerSizeY;
		float centerShiftX = data.centerShiftX;
		float centerShiftY = data.centerShiftY;
		float edgeRatioX = data.edgeRatioX;
		float edgeRatioY = data.edgeRatioY;

		float edgeSizeX = targetEyeWidth-centerSizeX*targetEyeWidth;
		float edgeSizeY = targetEyeHeight-centerSizeY*targetEyeHeight;

		float centerSizeXAligned = 1.-ceil(edgeSizeX/(edgeRatioX*2.))*(edgeRatioX*2.)/targetEyeWidth;
		float centerSizeYAligned = 1.-ceil(edgeSizeY/(edgeRatioY*2.))*(edgeRatioY*2.)/targetEyeHeight;

		float edgeSizeXAligned = targetEyeWidth-centerSizeXAligned*targetEyeWidth;
		float edgeSizeYAligned = targetEyeHeight-centerSizeYAligned*targetEyeHeight;

		float centerShiftXAligned = ceil(centerShiftX*edgeSizeXAligned/(edgeRatioX*2.))*(edgeRatioX*2.)/edgeSizeXAligned;
		float centerShiftYAligned = ceil(centerShiftY*edgeSizeYAligned/(edgeRatioY*2.))*(edgeRatioY*2.)/edgeSizeYAligned;

		float foveationScaleX = (centerSizeXAligned+(1.-centerSizeXAligned)/edgeRatioX);
		float foveationScaleY = (centerSizeYAligned+(1.-centerSizeYAligned)/edgeRatioY);

		float optimizedEyeWidth = foveationScaleX*targetEyeWidth;
		float optimizedEyeHeight = foveationScaleY*targetEyeHeight;

		// round the frame dimensions to a number of pixel multiple of 32 for the encoder
		auto optimizedEyeWidthAligned = (uint32_t)ceil(optimizedEyeWidth / 32.f) * 32;
		auto optimizedEyeHeightAligned = (uint32_t)ceil(optimizedEyeHeight / 32.f) * 32;

		float eyeWidthRatioAligned = optimizedEyeWidth/optimizedEyeWidthAligned;
		float eyeHeightRatioAligned = optimizedEyeHeight/optimizedEyeHeightAligned;

        return {data.eyeWidth, data.eyeHeight, optimizedEyeWidthAligned, optimizedEyeHeightAligned,
			eyeWidthRatioAligned, eyeHeightRatioAligned,
			centerSizeXAligned, centerSizeYAligned, centerShiftXAligned, centerShiftYAligned, edgeRatioX, edgeRatioY };
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
                                            fv.eyeWidthRatio, fv.eyeHeightRatio,
                                            fv.centerSizeX, fv.centerSizeY,
                                            fv.centerShiftX, fv.centerShiftY,
                                            fv.edgeRatioX, fv.edgeRatioY);

    mExpandedTexture.reset(
            new Texture(false, ffrData.eyeWidth * 2, ffrData.eyeHeight, GL_RGB8));
    mExpandedTextureState = make_unique<RenderState>(mExpandedTexture.get());

    auto decompressSlicesShaderStr = ffrCommonShaderStr + DECOMPRESS_SLICES_FRAGMENT_SHADER;
    mDecompressSlicesPipeline = unique_ptr<RenderPipeline>(
            new RenderPipeline({mInputSurface}, QUAD_2D_VERTEX_SHADER,
                               decompressSlicesShaderStr));
}

void FFR::Render() const {
    mExpandedTextureState->ClearDepth();
    mDecompressSlicesPipeline->Render(*mExpandedTextureState);
}
