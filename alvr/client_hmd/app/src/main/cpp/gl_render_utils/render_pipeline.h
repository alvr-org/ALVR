#pragma once

#include "texture.h"

namespace gl_render_utils {

    const std::string QUAD_2D_VERTEX_SHADER = R"glsl(
        #version 300 es
        out vec2 uv;
        void main() {
            uv = vec2(gl_VertexID & 1, gl_VertexID >> 1);
            gl_Position = vec4((uv - 0.5) * 2., 0, 1);
        }
    )glsl";

    class RenderPipeline {
    public:
        RenderPipeline(const std::vector<Texture *> &inputTextures,
                       const std::string &fragmentShaderStr,
                       Texture *renderTarget, size_t uniformBlockSize = 0);

        RenderPipeline(const std::vector<Texture *> &inputTextures,
                       const std::string &vertexShaderStr, const std::string &fragmentShaderStr,
                       Texture *renderTarget, size_t uniformBlockSize = 0);

        void Render(const void *uniformBlockData = nullptr);

        ~RenderPipeline();

    private:
        static GLuint mBindingPointCounter;

        void Initialize(const std::vector<Texture *> &inputTextures,
                        const std::string &vertexShader, const std::string &fragmentShader,
                        Texture *renderTarget, size_t uniformBlockSize);

        Texture *mRenderTarget;
        GLuint mProgram, mFrameBuffer, mVertexShader, mFragmentShader;

        struct TextureInfo {
            Texture *texture;
            GLint uniformLocation;
        };
        std::vector<TextureInfo> mInputTexturesInfo;

        GLuint mBlockBuffer;
        size_t mUniformBlockSize;
    };
}
