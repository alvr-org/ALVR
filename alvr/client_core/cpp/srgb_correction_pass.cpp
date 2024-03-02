#include "srgb_correction_pass.h"
#include "utils.h"
#include <cmath>
#include <cstdint>
#include <memory>
#include <sys/types.h>

using namespace std;
using namespace gl_render_utils;

namespace {
const string CORRECTION_FRAGMENT_SHADER_HEADER = R"glsl(#version 300 es
        #extension GL_OES_EGL_image_external_essl3 : enable
        precision mediump float;)glsl";

const string CORRECTION_FRAGMENT_SHADER = R"glsl(
        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;

        const float DIV12 = 1. / 12.92;
        const float DIV1 = 1. / 1.055;
        const float THRESHOLD = 0.04045;
        const vec3 GAMMA = vec3(2.4);

        // Convert from limited colors to full
        const float LIMITED_MIN = 16.0 / 255.0;
        const float LIMITED_MAX = 235.0 / 255.0;

        void main()
        {
            color = texture(tex0, uv);

            // For some reason, the encoder shifts full-range color into the negatives and over one.
            color.rgb = LIMITED_MIN + ((LIMITED_MAX - LIMITED_MIN) * color.rgb);
#ifdef SRGB_CORRECTION
            vec3 condition = vec3(color.r < THRESHOLD, color.g < THRESHOLD, color.b < THRESHOLD);
            vec3 lowValues = color.rgb * DIV12;
            vec3 highValues = pow((color.rgb + 0.055) * DIV1, GAMMA);
            color.rgb = condition * lowValues + (1.0 - condition) * highValues;
#endif
        }
    )glsl";
} // namespace

SrgbCorrectionPass::SrgbCorrectionPass(Texture *inputSurface) : mInputSurface(inputSurface) {}

void SrgbCorrectionPass::Initialize(uint32_t width, uint32_t height, bool passthrough) {
    mOutputTexture.reset(new Texture(false, 0, false, width * 2, height));
    mOutputTextureState = make_unique<RenderState>(mOutputTexture.get());

    string defines = passthrough ? "" : "#define SRGB_CORRECTION";

    auto fragmentShader = CORRECTION_FRAGMENT_SHADER_HEADER + "\n" + defines + "\n" + CORRECTION_FRAGMENT_SHADER;
    mStagingPipeline = unique_ptr<RenderPipeline>(
        new RenderPipeline({mInputSurface}, QUAD_2D_VERTEX_SHADER, fragmentShader));
}

void SrgbCorrectionPass::Render() const {
    mOutputTextureState->ClearDepth();
    mStagingPipeline->Render(*mOutputTextureState);
}
