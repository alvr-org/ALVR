#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>
#include <memory>

#include "render.h"
#include "utils.h"
#include "gltf_model.h"
#include <glm/gtc/type_ptr.hpp>
#include <glm/mat4x4.hpp>

using namespace gl_render_utils;

const float NEAR = 0.1;

Render_EGL egl;

enum VertexAttributeLocation {
    VERTEX_ATTRIBUTE_LOCATION_POSITION,
    VERTEX_ATTRIBUTE_LOCATION_COLOR,
    VERTEX_ATTRIBUTE_LOCATION_UV,
    VERTEX_ATTRIBUTE_LOCATION_TRANSFORM,
    VERTEX_ATTRIBUTE_LOCATION_NORMAL
};

/*
================================================================================

OpenGL-ES Utility Functions

================================================================================
*/

static const char *EglErrorString(const EGLint error) {
    switch (error) {
        case EGL_SUCCESS:
            return "EGL_SUCCESS";
        case EGL_NOT_INITIALIZED:
            return "EGL_NOT_INITIALIZED";
        case EGL_BAD_ACCESS:
            return "EGL_BAD_ACCESS";
        case EGL_BAD_ALLOC:
            return "EGL_BAD_ALLOC";
        case EGL_BAD_ATTRIBUTE:
            return "EGL_BAD_ATTRIBUTE";
        case EGL_BAD_CONTEXT:
            return "EGL_BAD_CONTEXT";
        case EGL_BAD_CONFIG:
            return "EGL_BAD_CONFIG";
        case EGL_BAD_CURRENT_SURFACE:
            return "EGL_BAD_CURRENT_SURFACE";
        case EGL_BAD_DISPLAY:
            return "EGL_BAD_DISPLAY";
        case EGL_BAD_SURFACE:
            return "EGL_BAD_SURFACE";
        case EGL_BAD_MATCH:
            return "EGL_BAD_MATCH";
        case EGL_BAD_PARAMETER:
            return "EGL_BAD_PARAMETER";
        case EGL_BAD_NATIVE_PIXMAP:
            return "EGL_BAD_NATIVE_PIXMAP";
        case EGL_BAD_NATIVE_WINDOW:
            return "EGL_BAD_NATIVE_WINDOW";
        case EGL_CONTEXT_LOST:
            return "EGL_CONTEXT_LOST";
        default:
            return "unknown";
    }
}

static const char VERTEX_SHADER[] = R"glsl(
#ifndef DISABLE_MULTIVIEW
    #define DISABLE_MULTIVIEW 0
#endif
#define NUM_VIEWS 2
#if defined( GL_OVR_multiview2 ) && ! DISABLE_MULTIVIEW
    #extension GL_OVR_multiview2 : enable
    layout(num_views=NUM_VIEWS) in;
    #define VIEW_ID gl_ViewID_OVR
#else
    uniform lowp int ViewID;
    #define VIEW_ID ViewID
#endif
in vec3 vertexPosition;
in vec4 vertexColor;
in mat4 vertexTransform;
in vec2 vertexUv;
uniform mat4 mvpMatrix[NUM_VIEWS];
out vec4 fragmentColor;
out vec2 uv;
void main()
{
    gl_Position = mvpMatrix[VIEW_ID] * vec4( vertexPosition, 1.0 );
    if(uint(VIEW_ID) == uint(0)){
        uv = vec2(vertexUv.x, vertexUv.y);
    }else{
        uv = vec2(vertexUv.x + 0.5, vertexUv.y);
    }
    fragmentColor = vertexColor;
}
)glsl";


static const char FRAGMENT_SHADER[] = R"glsl(
#extension GL_OES_EGL_image_external_essl3 : enable
#extension GL_OES_EGL_image_external : enable
in lowp vec2 uv;
in lowp vec4 fragmentColor;
out lowp vec4 outColor;
uniform %s Texture0;
void main()
{
    outColor = texture(Texture0, uv);
}
)glsl";

static const char VERTEX_SHADER_LOADING[] = R"glsl(
#ifndef DISABLE_MULTIVIEW
    #define DISABLE_MULTIVIEW 0
#endif
#define NUM_VIEWS 2
#if defined( GL_OVR_multiview2 ) && ! DISABLE_MULTIVIEW
    #extension GL_OVR_multiview2 : enable
    layout(num_views=NUM_VIEWS) in;
    #define VIEW_ID gl_ViewID_OVR
