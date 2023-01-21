#include <cmath>
#include <cstdint>
#include <memory>
#include <sys/types.h>

#include "gamma_pass.h"
#include "utils.h"

using namespace std;
using namespace gl_render_utils;

namespace {
const string PASSTHROUGH_FRAGMENT_SHADER = R"glsl(#version 300 es
        #extension GL_OES_EGL_image_external_essl3 : enable
        precision highp float;

        uniform samplerExternalOES tex0;
        in vec2 uv;
        out vec4 color;
        void main()
        {
            color = texture(tex0, uv);
            color.rgb = pow(color.rgb, vec3(2.2));
        }
    )glsl";
}

GammaPass::GammaPass(Texture *inputSurface) : mInputSurface(inputSurface) {}

void GammaPass::Initialize(uint32_t width, uint32_t height) {
    mOutputTexture.reset(new Texture(false, 0, false, width * 2, height));
    mOutputTextureState = make_unique<RenderState>(mOutputTexture.get());

    auto decompressAxisAlignedShaderStr = PASSTHROUGH_FRAGMENT_SHADER;
    mStagingPipeline = unique_ptr<RenderPipeline>(
        new RenderPipeline({mInputSurface}, QUAD_2D_VERTEX_SHADER, decompressAxisAlignedShaderStr));
}

void GammaPass::Render() const {
    mOutputTextureState->ClearDepth();
    mStagingPipeline->Render(*mOutputTextureState);
}
