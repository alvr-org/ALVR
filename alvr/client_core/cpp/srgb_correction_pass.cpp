#include "srgb_correction_pass.h"
#include "utils.h"
#include <cmath>
#include <cstdint>
#include <memory>
#include <sys/types.h>

using namespace std;
using namespace gl_render_utils;

namespace {
const string SRGB_CORRECTION_FRAGMENT_SHADER = R"glsl(#version 300 es
        #extension GL_OES_EGL_image_external_essl3 : enable
        precision mediump float;

        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;

        const float DIV12 = 1. / 12.92;
        const float DIV1 = 1. / 1.055;
        const float THRESHOLD = 0.04045;
        const vec3 GAMMA = vec3(2.4);

        void main()
        {
            color = texture(tex0, uv);

            vec3 condition = vec3(color.r < THRESHOLD, color.g < THRESHOLD, color.b < THRESHOLD);
            vec3 lowValues = color.rgb * DIV12;
            vec3 highValues = pow((color.rgb + 0.055) * DIV1, GAMMA);
            color.rgb = condition * lowValues + (1.0 - condition) * highValues;
        }
    )glsl";
const string PASSTHOUGH_FRAGMENT_SHADER = R"glsl(#version 300 es
        #extension GL_OES_EGL_image_external_essl3 : enable
        precision mediump float;

        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;

        void main()
        {
            color = texture(tex0, uv);
        }
    )glsl";
} // namespace

SrgbCorrectionPass::SrgbCorrectionPass(Texture *inputSurface) : mInputSurface(inputSurface) {}

void SrgbCorrectionPass::Initialize(uint32_t width, uint32_t height, bool passthrough) {
    mOutputTexture.reset(new Texture(false, 0, false, width * 2, height));
    mOutputTextureState = make_unique<RenderState>(mOutputTexture.get());

    auto fragmentShader =
        passthrough ? PASSTHOUGH_FRAGMENT_SHADER : SRGB_CORRECTION_FRAGMENT_SHADER;
    mStagingPipeline = unique_ptr<RenderPipeline>(
        new RenderPipeline({mInputSurface}, QUAD_2D_VERTEX_SHADER, fragmentShader));
}

void SrgbCorrectionPass::Render() const {
    mOutputTextureState->ClearDepth();
    mStagingPipeline->Render(*mOutputTextureState);
}