#else
    uniform lowp int ViewID;
    #define VIEW_ID ViewID
#endif
in vec3 vertexPosition;
in vec4 vertexColor;
in mat4 vertexTransform;
in vec2 vertexUv;
in vec3 vertexNormal;
uniform mat4 mvpMatrix[NUM_VIEWS];
uniform lowp vec4 Color;
uniform mat4 mMatrix;
out vec4 fragmentColor;
out vec2 uv;
out lowp float fragmentLight;
out lowp vec3 lightPoint;
out lowp vec3 normal;
out lowp vec3 position;
void main()
{
    lowp vec4 position4 = mMatrix * vec4( vertexPosition, 1.0 );
    gl_Position = mvpMatrix[VIEW_ID] * position4;
    uv = vertexUv;
    position = position4.xyz / position4.w;
    lowp vec4 lightPoint4 = mvpMatrix[VIEW_ID] * vec4(100.0, 10000.0, 100.0, 1.0);
    lightPoint = lightPoint4.xyz / lightPoint4.w;
    normal = normalize((mvpMatrix[VIEW_ID] * mMatrix * vec4(vertexNormal, 1.0)).xyz);
    lowp float light = clamp(dot(normal, normalize(vec3(0.3, 1.0, 0.3))), 0.3, 1.0);
    fragmentLight = light;
    fragmentColor = Color;
}
)glsl";

static const char FRAGMENT_SHADER_LOADING[] = R"glsl(
in lowp vec2 uv;
in lowp vec4 fragmentColor;
in lowp float fragmentLight;
in lowp vec3 lightPoint;
in lowp vec3 normal;
in lowp vec3 position;
out lowp vec4 outColor;
uniform sampler2D sTexture;
uniform lowp int Mode;
void main()
{
    if(Mode == 0){                                      // ground
        vec3 groundCenter = vec3(1.0, 1.0, 1.0);
        vec3 groundHorizon = vec3(1.0, 1.0, 1.0);

        lowp float distance = length(position.xz);
        // Pick a coordinate to visualize in a grid
        lowp vec2 coord = position.xz / 2.0;
        // Compute anti-aliased world-space grid lines
        lowp vec2 grid = abs(fract(coord - 0.5) - 0.5) / fwidth(coord);
        lowp float line = min(grid.x, grid.y);
        outColor.rgb = vec3(min(line, 1.0) * (1.0 - exp(-distance / 5.0 - 0.01) / 4.0)) * groundCenter;
        if(distance > 3.0){
            lowp float coef = 1.0 - 3.0 / distance;
            outColor.rgb = (1.0 - coef) * outColor.rgb + coef * groundHorizon;
        }
        outColor.a = 1.0;
    } else if(Mode == 1) {                             // text
        vec3 textColor = vec3(0.0, 0.0, 0.0);

        outColor.rgb = textColor;
        outColor.a = texture(sTexture, uv).a;
    } else {                                           // sky
        vec3 skyCenter = vec3(0.8, 0.8, 1.0);
        vec3 skyHorizon = vec3(1.0, 1.0, 1.0);

        lowp float coef = 1.0;
        if(position.y < 50.0){
            coef = position.y / 100.0;
        }else if(position.y < 100.0){
            coef = (position.y - 50.0) / 50.0 * 0.3 + 0.5;
        }else{
            coef = (position.y - 100.0) / 150.0 * 0.2 + 0.8;
        }
        outColor.a = 1.0;
        outColor.rgb = skyCenter * coef + skyHorizon * (1.0 - coef);
    }
    %s
}
)glsl";

