#ifndef ALVRCLIENT_RENDER_H
#define ALVRCLIENT_RENDER_H

#include <VrApi.h>
#include <VrApi_Types.h>
#include <VrApi_Helpers.h>
#include <VrApi_SystemUtils.h>
#include <VrApi_Input.h>
#include <GLES3/gl3.h>
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include "gltf_model.h"
#include "utils.h"
#include "ffr.h"


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

//
// ovrFramebuffer
//

typedef struct {
    int TextureSwapChainLength;
    int TextureSwapChainIndex;
    ovrTextureSwapChain *ColorTextureSwapChain;
    std::vector<std::unique_ptr<gl_render_utils::Texture>> renderTargets;
    std::vector<std::unique_ptr<gl_render_utils::RenderState>> renderStates;
} ovrFramebuffer;

bool ovrFramebuffer_Create(ovrFramebuffer *frameBuffer, const GLenum colorFormat, const int width,
                           const int height);

void ovrFramebuffer_Destroy(ovrFramebuffer *frameBuffer);

void ovrFramebuffer_SetCurrent(ovrFramebuffer *frameBuffer);

void ovrFramebuffer_SetNone();

void ovrFramebuffer_Resolve();

void ovrFramebuffer_Advance(ovrFramebuffer *frameBuffer);

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

enum VertexAttributeLocation {
    VERTEX_ATTRIBUTE_LOCATION_POSITION,
    VERTEX_ATTRIBUTE_LOCATION_COLOR,
    VERTEX_ATTRIBUTE_LOCATION_UV,
    VERTEX_ATTRIBUTE_LOCATION_TRANSFORM,
    VERTEX_ATTRIBUTE_LOCATION_NORMAL
};

void ovrGeometry_Clear(ovrGeometry *geometry);

void ovrGeometry_CreatePanel(ovrGeometry *geometry);

void ovrGeometry_Destroy(ovrGeometry *geometry);

void ovrGeometry_CreateVAO(ovrGeometry *geometry);

void ovrGeometry_DestroyVAO(ovrGeometry *geometry);

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


bool
ovrProgram_Create(ovrProgram *program, const char *vertexSource, const char *fragmentSource);

void ovrProgram_Destroy(ovrProgram *program);

//
// ovrRenderer
//


typedef struct {
    ovrFramebuffer FrameBuffer[VRAPI_FRAME_LAYER_EYE_MAX];
    int NumBuffers;
    bool SceneCreated;
    ovrProgram Program;
    ovrProgram ProgramLoading;
    ovrGeometry Panel;
    gl_render_utils::Texture *streamTexture;
    GLuint LoadingTexture;
    GltfModel *loadingScene;
    std::unique_ptr<FFR> ffr;
    gl_render_utils::Texture *ffrSourceTexture;
    bool enableFFR;
} ovrRenderer;

void ovrRenderer_Create(ovrRenderer *renderer, int width, int height,
                        gl_render_utils::Texture *streamTexture, int LoadingTexture,
                        FFRData ffrData);

void ovrRenderer_Destroy(ovrRenderer *renderer);

void ovrRenderer_CreateScene(ovrRenderer *renderer, bool darkMode);

// Set up an OVR frame, render it, and submit it.
ovrLayerProjection2 ovrRenderer_RenderFrame(ovrRenderer *renderer, const ovrTracking2 *tracking,
                                            bool loading);

// Render the contents of the frame in an SDK-neutral manner.
void renderEye(int eye, ovrMatrix4f mvpMatrix[2], Recti *viewport, ovrRenderer *renderer,
               bool loading);

#endif //ALVRCLIENT_RENDER_H
