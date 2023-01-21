#include "render_pipeline.h"
#include "../utils.h"

using namespace std;

GLuint createShader(GLenum type, const string &shaderStr) {
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

namespace gl_render_utils {

RenderState::RenderState(const Texture *renderTarget) {
    mRenderTarget = renderTarget;
    mDepthTarget = make_unique<Texture>(false,
                                        0,
                                        false,
                                        renderTarget->GetWidth(),
                                        renderTarget->GetHeight(),
                                        GL_DEPTH_COMPONENT32F,
                                        GL_DEPTH_COMPONENT32F);

    GL(glGenFramebuffers(1, &mFrameBuffer));
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, mFrameBuffer));
    GL(glFramebufferTexture2D(GL_DRAW_FRAMEBUFFER,
                              GL_COLOR_ATTACHMENT0,
                              mRenderTarget->GetTarget(),
                              renderTarget->GetGLTexture(),
                              0));
    GL(glFramebufferTexture2D(GL_DRAW_FRAMEBUFFER,
                              GL_DEPTH_ATTACHMENT,
                              mDepthTarget->GetTarget(),
                              mDepthTarget->GetGLTexture(),
                              0));
}

RenderState::~RenderState() { GL(glDeleteFramebuffers(1, &mFrameBuffer)); }

void RenderState::ClearDepth() const {
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, mFrameBuffer));
    GL(glDisable(GL_SCISSOR_TEST));
    GL(glClear(GL_DEPTH_BUFFER_BIT));
}

GLuint RenderPipeline::mBindingPointCounter = 0;

RenderPipeline::RenderPipeline(const vector<const Texture *> &inputTextures,
                               const string &vertexShader,
                               const string &fragmentShader,
                               size_t uniformBlockSize) {
    mVertexShader = createShader(GL_VERTEX_SHADER, vertexShader);
    mFragmentShader = createShader(GL_FRAGMENT_SHADER, fragmentShader);

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
            {inputTextures[i], glGetUniformLocation(mProgram, ("tex" + to_string(i)).c_str())});
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
}

void RenderPipeline::Render(const RenderState &renderState, const void *uniformBlockData) const {
    GL(glUseProgram(mProgram));
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, renderState.GetFrameBuffer()));

    GL(glDisable(GL_SCISSOR_TEST));
    GL(glDepthMask(GL_TRUE));
    GL(glDisable(GL_CULL_FACE));
    GL(glEnable(GL_DEPTH_TEST));
    GL(glEnable(GL_BLEND));
    GL(glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA));
    GL(glViewport(0,
                  0,
                  renderState.GetRenderTarget()->GetWidth(),
                  renderState.GetRenderTarget()->GetHeight()));

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
    if (GL_TRUE == glIsBuffer(mBlockBuffer)) {
        GL(glDeleteBuffers(1, &mBlockBuffer));
    }
    if (GL_TRUE == glIsShader(mVertexShader)) {
        GL(glDetachShader(mProgram, mVertexShader));
        GL(glDeleteShader(mVertexShader));
    }
    if (GL_TRUE == glIsShader(mFragmentShader)) {
        GL(glDetachShader(mProgram, mFragmentShader));
        GL(glDeleteShader(mFragmentShader));
    }
    if (GL_TRUE == glIsProgram(mProgram)) {
        GL(glDeleteProgram(mProgram));
    }
}
} // namespace gl_render_utils