void eglInit() {
    EGLint major, minor;

    egl.Display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    eglInitialize(egl.Display, &major, &minor);

// Do NOT use eglChooseConfig, because the Android EGL code pushes in multisample
    // flags in eglChooseConfig if the user has selected the "force 4x MSAA" option in
    // settings, and that is completely wasted for our warp target.
    const int MAX_CONFIGS = 1024;
    EGLConfig configs[MAX_CONFIGS];
    EGLint numConfigs = 0;
    if (eglGetConfigs(egl.Display, configs, MAX_CONFIGS, &numConfigs) == EGL_FALSE) {
        LOGE("        eglGetConfigs() failed: %s", EglErrorString(eglGetError()));
        return;
    }
    const EGLint configAttribs[] =
            {
                    EGL_RED_SIZE, 8,
                    EGL_GREEN_SIZE, 8,
                    EGL_BLUE_SIZE, 8,
                    EGL_ALPHA_SIZE, 8, // need alpha for the multi-pass timewarp compositor
                    EGL_DEPTH_SIZE, 0,
                    EGL_STENCIL_SIZE, 0,
                    EGL_SAMPLES, 0,
                    EGL_NONE
            };
    egl.Config = 0;
    for (int i = 0; i < numConfigs; i++) {
        EGLint value = 0;

        eglGetConfigAttrib(egl.Display, configs[i], EGL_RENDERABLE_TYPE, &value);
        if ((value & EGL_OPENGL_ES3_BIT_KHR) != EGL_OPENGL_ES3_BIT_KHR) {
            continue;
        }

        // The pbuffer config also needs to be compatible with normal window rendering
        // so it can share textures with the window context.
        eglGetConfigAttrib(egl.Display, configs[i], EGL_SURFACE_TYPE, &value);
        if ((value & (EGL_WINDOW_BIT | EGL_PBUFFER_BIT)) != (EGL_WINDOW_BIT | EGL_PBUFFER_BIT)) {
            continue;
        }

        int j = 0;
        for (; configAttribs[j] != EGL_NONE; j += 2) {
            eglGetConfigAttrib(egl.Display, configs[i], configAttribs[j], &value);
            if (value != configAttribs[j + 1]) {
                break;
            }
        }
        if (configAttribs[j] == EGL_NONE) {
            egl.Config = configs[i];
            break;
        }
    }
    if (egl.Config == 0) {
        LOGE("        eglChooseConfig() failed: %s", EglErrorString(eglGetError()));
        return;
    }
    EGLint contextAttribs[] =
            {
                    EGL_CONTEXT_CLIENT_VERSION, 3,
                    EGL_NONE
            };
    LOG("        Context = eglCreateContext( Display, Config, EGL_NO_CONTEXT, contextAttribs )");
    egl.Context = eglCreateContext(egl.Display, egl.Config, EGL_NO_CONTEXT, contextAttribs);
    if (egl.Context == EGL_NO_CONTEXT) {
        LOGE("        eglCreateContext() failed: %s", EglErrorString(eglGetError()));
        return;
    }
    const EGLint surfaceAttribs[] =
            {
                    EGL_WIDTH, 16,
                    EGL_HEIGHT, 16,
                    EGL_NONE
            };
    LOG("        TinySurface = eglCreatePbufferSurface( Display, Config, surfaceAttribs )");
    egl.TinySurface = eglCreatePbufferSurface(egl.Display, egl.Config, surfaceAttribs);
    if (egl.TinySurface == EGL_NO_SURFACE) {
        LOGE("        eglCreatePbufferSurface() failed: %s", EglErrorString(eglGetError()));
        eglDestroyContext(egl.Display, egl.Context);
        egl.Context = EGL_NO_CONTEXT;
        return;
    }
    LOG("        eglMakeCurrent( Display, TinySurface, TinySurface, Context )");
    if (eglMakeCurrent(egl.Display, egl.TinySurface, egl.TinySurface, egl.Context) == EGL_FALSE) {
        LOGE("        eglMakeCurrent() failed: %s", EglErrorString(eglGetError()));
        eglDestroySurface(egl.Display, egl.TinySurface);
        eglDestroyContext(egl.Display, egl.Context);
        egl.Context = EGL_NO_CONTEXT;
        return;
    }
}

void eglDestroy() {
    if (egl.Display != 0) {
        LOGE("        eglMakeCurrent( Display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT )");
        if (eglMakeCurrent(egl.Display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT) ==
            EGL_FALSE) {
            LOGE("        eglMakeCurrent() failed: %s", EglErrorString(eglGetError()));
        }
    }
    if (egl.Context != EGL_NO_CONTEXT) {
        LOGE("        eglDestroyContext( Display, Context )");
        if (eglDestroyContext(egl.Display, egl.Context) == EGL_FALSE) {
            LOGE("        eglDestroyContext() failed: %s", EglErrorString(eglGetError()));
        }
        egl.Context = EGL_NO_CONTEXT;
    }
    if (egl.TinySurface != EGL_NO_SURFACE) {
        LOGE("        eglDestroySurface( Display, TinySurface )");
        if (eglDestroySurface(egl.Display, egl.TinySurface) == EGL_FALSE) {
            LOGE("        eglDestroySurface() failed: %s", EglErrorString(eglGetError()));
        }
        egl.TinySurface = EGL_NO_SURFACE;
    }
    if (egl.Display != 0) {
        LOGE("        eglTerminate( Display )");
        if (eglTerminate(egl.Display) == EGL_FALSE) {
            LOGE("        eglTerminate() failed: %s", EglErrorString(eglGetError()));
        }
        egl.Display = 0;
    }
}

