#pragma once

#include "render_utils.h"

namespace gl_render_utils {

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
