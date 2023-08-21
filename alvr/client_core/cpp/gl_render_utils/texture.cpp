#include "texture.h"

#include "../utils.h"

using namespace std;

namespace gl_render_utils {
Texture::Texture(bool external,
                 GLuint externalHandle,
                 bool oes,
                 uint32_t width,
                 uint32_t height,
                 GLint internalFormat,
                 GLenum format,
                 vector<uint8_t> content) {
    mOES = oes;
    mWidth = width;
    mHeight = height;
    mTarget = oes ? GL_TEXTURE_EXTERNAL_OES : GL_TEXTURE_2D;
    mExternal = external;

    if (external) {
        mGLTexture = externalHandle;
    } else {
        GL(glGenTextures(1, &mGLTexture));
    }
    GL(glBindTexture(mTarget, mGLTexture));
    if (!oes && !external && width != 0 && height != 0) {
        if (!content.empty()) {
            GL(glTexImage2D(mTarget,
                            0,
                            internalFormat,
                            width,
                            height,
                            0,
                            format,
                            GL_UNSIGNED_BYTE,
                            &content[0]));
        } else {
            GL(glTexStorage2D(mTarget, 1, internalFormat, width, height));
        }
    }
    GL(glTexParameteri(mTarget, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE));
    GL(glTexParameteri(mTarget, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE));
    GL(glTexParameteri(mTarget, GL_TEXTURE_MAG_FILTER, GL_LINEAR));
    GL(glTexParameteri(mTarget, GL_TEXTURE_MIN_FILTER, GL_LINEAR));
    LOGV("New texture Created id#%d", mGLTexture);
}

Texture::~Texture() {
    if (!mExternal) {
        GL(glDeleteTextures(1, &mGLTexture));
        LOGV("texture id#%d deleted", mGLTexture);
    }
}
} // namespace gl_render_utils