void ovrFramebuffer_Create(ovrFramebuffer *frameBuffer, std::vector<GLuint> textures, const int width,
                           const int height) {
    for (int i = 0; i < textures.size(); i++) {
        auto glRenderTarget = textures[i];
        frameBuffer->renderTargets.push_back(std::make_unique<gl_render_utils::Texture>(
                glRenderTarget, false, width, height));
        frameBuffer->renderStates.push_back(std::make_unique<gl_render_utils::RenderState>(
                frameBuffer->renderTargets[i].get()));
    }
}

void ovrFramebuffer_Destroy(ovrFramebuffer *frameBuffer) {
    frameBuffer->renderStates.clear();
    frameBuffer->renderTargets.clear();
}

void ovrFramebuffer_SetCurrent(ovrFramebuffer *frameBuffer, int index) {
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, frameBuffer->renderStates[index]->GetFrameBuffer()));
}

void ovrFramebuffer_SetNone() {
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0));
}

void ovrFramebuffer_Resolve() {
    // Discard the depth buffer, so the tiler won't need to write it back out to memory.
    const GLenum depthAttachment[1] = {GL_DEPTH_ATTACHMENT};
    glInvalidateFramebuffer(GL_DRAW_FRAMEBUFFER, 1, depthAttachment);

    // Flush this frame worth of commands.
    glFlush();
}

//
// ovrGeometry
//

typedef struct {
    enum VertexAttributeLocation location;
    const char *name;
} ovrVertexAttribute;

ovrVertexAttribute ProgramVertexAttributes[] =
        {
                {VERTEX_ATTRIBUTE_LOCATION_POSITION,  "vertexPosition"},
                {VERTEX_ATTRIBUTE_LOCATION_COLOR,     "vertexColor"},
                {VERTEX_ATTRIBUTE_LOCATION_UV,        "vertexUv"},
                {VERTEX_ATTRIBUTE_LOCATION_TRANSFORM, "vertexTransform"},
                {VERTEX_ATTRIBUTE_LOCATION_NORMAL,    "vertexNormal"}
        };

void ovrGeometry_Clear(ovrGeometry *geometry) {
    geometry->VertexBuffer = 0;
    geometry->IndexBuffer = 0;
    geometry->VertexArrayObject = 0;
    geometry->VertexCount = 0;
    geometry->IndexCount = 0;
    for (int i = 0; i < MAX_VERTEX_ATTRIB_POINTERS; i++) {
        memset(&geometry->VertexAttribs[i], 0, sizeof(geometry->VertexAttribs[i]));
        geometry->VertexAttribs[i].Index = -1;
    }
}

