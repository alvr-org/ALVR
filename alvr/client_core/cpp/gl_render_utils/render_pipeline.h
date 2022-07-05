#pragma once

#include "texture.h"

namespace gl_render_utils {

    const std::string QUAD_2D_VERTEX_SHADER = R"glsl(#version 300 es
        out vec2 uv;
        void main() {
            uv = vec2(gl_VertexID & 1, gl_VertexID >> 1);
            gl_Position = vec4((uv - 0.5) * 2., 0, 1);
        }
    )glsl";

    class RenderState {
    public:
        RenderState(const Texture *renderTarget);

        void ClearDepth() const;

        GLuint GetFrameBuffer() const {
            return mFrameBuffer;
        }

        const Texture *GetRenderTarget() const {
            return mRenderTarget;
        }

        ~RenderState();

    private:
        const Texture *mRenderTarget;
        std::unique_ptr<Texture> mDepthTarget;
        GLuint mFrameBuffer = 0;
    };

    // Supports rendering a single quad without vertex buffers. The geometry must be defined in the vertex shader.
    // Texture samplers defined within the fragment shader must be named tex0, tex1, etc... with the sampler type matching the ones of the inputTextures.
    class RenderPipeline {
    public:
        RenderPipeline(const std::vector<const Texture *> &inputTextures,
                       const std::string &vertexShaderStr, const std::string &fragmentShaderStr,
                       size_t uniformBlockSize = 0);

        void Render(const RenderState &renderState, const void *uniformBlockData = nullptr) const;

        ~RenderPipeline();

    private:
        static GLuint mBindingPointCounter;

        GLuint mProgram, mVertexShader, mFragmentShader;

        struct TextureInfo {
            const Texture *texture;
            GLint uniformLocation;
        };
        std::vector<TextureInfo> mInputTexturesInfo;

        GLuint mBlockBuffer = 0;
        size_t mUniformBlockSize;
    };
}
