#pragma once

#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>

#include <vector>
#include <string>

namespace gl_render_utils {

    class Texture {
    public:
        Texture(bool oes, uint32_t width = 0, uint32_t height = 0, GLenum format = GL_RGBA);

        Texture(GLuint externalHandle, bool oes, uint32_t width = 0, uint32_t height = 0,
                GLenum format = GL_RGBA);

        uint32_t GetWidth() { return mWidth; }

        uint32_t GetHeight() { return mHeight; }

        GLuint GetGLTexture() { return mGLTexture; }

        GLenum GetTarget() { return mTarget; }

        bool IsOES() { return mOES; }

        ~Texture();

    private:
        void
        initialize(bool external, GLuint externalHandle, bool oes, uint32_t width, uint32_t height,
                   GLenum format);

        bool mOES;
        uint32_t mWidth, mHeight;
        GLuint mGLTexture;
        GLenum mTarget;
        bool mExternal;
    };
}