void ovrGeometry_CreatePanel(ovrGeometry *geometry) {
    typedef struct {
        float positions[4][4];
        float uv[4][2];
    } ovrCubeVertices;

    static const ovrCubeVertices cubeVertices =
            {
                    // positions
                    {
                            {-1, -1, 0, 1}, {1,   1, 0, 1}, {1,   -1, 0, 1}, {-1, 1, 0, 1}
                    },
                    // uv
                    {       {0,  1},        {0.5, 0},       {0.5, 1},        {0,  0}}
            };

    static const unsigned short cubeIndices[6] =
            {
                    0, 2, 1, 0, 1, 3,
            };


    geometry->VertexCount = 4;
    geometry->IndexCount = 6;

    geometry->VertexAttribs[0].Index = VERTEX_ATTRIBUTE_LOCATION_POSITION;
    geometry->VertexAttribs[0].Size = 4;
    geometry->VertexAttribs[0].Type = GL_FLOAT;
    geometry->VertexAttribs[0].Normalized = true;
    geometry->VertexAttribs[0].Stride = sizeof(cubeVertices.positions[0]);
    geometry->VertexAttribs[0].Pointer = (const GLvoid *) offsetof(ovrCubeVertices, positions);

    geometry->VertexAttribs[1].Index = VERTEX_ATTRIBUTE_LOCATION_UV;
    geometry->VertexAttribs[1].Size = 2;
    geometry->VertexAttribs[1].Type = GL_FLOAT;
    geometry->VertexAttribs[1].Normalized = true;
    geometry->VertexAttribs[1].Stride = 8;
    geometry->VertexAttribs[1].Pointer = (const GLvoid *) offsetof(ovrCubeVertices, uv);

    geometry->VertexAttribs[2].Index = -1;
    geometry->VertexAttribs[3].Index = -1;
    geometry->VertexAttribs[4].Index = -1;

    GL(glGenBuffers(1, &geometry->VertexBuffer));
    GL(glBindBuffer(GL_ARRAY_BUFFER, geometry->VertexBuffer));
    GL(glBufferData(GL_ARRAY_BUFFER, sizeof(cubeVertices), &cubeVertices, GL_STATIC_DRAW));
    GL(glBindBuffer(GL_ARRAY_BUFFER, 0));

    GL(glGenBuffers(1, &geometry->IndexBuffer));
    GL(glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, geometry->IndexBuffer));
    GL(glBufferData(GL_ELEMENT_ARRAY_BUFFER, sizeof(cubeIndices), cubeIndices, GL_STATIC_DRAW));
    GL(glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, 0));
}

void ovrGeometry_Destroy(ovrGeometry *geometry) {
    GL(glDeleteBuffers(1, &geometry->IndexBuffer));
    GL(glDeleteBuffers(1, &geometry->VertexBuffer));

    ovrGeometry_Clear(geometry);
}

void ovrGeometry_CreateVAO(ovrGeometry *geometry) {
    GL(glGenVertexArrays(1, &geometry->VertexArrayObject));
    GL(glBindVertexArray(geometry->VertexArrayObject));

    GL(glBindBuffer(GL_ARRAY_BUFFER, geometry->VertexBuffer));

    for (int i = 0; i < MAX_VERTEX_ATTRIB_POINTERS; i++) {
        if ((int)geometry->VertexAttribs[i].Index != -1) {
            GL(glEnableVertexAttribArray(geometry->VertexAttribs[i].Index));
            GL(glVertexAttribPointer(geometry->VertexAttribs[i].Index,
                                     geometry->VertexAttribs[i].Size,
                                     geometry->VertexAttribs[i].Type,
                                     geometry->VertexAttribs[i].Normalized,
                                     geometry->VertexAttribs[i].Stride,
                                     geometry->VertexAttribs[i].Pointer));
        }
    }

    GL(glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, geometry->IndexBuffer));

    GL(glBindVertexArray(0));
}

void ovrGeometry_DestroyVAO(ovrGeometry *geometry) {
    GL(glDeleteVertexArrays(1, &geometry->VertexArrayObject));
}

//
// ovrProgram
//

enum E1test {
    UNIFORM_VIEW_ID,
    UNIFORM_MVP_MATRIX,
    UNIFORM_ALPHA,
    UNIFORM_COLOR,
    UNIFORM_M_MATRIX,
    UNIFORM_MODE
};
enum E2test {
    UNIFORM_TYPE_VECTOR4,
    UNIFORM_TYPE_MATRIX4X4,
    UNIFORM_TYPE_INT,
    UNIFORM_TYPE_BUFFER,
    UNIFORM_TYPE_FLOAT,
};
typedef struct {
    E1test index;
    E2test type;
    const char *name;
} ovrUniform;

static ovrUniform ProgramUniforms[] =
        {
                {UNIFORM_VIEW_ID,    UNIFORM_TYPE_INT,       "ViewID"},
                {UNIFORM_MVP_MATRIX, UNIFORM_TYPE_MATRIX4X4, "mvpMatrix"},
                {UNIFORM_ALPHA,      UNIFORM_TYPE_FLOAT,     "alpha"},
                {UNIFORM_COLOR,      UNIFORM_TYPE_VECTOR4,   "Color"},
                {UNIFORM_M_MATRIX,   UNIFORM_TYPE_MATRIX4X4, "mMatrix"},
                {UNIFORM_MODE,       UNIFORM_TYPE_INT,       "Mode"},
        };

