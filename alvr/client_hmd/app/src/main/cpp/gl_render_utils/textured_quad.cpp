#include "textured_quad.h"
#include "../utils.h"

#include <vector>
#include <string>

using namespace std;
using namespace glm;

const string QUAD_3D_VERTEX_SHADER = R"glsl(
    #version 300 es

    uniform UniformBlock {
        mat4 mvp;
        float opacity;
    };

    out vec2 uv;

    void main() {
        uv = vec2(gl_VertexID & 1, gl_VertexID >> 1);
        gl_Position = mvp * vec4((uv - 0.5) * 2., 0, 1);
    }
)glsl";

const string QUAD_3D_FRAGMENT_SHADER_FORMAT = R"glsl(
    #version 300 es
    #extension GL_OES_EGL_image_external_essl3 : enable

    uniform UniformBlock {
        mat4 mvp;
        float opacity;
    };
    uniform %s tex0;

    in vec2 uv;
    out vec4 color;

    void main() {
        color = texture(tex0, uv);
        color = vec4(color.rgb, color.a * opacity);
    }
)glsl";

namespace gl_render_utils {
    TexturedQuad::TexturedQuad(const Texture *texture, mat4 transform) {
        mTransform = transform;

        const char *samplerString = texture->IsOES() ? "samplerExternalOES" : "sampler2D";
        string fragmentShader = string_format(QUAD_3D_FRAGMENT_SHADER_FORMAT, samplerString);

        mPipeline.reset(new RenderPipeline({texture}, QUAD_3D_VERTEX_SHADER, fragmentShader,
                                           sizeof(UniformBlock)));
    }

    void TexturedQuad::Render(const RenderState &renderState, const mat4 &camera) const {
        auto mvp = camera * mTransform;
        UniformBlock block = {mvp, mOpacity};

        mPipeline->Render(renderState, &block);
    }
}