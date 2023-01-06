#pragma once

#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>

#include <vector>
#include <string>

namespace gl_render_utils {

    class Texture {
    public:
        Texture(bool oes, uint32_t width = 0, uint32_t height = 0, GLenum format = GL_RGBA,
                std::vector<uint8_t> content = {});

        Texture(GLuint externalHandle, bool oes, uint32_t width = 0, uint32_t height = 0,
                GLenum format = GL_RGBA, std::vector<uint8_t> content = {});

        uint32_t GetWidth() const { return mWidth; }

        uint32_t GetHeight() const { return mHeight; }

        GLuint GetGLTexture() const { return mGLTexture; }

        GLenum GetTarget() const { return mTarget; }

        bool IsOES() const { return mOES; }

        ~Texture();

    private:
        void
        initialize(bool external, GLuint externalHandle, bool oes, uint32_t width, uint32_t height,
                   GLenum format, std::vector<uint8_t> &content);

        bool mOES;
        uint32_t mWidth, mHeight;
        GLuint mGLTexture;
        GLenum mTarget;
        bool mExternal;
    };
}