static const char *programVersion = "#version 300 es\n";

bool
ovrProgram_Create(ovrProgram *program, const char *vertexSource, const char *fragmentSource) {
    GLint r;

    LOGI("Compiling shaders.");
    GL(program->VertexShader = glCreateShader(GL_VERTEX_SHADER));
    if (program->VertexShader == 0) {
        LOGE("glCreateShader error: %d", glGetError());
        return false;
    }

    const char *vertexSources[3] = {programVersion, "#define DISABLE_MULTIVIEW 1\n", vertexSource};
    GL(glShaderSource(program->VertexShader, 3, vertexSources, 0));
    GL(glCompileShader(program->VertexShader));
    GL(glGetShaderiv(program->VertexShader, GL_COMPILE_STATUS, &r));
    if (r == GL_FALSE) {
        GLchar msg[4096];
        GL(glGetShaderInfoLog(program->VertexShader, sizeof(msg), 0, msg));
        LOGE("Error on compiling vertex shader. Message=%s", msg);
        LOGE("%s\n%s\n", vertexSource, msg);
        // Ignore compile error. If this error is only a warning, we can proceed to next.
    }

    const char *fragmentSources[2] = {programVersion, fragmentSource};
    GL(program->FragmentShader = glCreateShader(GL_FRAGMENT_SHADER));
    GL(glShaderSource(program->FragmentShader, 2, fragmentSources, 0));
    GL(glCompileShader(program->FragmentShader));
    GL(glGetShaderiv(program->FragmentShader, GL_COMPILE_STATUS, &r));
    if (r == GL_FALSE) {
        GLchar msg[4096];
        GL(glGetShaderInfoLog(program->FragmentShader, sizeof(msg), 0, msg));
        LOGE("Error on compiling fragment shader. Message=%s", msg);
        LOGE("%s\n%s\n", fragmentSource, msg);
        // Ignore compile error. If this error is only a warning, we can proceed to next.
    }

    GL(program->Program = glCreateProgram());
    GL(glAttachShader(program->Program, program->VertexShader));
    GL(glAttachShader(program->Program, program->FragmentShader));

    // Bind the vertex attribute locations.
    for (size_t i = 0; i < sizeof(ProgramVertexAttributes) / sizeof(ProgramVertexAttributes[0]); i++) {
        GL(glBindAttribLocation(program->Program, ProgramVertexAttributes[i].location,
                                ProgramVertexAttributes[i].name));
    }

    GL(glLinkProgram(program->Program));
    GL(glGetProgramiv(program->Program, GL_LINK_STATUS, &r));
    if (r == GL_FALSE) {
        GLchar msg[4096];
        GL(glGetProgramInfoLog(program->Program, sizeof(msg), 0, msg));
        LOGE("Linking program failed: %s\n", msg);
        return false;
    }

    int numBufferBindings = 0;

    // Get the uniform locations.
    memset(program->UniformLocation, -1, sizeof(program->UniformLocation));
    for (unsigned long i = 0; i < sizeof(ProgramUniforms) / sizeof(ProgramUniforms[0]); i++) {
        const int uniformIndex = ProgramUniforms[i].index;
        if (ProgramUniforms[i].type == UNIFORM_TYPE_BUFFER) {
            GL(program->UniformLocation[uniformIndex] = glGetUniformBlockIndex(program->Program,
                                                                               ProgramUniforms[i].name));
            program->UniformBinding[uniformIndex] = numBufferBindings++;
            GL(glUniformBlockBinding(program->Program, program->UniformLocation[uniformIndex],
                                     program->UniformBinding[uniformIndex]));
        } else {
            GL(program->UniformLocation[uniformIndex] = glGetUniformLocation(program->Program,
                                                                             ProgramUniforms[i].name));
            program->UniformBinding[uniformIndex] = program->UniformLocation[uniformIndex];
        }
    }

    GL(glUseProgram(program->Program));

    // Get the texture locations.
    for (int i = 0; i < MAX_PROGRAM_TEXTURES; i++) {
        char name[32];
        sprintf(name, "Texture%i", i);
        program->Textures[i] = glGetUniformLocation(program->Program, name);
        if (program->Textures[i] != -1) {
            GL(glUniform1i(program->Textures[i], i));
        }
    }

    GL(glUseProgram(0));

    LOGI("Successfully compiled shader.");
    return true;
}

