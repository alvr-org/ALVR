#ifndef ALVRCLIENT_RENDER_H
#define ALVRCLIENT_RENDER_H

#include <GLES3/gl3.h>
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <glm/glm.hpp>
#include <glm/gtc/quaternion.hpp>
#include "gltf_model.h"
#include "utils.h"
#include "ffr.h"
#include "bindings.h"

// Must use EGLSyncKHR because the VrApi still supports OpenGL ES 2.0
#define EGL_SYNC

struct Render_EGL {
    EGLDisplay Display;
    EGLConfig Config;
    EGLSurface TinySurface;
    EGLSurface MainSurface;
    EGLContext Context;
};
extern Render_EGL egl;

void eglInit();
void eglDestroy();

struct EyeInput {
    glm::quat orientation;
    glm::vec3 position;
    EyeFov fov;
};

//
// ovrFramebuffer
//

typedef struct {
    std::vector<std::unique_ptr<gl_render_utils::Texture>> renderTargets;
    std::vector<std::unique_ptr<gl_render_utils::RenderState>> renderStates;
} ovrFramebuffer;

//
// ovrGeometry
//

typedef struct {
    GLuint Index;
    GLint Size;
    GLenum Type;
    GLboolean Normalized;
    GLsizei Stride;
    const GLvoid *Pointer;
} ovrVertexAttribPointer;

static const int MAX_VERTEX_ATTRIB_POINTERS = 5;

typedef struct {
    GLuint VertexBuffer;
    GLuint IndexBuffer;
    GLuint VertexArrayObject;
    GLuint VertexUVBuffer;
    int VertexCount;
    int IndexCount;
    ovrVertexAttribPointer VertexAttribs[MAX_VERTEX_ATTRIB_POINTERS];
} ovrGeometry;

//
// ovrProgram
//

static const int MAX_PROGRAM_UNIFORMS = 8;
static const int MAX_PROGRAM_TEXTURES = 8;

typedef struct {
    GLuint Program;
    GLuint VertexShader;
    GLuint FragmentShader;
    // These will be -1 if not used by the program.
    GLint UniformLocation[MAX_PROGRAM_UNIFORMS];    // ProgramUniforms[].name
    GLint UniformBinding[MAX_PROGRAM_UNIFORMS];    // ProgramUniforms[].name
    GLint Textures[MAX_PROGRAM_TEXTURES];            // Texture%i
} ovrProgram;

//
// ovrRenderer
//

typedef struct {
    ovrFramebuffer FrameBuffer[2];
    bool SceneCreated;
    ovrProgram Program;
    ovrProgram ProgramLoading;
    ovrGeometry Panel;
    gl_render_utils::Texture *streamTexture;
    GLuint LoadingTexture;
    GltfModel *loadingScene;
    std::unique_ptr<FFR> ffr;
    bool enableFFR;
} ovrRenderer;

void ovrRenderer_Create(ovrRenderer *renderer, int width, int height,
                        gl_render_utils::Texture *streamTexture, int LoadingTexture,
                        std::vector<GLuint> textures[2], bool darkMode, FFRData ffrData);
void ovrRenderer_Destroy(ovrRenderer *renderer);
void ovrRenderer_RenderFrame(ovrRenderer *renderer, EyeInput input[2], int swapchainIndex[2], bool loading);

#endif //ALVRCLIENT_RENDER_H
