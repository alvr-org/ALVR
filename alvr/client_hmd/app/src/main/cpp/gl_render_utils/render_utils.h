#pragma once

#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>

#include <vector>
#include <string>

namespace gl_render_utils {

    const std::string QUAD_VERTEX_SHADER = R"glsl(
        #version 300 es
        out vec2 uv;
        void main() {
            uv = vec2(gl_VertexID & 1, gl_VertexID >> 1);
            gl_Position = vec4((uv - 0.5) * 2., 0, 1);
        }
    )glsl";

    class Texture {
    public:
        Texture(bool oes, uint32_t width = 0, uint32_t height = 0, GLenum format = GL_RGBA);

        Texture(GLuint externalHandle, bool oes, uint32_t width = 0, uint32_t height = 0,
                GLenum format = GL_RGBA);

        uint32_t GetWidth() { return mWidth; }

        uint32_t GetHeight() { return mHeight; }

        GLuint GetGLTexture() { return mGLTexture; }

        GLenum GetTarget() { return mTarget; }

        ~Texture();

    private:
        void
        initialize(bool external, GLuint externalHandle, bool oes, uint32_t width, uint32_t height,
                   GLenum format);

        uint32_t mWidth, mHeight;
        GLuint mGLTexture;
        GLenum mTarget;
        bool mExternal;
    };
}