void ovrProgram_Destroy(ovrProgram *program) {
    if (program->Program != 0) {
        GL(glDeleteProgram(program->Program));
        program->Program = 0;
    }
    if (program->VertexShader != 0) {
        GL(glDeleteShader(program->VertexShader));
        program->VertexShader = 0;
    }
    if (program->FragmentShader != 0) {
        GL(glDeleteShader(program->FragmentShader));
        program->FragmentShader = 0;
    }
}

//
// ovrRenderer
//

void ovrRenderer_Create(ovrRenderer *renderer, int width, int height, Texture *streamTexture,
                        int LoadingTexture, std::vector<GLuint> textures[2], bool darkMode,
                        FFRData ffrData) {
    renderer->enableFFR = ffrData.enabled;
    if (renderer->enableFFR) {
        renderer->ffr = std::make_unique<FFR>(streamTexture);
        renderer->ffr->Initialize(ffrData);
    }

    // Create the frame buffers.
    for (int eye = 0; eye < 2; eye++) {
        ovrFramebuffer_Create(&renderer->FrameBuffer[eye], textures[eye], width, height);
    }

    renderer->streamTexture = streamTexture;
    renderer->LoadingTexture = LoadingTexture;
    renderer->SceneCreated = false;
    renderer->loadingScene = new GltfModel();
    renderer->loadingScene->load();

    std::string fragment_shader;
    fragment_shader = string_format(FRAGMENT_SHADER,
                                    renderer->enableFFR ? "sampler2D" : "samplerExternalOES");
    ovrProgram_Create(&renderer->Program, VERTEX_SHADER, fragment_shader.c_str());

    fragment_shader = string_format(FRAGMENT_SHADER_LOADING,
                                    darkMode ? "outColor.rgb = 1.0 - outColor.rgb;" : "");
    ovrProgram_Create(&renderer->ProgramLoading, VERTEX_SHADER_LOADING, fragment_shader.c_str());

    ovrGeometry_CreatePanel(&renderer->Panel);
    ovrGeometry_CreateVAO(&renderer->Panel);
    renderer->SceneCreated = true;
}

void ovrRenderer_Destroy(ovrRenderer *renderer) {
    ovrProgram_Destroy(&renderer->Program);
    ovrProgram_Destroy(&renderer->ProgramLoading);
    ovrGeometry_DestroyVAO(&renderer->Panel);
    ovrGeometry_Destroy(&renderer->Panel);

    for (int eye = 0; eye < 2; eye++) {
        ovrFramebuffer_Destroy(&renderer->FrameBuffer[eye]);
    }
}

