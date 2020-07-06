#include "render_pipeline.h"
#include "../utils.h"

namespace {

    GLuint createShader(GLenum type, const std::string &shaderStr) {
        auto shader = glCreateShader(type);
        auto *shaderCStr = shaderStr.c_str();
        GL(glShaderSource(shader, 1, &shaderCStr, nullptr));

        GLint compiled;
        GL(glCompileShader(shader));
        GL(glGetShaderiv(shader, GL_COMPILE_STATUS, &compiled));
        if (!compiled) {
            char errorLog[1000];
            GL(glGetShaderInfoLog(shader, sizeof(errorLog), nullptr, errorLog));
            LOGE("SHADER COMPILATION ERROR: %s\nSHADER:\n%s", errorLog, shaderCStr);
        }
        return shader;
    }
}

namespace gl_render_utils {

    GLuint RenderPipeline::mBindingPointCounter = 0;

    RenderPipeline::RenderPipeline(const std::vector<Texture *> &inputTextures,
                                   const std::string &fragmentShader,
                                   Texture *renderTarget, size_t uniformBlockSize) {
        Initialize(inputTextures, QUAD_2D_VERTEX_SHADER, fragmentShader, renderTarget,
                   uniformBlockSize);
    }

    RenderPipeline::RenderPipeline(const std::vector<Texture *> &inputTextures,
                                   const std::string &vertexShader,
                                   const std::string &fragmentShader,
                                   Texture *renderTarget, size_t uniformBlockSize) {
        Initialize(inputTextures, vertexShader, fragmentShader, renderTarget, uniformBlockSize);
    }

    void RenderPipeline::Initialize(const std::vector<Texture *> &inputTextures,
                                    const std::string &vertexShaderStr,
                                    const std::string &fragmentShaderStr,
                                    Texture *renderTarget, size_t uniformBlockSize) {
        mRenderTarget = renderTarget;

        mVertexShader = createShader(GL_VERTEX_SHADER, vertexShaderStr);
        mFragmentShader = createShader(GL_FRAGMENT_SHADER, fragmentShaderStr);

        mProgram = glCreateProgram();
        GL(glAttachShader(mProgram, mVertexShader));
        GL(glAttachShader(mProgram, mFragmentShader));

        GLint linked;
        GL(glLinkProgram(mProgram));
        GL(glGetProgramiv(mProgram, GL_LINK_STATUS, &linked));
        if (!linked) {
            char errorLog[1000];
            GL(glGetProgramInfoLog(mProgram, sizeof(errorLog), nullptr, errorLog));
            LOGE("SHADER LINKING ERROR: %s", errorLog);
        }

        for (size_t i = 0; i < inputTextures.size(); i++) {
            mInputTexturesInfo.push_back(
                    {inputTextures[i],
                     glGetUniformLocation(mProgram, ("tex" + std::to_string(i)).c_str())});
        }

        mUniformBlockSize = uniformBlockSize;
        if (mUniformBlockSize > 0) {
            GL(glUniformBlockBinding(mProgram, 0, mBindingPointCounter));
            GL(glGenBuffers(1, &mBlockBuffer));
            GL(glBindBuffer(GL_UNIFORM_BUFFER, mBlockBuffer));
            GL(glBufferData(GL_UNIFORM_BUFFER, uniformBlockSize, nullptr, GL_DYNAMIC_DRAW));
            GL(glBindBufferBase(GL_UNIFORM_BUFFER, mBindingPointCounter, mBlockBuffer));
            mBindingPointCounter++;
        }

        GL(glGenFramebuffers(1, &mFrameBuffer));
        GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, mFrameBuffer));
        GL(glFramebufferTexture2D(GL_DRAW_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                                  mRenderTarget->GetGLTexture(), 0));
    }

    void RenderPipeline::Render(const void *uniformBlockData) {
        GL(glUseProgram(mProgram));
        GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, mFrameBuffer));

        GL(glDisable(GL_SCISSOR_TEST));
        GL(glDepthMask(GL_FALSE));
        GL(glDisable(GL_DEPTH_TEST));
        GL(glDisable(GL_CULL_FACE));
        GL(glViewport(0, 0, mRenderTarget->GetWidth(), mRenderTarget->GetHeight()));

        for (size_t i = 0; i < mInputTexturesInfo.size(); i++) {
            GL(glActiveTexture(GL_TEXTURE0 + i));
            GL(glBindTexture(mInputTexturesInfo[i].texture->GetTarget(),
                             mInputTexturesInfo[i].texture->GetGLTexture()));
            GL(glUniform1i(mInputTexturesInfo[i].uniformLocation, i));
        }

        if (uniformBlockData != nullptr) {
            GL(glBindBuffer(GL_UNIFORM_BUFFER, mBlockBuffer));
            GL(glBufferSubData(GL_UNIFORM_BUFFER, 0, mUniformBlockSize, uniformBlockData));
        }

        GL(glDrawArrays(GL_TRIANGLE_STRIP, 0, 4));
    }

    RenderPipeline::~RenderPipeline() {
        GL(glDeleteFramebuffers(1, &mFrameBuffer));
        GL(glDeleteBuffers(1, &mBlockBuffer))
        GL(glDeleteShader(mVertexShader));
        GL(glDeleteShader(mFragmentShader));
        GL(glDeleteProgram(mProgram));
    }
}