void renderEye(int eye, glm::mat4 mvpMatrix[2], Recti *viewport, ovrRenderer *renderer,
               bool loading) {
    if (loading) {
        GL(glUseProgram(renderer->ProgramLoading.Program));
        if (renderer->ProgramLoading.UniformLocation[UNIFORM_VIEW_ID] >=
            0)  // NOTE: will not be present when multiview path is enabled.
        {
            GL(glUniform1i(renderer->ProgramLoading.UniformLocation[UNIFORM_VIEW_ID], eye));
        }
    } else {
        GL(glUseProgram(renderer->Program.Program));
        if (renderer->Program.UniformLocation[UNIFORM_VIEW_ID] >=
            0)  // NOTE: will not be present when multiview path is enabled.
        {
            GL(glUniform1i(renderer->Program.UniformLocation[UNIFORM_VIEW_ID], eye));
        }
    }
    GL(glEnable(GL_SCISSOR_TEST));
    GL(glDepthMask(GL_TRUE));
    GL(glEnable(GL_DEPTH_TEST));
    GL(glDepthFunc(GL_LEQUAL));
    GL(glEnable(GL_CULL_FACE));
    GL(glCullFace(GL_BACK));
    GL(glViewport(viewport->x, viewport->y, viewport->width, viewport->height));
    GL(glScissor(viewport->x, viewport->y, viewport->width, viewport->height));

    if (loading) {
        // For drawing back frace of the sphere in gltf
        GL(glDisable(GL_CULL_FACE));
        GL(glClearColor(0.88f, 0.95f, 0.95f, 1.0f));
        GL(glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT));

        GL(glEnable(GL_BLEND));
        GL(glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA));

        //LOG("mm:\n%s", DumpMatrix(&mvpMatrix[0]).c_str());
        GL(glUniformMatrix4fv(renderer->ProgramLoading.UniformLocation[UNIFORM_MVP_MATRIX], 2, true,
                              (float *) mvpMatrix));
        GL(glActiveTexture(GL_TEXTURE0));

        GL(glBindTexture(GL_TEXTURE_2D, renderer->LoadingTexture));
        renderer->loadingScene->drawScene(VERTEX_ATTRIBUTE_LOCATION_POSITION,
                                          VERTEX_ATTRIBUTE_LOCATION_UV,
                                          VERTEX_ATTRIBUTE_LOCATION_NORMAL,
                                          renderer->ProgramLoading.UniformLocation[UNIFORM_COLOR],
                                          renderer->ProgramLoading.UniformLocation[UNIFORM_M_MATRIX],
                                          renderer->ProgramLoading.UniformLocation[UNIFORM_MODE]);
        GL(glBindVertexArray(0));
        GL(glBindTexture(GL_TEXTURE_2D, 0));
    } else {
        GL(glClear(GL_DEPTH_BUFFER_BIT));

        glm::mat4 mvpMatrix[2];
        mvpMatrix[0] = glm::mat4(1.0);
        mvpMatrix[1] = glm::mat4(1.0);

        GL(glBindVertexArray(renderer->Panel.VertexArrayObject));

        GL(glUniformMatrix4fv(renderer->Program.UniformLocation[UNIFORM_MVP_MATRIX], 2, true,
                              (float *) mvpMatrix));

        GL(glUniform1f(renderer->Program.UniformLocation[UNIFORM_ALPHA], 2.0f));
        GL(glActiveTexture(GL_TEXTURE0));
        if (renderer->enableFFR) {
            GL(glBindTexture(GL_TEXTURE_2D,
                             renderer->ffr->GetOutputTexture()->GetGLTexture()));
        } else {
            GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, renderer->streamTexture->GetGLTexture()));
        }

        GL(glDrawElements(GL_TRIANGLES, renderer->Panel.IndexCount, GL_UNSIGNED_SHORT, NULL));

        GL(glBindVertexArray(0));

        GL(glActiveTexture(GL_TEXTURE0));
        GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0));
        GL(glActiveTexture(GL_TEXTURE1));
        GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0));
    }

    GL(glUseProgram(0));
}

void ovrRenderer_RenderFrame(ovrRenderer *renderer, EyeInput input[2], int swapchainIndex[2], bool loading) {
    if (renderer->enableFFR) {
        renderer->ffr->Render();
    }

    glm::mat4 mvpMatrix[2];
    for (int eye = 0; eye < 2; eye++) {
        auto trans = glm::translate(glm::mat4(1.0), input[eye].position);
        auto rot = glm::mat4_cast(input[eye].orientation);
        auto viewInv = glm::inverse(trans * rot);

        auto tanl = tan(input[eye].fov.left);
        auto tanr = tan(input[eye].fov.right);
        auto tant = tan(-input[eye].fov.top);
        auto tanb = tan(-input[eye].fov.bottom);
        auto a = 2 / (tanr - tanl);
        auto b = 2 / (tanb - tant);
        auto c = (tanr + tanl) / (tanr - tanl);
        auto d = (tanb + tant) / (tanb - tant);
        auto proj = glm::mat4(a, 0.f, c,    0.f,
                            0.f, b,   d,    0.f,
                            0.f, 0.f, -1.f, -2 * NEAR,
                            0.f, 0.f, -1.f, 0.f);
        proj = glm::transpose(proj);

        mvpMatrix[eye] = glm::transpose(proj * viewInv);
    }

    // Render the eye images.
    for (int eye = 0; eye < 2; eye++) {
        ovrFramebuffer *frameBuffer = &renderer->FrameBuffer[eye];
        ovrFramebuffer_SetCurrent(frameBuffer, swapchainIndex[eye]);

        Recti viewport = {0, 0, (int) frameBuffer->renderTargets[0]->GetWidth(),
                          (int) frameBuffer->renderTargets[0]->GetHeight()};

        renderEye(eye, mvpMatrix, &viewport, renderer, loading);

        ovrFramebuffer_Resolve();
    }

    ovrFramebuffer_SetNone();
}
