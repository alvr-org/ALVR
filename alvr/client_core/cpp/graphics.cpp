#include "bindings.h"
#include "ffr.h"
#include "gltf_model.h"
#include "srgb_correction_pass.h"
#include "utils.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <glm/gtc/quaternion.hpp>
#include <mutex>

using namespace gl_render_utils;

const float NEAR = 0.1;
const int MAX_VERTEX_ATTRIB_POINTERS = 5;
const int MAX_PROGRAM_UNIFORMS = 8;
const int MAX_PROGRAM_TEXTURES = 8;
const int HUD_TEXTURE_WIDTH = 1280;
const int HUD_TEXTURE_HEIGHT = 720;

/// Integer version of ovrRectf
typedef struct Recti_ {
    int x;
    int y;
    int width;
    int height;
} Recti;

typedef struct {
    std::vector<std::unique_ptr<gl_render_utils::Texture>> renderTargets;
    std::vector<std::unique_ptr<gl_render_utils::RenderState>> renderStates;
} ovrFramebuffer;

typedef struct {
    GLuint Index;
    GLint Size;
    GLenum Type;
    GLboolean Normalized;
    GLsizei Stride;
    const GLvoid *Pointer;
} ovrVertexAttribPointer;

typedef struct {
    GLuint VertexBuffer;
    GLuint IndexBuffer;
    GLuint VertexArrayObject;
    GLuint VertexUVBuffer;
    int VertexCount;
    int IndexCount;
    ovrVertexAttribPointer VertexAttribs[MAX_VERTEX_ATTRIB_POINTERS];
} ovrGeometry;

typedef struct {
    GLuint streamProgram;
    GLuint VertexShader;
    GLuint FragmentShader;
    // These will be -1 if not used by the program.
    GLint UniformLocation[MAX_PROGRAM_UNIFORMS]; // ProgramUniforms[].name
    GLint UniformBinding[MAX_PROGRAM_UNIFORMS];  // ProgramUniforms[].name
    GLint Textures[MAX_PROGRAM_TEXTURES];        // Texture%i
} ovrProgram;

enum ovrProgramType {
    STREAMER_PROG,
    LOBBY_PROG,
    MAX_PROGS // Not to be used as a type, just a placeholder for len
};

typedef struct {
    ovrFramebuffer FrameBuffer[2];
    bool SceneCreated;
    ovrProgram streamProgram;
    ovrProgram lobbyProgram;
    ovrGeometry Panel;
    gl_render_utils::Texture *streamTexture;
    GLuint hudTexture;
    GltfModel *lobbyScene;
    std::unique_ptr<FFR> ffr;
    std::unique_ptr<SrgbCorrectionPass> srgbCorrectionPass;
    bool enableFFE;
    GLuint streamRenderTexture;
} ovrRenderer;

enum VertexAttributeLocation {
    VERTEX_ATTRIBUTE_LOCATION_POSITION,
    VERTEX_ATTRIBUTE_LOCATION_COLOR,
    VERTEX_ATTRIBUTE_LOCATION_UV,
    VERTEX_ATTRIBUTE_LOCATION_TRANSFORM,
    VERTEX_ATTRIBUTE_LOCATION_NORMAL
};

typedef struct {
    enum VertexAttributeLocation location;
    const char *name;
    bool usedInProg[MAX_PROGS];
} ovrVertexAttribute;

ovrVertexAttribute ProgramVertexAttributes[] = {
    {VERTEX_ATTRIBUTE_LOCATION_POSITION, "vertexPosition", {true, true}},
    {VERTEX_ATTRIBUTE_LOCATION_COLOR, "vertexColor", {true, false}},
    {VERTEX_ATTRIBUTE_LOCATION_UV, "vertexUv", {true, true}},
    {VERTEX_ATTRIBUTE_LOCATION_TRANSFORM, "vertexTransform", {false, false}},
    {VERTEX_ATTRIBUTE_LOCATION_NORMAL, "vertexNormal", {false, true}}};

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

static ovrUniform ProgramUniforms[] = {
    {UNIFORM_VIEW_ID, UNIFORM_TYPE_INT, "ViewID"},
    {UNIFORM_MVP_MATRIX, UNIFORM_TYPE_MATRIX4X4, "mvpMatrix"},
    {UNIFORM_ALPHA, UNIFORM_TYPE_FLOAT, "alpha"},
    {UNIFORM_COLOR, UNIFORM_TYPE_VECTOR4, "Color"},
    {UNIFORM_M_MATRIX, UNIFORM_TYPE_MATRIX4X4, "mMatrix"},
    {UNIFORM_MODE, UNIFORM_TYPE_INT, "Mode"},
};

class GraphicsContext {
  public:
    EGLDisplay eglDisplay;

    std::vector<uint8_t> hudTextureBitmap;
    std::mutex hudTextureMutex;
    std::unique_ptr<Texture> hudTexture;
    std::vector<GLuint> lobbySwapchainTextures[2];
    std::unique_ptr<ovrRenderer> lobbyRenderer;

    std::unique_ptr<Texture> streamTexture;
    std::vector<GLuint> streamSwapchainTextures[2];
    std::unique_ptr<ovrRenderer> streamRenderer;
};

namespace {
PFNEGLCREATEIMAGEKHRPROC eglCreateImageKHR;
PFNEGLDESTROYIMAGEKHRPROC eglDestroyImageKHR;
PFNEGLGETNATIVECLIENTBUFFERANDROIDPROC eglGetNativeClientBufferANDROID;
PFNGLEGLIMAGETARGETTEXTURE2DOESPROC glEGLImageTargetTexture2DOES;
GraphicsContext g_ctx;
} // namespace

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
in lowp vec2 uv;
in lowp vec4 fragmentColor;
out lowp vec4 outColor;
uniform sampler2D Texture0;
void main()
{
    outColor = texture(Texture0, uv);
}
)glsl";

static const char LOBBY_VERTEX_SHADER[] = R"glsl(
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

static const char LOBBY_FRAGMENT_SHADER[] = R"glsl(
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
        lowp vec3 groundCenter = vec3(0.0, 0.0, 0.00);
        lowp vec3 groundHorizon = vec3(0.00, 0.00, 0.015);

        lowp vec3 gridClose = vec3(0.114, 0.545, 0.804);
        lowp vec3 gridFar = vec3(0.259, 0.863, 0.886);

        lowp float lineFadeStart = 10.0;
        lowp float lineFadeEnd = 50.0;
        lowp float lineFadeDist = lineFadeEnd - lineFadeStart;

        lowp float lineBloom = 10.0;

        lowp float distance = length(position.xz);

        // Pick a coordinate to visualize in a grid
        lowp vec2 coord = position.xz / 2.0;

        // Compute anti-aliased world-space grid lines
        lowp vec2 grid = abs(fract(coord - 0.5) - 0.5) / fwidth(coord);

        // Create mask for grid lines and fade over distance
        lowp float line = clamp(1.0 - min(grid.x, grid.y), 0.0, 1.0);
        line *= clamp((lineFadeStart - distance) / lineFadeDist, 0.0, 1.0);

        // Fill in normal ground colour
        outColor.rgb = groundCenter * (1.0 - line);

        // Add cheap and simple "bloom" to the grid lines
        line *= 1.0 + lineBloom;

        // Fill in grid line colour
        outColor.rgb += line * mix(gridFar, gridClose, clamp((lineFadeEnd - distance) / lineFadeEnd, 0.0, 1.0));

        // Fade to the horizon colour over distance
        if(distance > 10.0){
            lowp float coef = 1.0 - 10.0 / distance;
            outColor.rgb = (1.0 - coef) * outColor.rgb + coef * groundHorizon;
        }

        outColor.a = 1.0;
    } else if(Mode == 1) {                             // text
        lowp vec3 textColor = vec3(1.0, 1.0, 1.0);

        outColor.rgb = textColor;
        outColor.a = texture(sTexture, uv).a;
    } else {                                           // sky
        lowp vec3 skyCenter = vec3(0.0, 0.0, 0.0);
        lowp vec3 skyHorizon = vec3(0.0, 0.0, 0.02);

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
}
)glsl";

void ovrFramebuffer_Create(ovrFramebuffer *frameBuffer,
                           std::vector<GLuint> textures,
                           const int width,
                           const int height) {
    for (int i = 0; i < textures.size(); i++) {
        auto glRenderTarget = textures[i];
        frameBuffer->renderTargets.push_back(std::make_unique<gl_render_utils::Texture>(
            true, glRenderTarget, false, width, height, GL_SRGB8_ALPHA8, GL_RGBA));
        frameBuffer->renderStates.push_back(
            std::make_unique<gl_render_utils::RenderState>(frameBuffer->renderTargets[i].get()));
    }
}

void ovrFramebuffer_Destroy(ovrFramebuffer *frameBuffer) {
    frameBuffer->renderStates.clear();
    frameBuffer->renderTargets.clear();
}

void ovrFramebuffer_SetCurrent(ovrFramebuffer *frameBuffer, int index) {
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, frameBuffer->renderStates[index]->GetFrameBuffer()));
}

void ovrFramebuffer_SetNone() { GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0)); }

void ovrFramebuffer_Resolve() {
    // Discard the depth buffer, so the tiler won't need to write it back out to memory.
    const GLenum depthAttachment[1] = {GL_DEPTH_ATTACHMENT};
    glInvalidateFramebuffer(GL_DRAW_FRAMEBUFFER, 1, depthAttachment);

    // Flush this frame worth of commands.
    glFlush();
}

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

    static const ovrCubeVertices cubeVertices = {
        // positions
        {{-1, -1, 0, 1}, {1, 1, 0, 1}, {1, -1, 0, 1}, {-1, 1, 0, 1}},
        // uv
        {{0, 1}, {0.5, 0}, {0.5, 1}, {0, 0}}};

    static const unsigned short cubeIndices[6] = {
        0,
        2,
        1,
        0,
        1,
        3,
    };

    geometry->VertexCount = 4;
    geometry->IndexCount = 6;

    geometry->VertexAttribs[0].Index = VERTEX_ATTRIBUTE_LOCATION_POSITION;
    geometry->VertexAttribs[0].Size = 4;
    geometry->VertexAttribs[0].Type = GL_FLOAT;
    geometry->VertexAttribs[0].Normalized = true;
    geometry->VertexAttribs[0].Stride = sizeof(cubeVertices.positions[0]);
    geometry->VertexAttribs[0].Pointer = (const GLvoid *)offsetof(ovrCubeVertices, positions);

    geometry->VertexAttribs[1].Index = VERTEX_ATTRIBUTE_LOCATION_UV;
    geometry->VertexAttribs[1].Size = 2;
    geometry->VertexAttribs[1].Type = GL_FLOAT;
    geometry->VertexAttribs[1].Normalized = true;
    geometry->VertexAttribs[1].Stride = 8;
    geometry->VertexAttribs[1].Pointer = (const GLvoid *)offsetof(ovrCubeVertices, uv);

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

static const char *programVersion = "#version 300 es\n";

bool ovrProgram_Create(ovrProgram *program,
                       const char *vertexSource,
                       const char *fragmentSource,
                       ovrProgramType progType) {
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

    GL(program->streamProgram = glCreateProgram());

    // Bind the vertex attribute locations.
    for (size_t i = 0; i < sizeof(ProgramVertexAttributes) / sizeof(ProgramVertexAttributes[0]);
         i++) {
        // Only bind vertex attributes which are used/active in shader else causes uncessary bugs
        // via compiler optimization/aliasing
        if (ProgramVertexAttributes[i].usedInProg[progType]) {
            GL(glBindAttribLocation(program->streamProgram,
                                    ProgramVertexAttributes[i].location,
                                    ProgramVertexAttributes[i].name));
            LOGD("Binding ProgramVertexAttribute [id.%d] %s to location %d",
                 i,
                 ProgramVertexAttributes[i].name,
                 ProgramVertexAttributes[i].location);
        }
    }

    GL(glAttachShader(program->streamProgram, program->VertexShader));
    GL(glAttachShader(program->streamProgram, program->FragmentShader));
    GL(glLinkProgram(program->streamProgram));

    GL(glGetProgramiv(program->streamProgram, GL_LINK_STATUS, &r));
    if (r == GL_FALSE) {
        GLchar msg[4096];
        GL(glGetProgramInfoLog(program->streamProgram, sizeof(msg), 0, msg));
        LOGE("Linking program failed: %s (%s, %d)\n", msg, __FILE__, __LINE__);
        LOGE("vertexSource: %s\n", vertexSource);
        LOGE("fragmentSource: %s\n", fragmentSource);
        return false;
    }

    int numBufferBindings = 0;

    // Get the uniform locations.
    memset(program->UniformLocation, -1, sizeof(program->UniformLocation));
    for (unsigned long i = 0; i < sizeof(ProgramUniforms) / sizeof(ProgramUniforms[0]); i++) {
        const int uniformIndex = ProgramUniforms[i].index;
        if (ProgramUniforms[i].type == UNIFORM_TYPE_BUFFER) {
            GL(program->UniformLocation[uniformIndex] =
                   glGetUniformBlockIndex(program->streamProgram, ProgramUniforms[i].name));
            program->UniformBinding[uniformIndex] = numBufferBindings++;
            GL(glUniformBlockBinding(program->streamProgram,
                                     program->UniformLocation[uniformIndex],
                                     program->UniformBinding[uniformIndex]));
        } else {
            GL(program->UniformLocation[uniformIndex] =
                   glGetUniformLocation(program->streamProgram, ProgramUniforms[i].name));
            program->UniformBinding[uniformIndex] = program->UniformLocation[uniformIndex];
        }
    }

    GL(glUseProgram(program->streamProgram));

    // Get the texture locations.
    for (int i = 0; i < MAX_PROGRAM_TEXTURES; i++) {
        char name[32];
        sprintf(name, "Texture%i", i);
        program->Textures[i] = glGetUniformLocation(program->streamProgram, name);
        if (program->Textures[i] != -1) {
            GL(glUniform1i(program->Textures[i], i));
        }
    }

    GL(glUseProgram(0));

    LOGI("Successfully compiled shader.");
    return true;
}

void ovrProgram_Destroy(ovrProgram *program) {
    if (program->streamProgram != 0) {
        GL(glDeleteProgram(program->streamProgram));
        program->streamProgram = 0;
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

void ovrRenderer_Create(ovrRenderer *renderer,
                        int width,
                        int height,
                        Texture *streamTexture,
                        int hudTexture,
                        std::vector<GLuint> textures[2],
                        FFRData ffrData,
                        bool isLobby,
                        bool enableSrgbCorrection) {
    if (!isLobby) {
        renderer->srgbCorrectionPass = std::make_unique<SrgbCorrectionPass>(streamTexture);
        renderer->enableFFE = ffrData.enabled;
        if (renderer->enableFFE) {
            FoveationVars fv = CalculateFoveationVars(ffrData);
            renderer->srgbCorrectionPass->Initialize(
                fv.optimizedEyeWidth, fv.optimizedEyeHeight, !enableSrgbCorrection);
            renderer->ffr = std::make_unique<FFR>(renderer->srgbCorrectionPass->GetOutputTexture());
            renderer->ffr->Initialize(fv);
            renderer->streamRenderTexture = renderer->ffr->GetOutputTexture()->GetGLTexture();
        } else {
            renderer->srgbCorrectionPass->Initialize(width, height, !enableSrgbCorrection);
            renderer->streamRenderTexture =
                renderer->srgbCorrectionPass->GetOutputTexture()->GetGLTexture();
        }
    }

    // Create the frame buffers.
    for (int eye = 0; eye < 2; eye++) {
        ovrFramebuffer_Create(&renderer->FrameBuffer[eye], textures[eye], width, height);
    }

    renderer->streamTexture = streamTexture;
    renderer->hudTexture = hudTexture;
    renderer->SceneCreated = false;
    renderer->lobbyScene = new GltfModel();
    renderer->lobbyScene->load();

    ovrProgram_Create(&renderer->streamProgram, VERTEX_SHADER, FRAGMENT_SHADER, STREAMER_PROG);

    ovrProgram_Create(
        &renderer->lobbyProgram, LOBBY_VERTEX_SHADER, LOBBY_FRAGMENT_SHADER, LOBBY_PROG);

    ovrGeometry_CreatePanel(&renderer->Panel);
    ovrGeometry_CreateVAO(&renderer->Panel);
    renderer->SceneCreated = true;
}

void ovrRenderer_Destroy(ovrRenderer *renderer) {
    ovrProgram_Destroy(&renderer->streamProgram);
    ovrProgram_Destroy(&renderer->lobbyProgram);
    ovrGeometry_DestroyVAO(&renderer->Panel);
    ovrGeometry_Destroy(&renderer->Panel);

    for (int eye = 0; eye < 2; eye++) {
        ovrFramebuffer_Destroy(&renderer->FrameBuffer[eye]);
    }
}

void renderEye(
    int eye, glm::mat4 mvpMatrix[2], Recti *viewport, ovrRenderer *renderer, bool isLobby) {
    if (isLobby) {
        GL(glUseProgram(renderer->lobbyProgram.streamProgram));
        if (renderer->lobbyProgram.UniformLocation[UNIFORM_VIEW_ID] >=
            0) // NOTE: will not be present when multiview path is enabled.
        {
            GL(glUniform1i(renderer->lobbyProgram.UniformLocation[UNIFORM_VIEW_ID], eye));
        }
    } else {
        GL(glUseProgram(renderer->streamProgram.streamProgram));
        if (renderer->streamProgram.UniformLocation[UNIFORM_VIEW_ID] >=
            0) // NOTE: will not be present when multiview path is enabled.
        {
            GL(glUniform1i(renderer->streamProgram.UniformLocation[UNIFORM_VIEW_ID], eye));
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

    if (isLobby) {
        // For drawing back frace of the sphere in gltf
        GL(glDisable(GL_CULL_FACE));
        GL(glClearColor(0.88f, 0.95f, 0.95f, 1.0f));
        GL(glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT));

        GL(glEnable(GL_BLEND));
        GL(glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA));

        GL(glUniformMatrix4fv(renderer->lobbyProgram.UniformLocation[UNIFORM_MVP_MATRIX],
                              2,
                              true,
                              (float *)mvpMatrix));
        GL(glActiveTexture(GL_TEXTURE0));

        GL(glBindTexture(GL_TEXTURE_2D, renderer->hudTexture));
        renderer->lobbyScene->drawScene(VERTEX_ATTRIBUTE_LOCATION_POSITION,
                                        VERTEX_ATTRIBUTE_LOCATION_UV,
                                        VERTEX_ATTRIBUTE_LOCATION_NORMAL,
                                        renderer->lobbyProgram.UniformLocation[UNIFORM_COLOR],
                                        renderer->lobbyProgram.UniformLocation[UNIFORM_M_MATRIX],
                                        renderer->lobbyProgram.UniformLocation[UNIFORM_MODE]);
        GL(glBindVertexArray(0));
        GL(glBindTexture(GL_TEXTURE_2D, 0));
    } else {
        GL(glClear(GL_DEPTH_BUFFER_BIT));

        GL(glBindVertexArray(renderer->Panel.VertexArrayObject));

        GL(glUniformMatrix4fv(renderer->streamProgram.UniformLocation[UNIFORM_MVP_MATRIX],
                              2,
                              true,
                              (float *)mvpMatrix));

        GL(glUniform1f(renderer->streamProgram.UniformLocation[UNIFORM_ALPHA], 2.0f));
        GL(glActiveTexture(GL_TEXTURE0));
        GL(glBindTexture(GL_TEXTURE_2D, renderer->streamRenderTexture));

        GL(glDrawElements(GL_TRIANGLES, renderer->Panel.IndexCount, GL_UNSIGNED_SHORT, NULL));

        GL(glBindVertexArray(0));

        GL(glActiveTexture(GL_TEXTURE0));
        GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0));
        GL(glActiveTexture(GL_TEXTURE1));
        GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0));
    }

    GL(glUseProgram(0));
}

void ovrRenderer_RenderFrame(ovrRenderer *renderer, const FfiViewInput input[2], bool isLobby) {
    glm::mat4 mvpMatrix[2];
    if (isLobby) {
        for (int eye = 0; eye < 2; eye++) {
            auto p = input[eye].position;
            auto o = input[eye].orientation;
            auto trans = glm::translate(glm::mat4(1.0), glm::vec3(p[0], p[1], p[2]));
            auto rot = glm::mat4_cast(glm::quat(o[3], o[0], o[1], o[2]));
            auto viewInv = glm::inverse(trans * rot);

            auto tanl = tan(input[eye].fovLeft);
            auto tanr = tan(input[eye].fovRight);
            auto tant = tan(input[eye].fovUp);
            auto tanb = tan(input[eye].fovDown);
            auto a = 2 / (tanr - tanl);
            auto b = 2 / (tant - tanb);
            auto c = (tanr + tanl) / (tanr - tanl);
            auto d = (tant + tanb) / (tant - tanb);
            auto proj = glm::mat4(
                a, 0.f, c, 0.f, 0.f, b, d, 0.f, 0.f, 0.f, -1.f, -2 * NEAR, 0.f, 0.f, -1.f, 0.f);
            proj = glm::transpose(proj);

            mvpMatrix[eye] = glm::transpose(proj * viewInv);
        }
    } else {
        mvpMatrix[0] = glm::mat4(1.0);
        mvpMatrix[1] = glm::mat4(1.0);
    }

    // Render the eye images.
    for (int eye = 0; eye < 2; eye++) {
        ovrFramebuffer *frameBuffer = &renderer->FrameBuffer[eye];
        ovrFramebuffer_SetCurrent(frameBuffer, input[eye].swapchainIndex);

        Recti viewport = {0,
                          0,
                          (int)frameBuffer->renderTargets[0]->GetWidth(),
                          (int)frameBuffer->renderTargets[0]->GetHeight()};

        renderEye(eye, mvpMatrix, &viewport, renderer, isLobby);

        ovrFramebuffer_Resolve();
    }

    ovrFramebuffer_SetNone();
}

void initGraphicsNative() {
    g_ctx.eglDisplay = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    eglCreateImageKHR = (PFNEGLCREATEIMAGEKHRPROC)eglGetProcAddress("eglCreateImageKHR");
    eglDestroyImageKHR = (PFNEGLDESTROYIMAGEKHRPROC)eglGetProcAddress("eglDestroyImageKHR");
    eglGetNativeClientBufferANDROID = (PFNEGLGETNATIVECLIENTBUFFERANDROIDPROC)eglGetProcAddress(
        "eglGetNativeClientBufferANDROID");
    glEGLImageTargetTexture2DOES =
        (PFNGLEGLIMAGETARGETTEXTURE2DOESPROC)eglGetProcAddress("glEGLImageTargetTexture2DOES");

    g_ctx.streamTexture = std::make_unique<Texture>(false, 0, true);
    g_ctx.hudTexture = std::make_unique<Texture>(
        false, 0, false, 1280, 720, GL_RGBA8, GL_RGBA, std::vector<uint8_t>(1280 * 720 * 4, 0));

    const GLubyte *sVendor, *sRenderer, *sVersion, *sExts;

    GL(sVendor = glGetString(GL_VENDOR));
    GL(sRenderer = glGetString(GL_RENDERER));
    GL(sVersion = glGetString(GL_VERSION));
    GL(sExts = glGetString(GL_EXTENSIONS));

    LOGI("glVendor : %s, glRenderer : %s, glVersion : %s", sVendor, sRenderer, sVersion);
    LOGI("glExts : %s", sExts);
}

void destroyGraphicsNative() {
    LOGV("Resetting stream texture and hud texture %p, %p",
         g_ctx.streamTexture.get(),
         g_ctx.hudTexture.get());
    g_ctx.streamTexture.reset();
    g_ctx.hudTexture.reset();
    LOGV("Resetted stream texture and hud texture to %p, %p",
         g_ctx.streamTexture.get(),
         g_ctx.hudTexture.get());
}

// on resume
void prepareLobbyRoom(int viewWidth,
                      int viewHeight,
                      const unsigned int *swapchainTextures[2],
                      int swapchainLength) {
    for (int eye = 0; eye < 2; eye++) {
        g_ctx.lobbySwapchainTextures[eye].clear();

        for (int i = 0; i < swapchainLength; i++) {
            g_ctx.lobbySwapchainTextures[eye].push_back(swapchainTextures[eye][i]);
        }
    }

    g_ctx.lobbyRenderer = std::make_unique<ovrRenderer>();
    ovrRenderer_Create(g_ctx.lobbyRenderer.get(),
                       viewWidth,
                       viewHeight,
                       nullptr,
                       g_ctx.hudTexture->GetGLTexture(),
                       g_ctx.lobbySwapchainTextures,
                       {false},
                       true,
                       false);
}

// on pause
void destroyRenderers() {
    if (g_ctx.streamRenderer) {
        ovrRenderer_Destroy(g_ctx.streamRenderer.get());
        g_ctx.streamRenderer.release();
    }
    if (g_ctx.lobbyRenderer) {
        ovrRenderer_Destroy(g_ctx.lobbyRenderer.get());
        g_ctx.lobbyRenderer.release();
    }
}

void streamStartNative(FfiStreamConfig config) {
    if (g_ctx.streamRenderer) {
        ovrRenderer_Destroy(g_ctx.streamRenderer.get());
        g_ctx.streamRenderer.release();
    }

    for (int eye = 0; eye < 2; eye++) {
        g_ctx.streamSwapchainTextures[eye].clear();

        for (int i = 0; i < config.swapchainLength; i++) {
            g_ctx.streamSwapchainTextures[eye].push_back(config.swapchainTextures[eye][i]);
        }
    }

    g_ctx.streamRenderer = std::make_unique<ovrRenderer>();
    ovrRenderer_Create(g_ctx.streamRenderer.get(),
                       config.viewWidth,
                       config.viewHeight,
                       g_ctx.streamTexture.get(),
                       g_ctx.hudTexture->GetGLTexture(),
                       g_ctx.streamSwapchainTextures,
                       {(bool)config.enableFoveation,
                        config.viewWidth,
                        config.viewHeight,
                        config.foveationCenterSizeX,
                        config.foveationCenterSizeY,
                        config.foveationCenterShiftX,
                        config.foveationCenterShiftY,
                        config.foveationEdgeRatioX,
                        config.foveationEdgeRatioY},
                       false,
                       config.enableSrgbCorrection);
}

void updateLobbyHudTexture(const unsigned char *data) {
    std::lock_guard<std::mutex> lock(g_ctx.hudTextureMutex);

    g_ctx.hudTextureBitmap.resize(HUD_TEXTURE_WIDTH * HUD_TEXTURE_HEIGHT * 4);

    memcpy(&g_ctx.hudTextureBitmap[0], data, HUD_TEXTURE_WIDTH * HUD_TEXTURE_HEIGHT * 4);
}

void renderLobbyNative(const FfiViewInput eyeInputs[2]) {
    // update text image
    {
        std::lock_guard<std::mutex> lock(g_ctx.hudTextureMutex);

        if (!g_ctx.hudTextureBitmap.empty()) {
            GL(glBindTexture(GL_TEXTURE_2D, g_ctx.hudTexture->GetGLTexture()));
            GL(glTexSubImage2D(GL_TEXTURE_2D,
                               0,
                               0,
                               0,
                               HUD_TEXTURE_WIDTH,
                               HUD_TEXTURE_HEIGHT,
                               GL_RGBA,
                               GL_UNSIGNED_BYTE,
                               &g_ctx.hudTextureBitmap[0]));
        }
        g_ctx.hudTextureBitmap.clear();
    }

    ovrRenderer_RenderFrame(g_ctx.lobbyRenderer.get(), eyeInputs, true);
}

void renderStreamNative(void *streamHardwareBuffer, const unsigned int swapchainIndices[2]) {
    auto renderer = g_ctx.streamRenderer.get();

    if (streamHardwareBuffer != 0) {
        GL(EGLClientBuffer clientBuffer =
               eglGetNativeClientBufferANDROID((const AHardwareBuffer *)streamHardwareBuffer));
        GL(EGLImageKHR image = eglCreateImageKHR(
               g_ctx.eglDisplay, EGL_NO_CONTEXT, EGL_NATIVE_BUFFER_ANDROID, clientBuffer, nullptr));

        GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, g_ctx.streamTexture->GetGLTexture()));
        GL(glEGLImageTargetTexture2DOES(GL_TEXTURE_EXTERNAL_OES, (GLeglImageOES)image));

        renderer->srgbCorrectionPass->Render();
        if (renderer->enableFFE) {
            renderer->ffr->Render();
        }

        GL(eglDestroyImageKHR(g_ctx.eglDisplay, image));
    }

    FfiViewInput eyeInputs[2] = {};
    eyeInputs[0].swapchainIndex = swapchainIndices[0];
    eyeInputs[1].swapchainIndex = swapchainIndices[1];
    ovrRenderer_RenderFrame(renderer, eyeInputs, false);
}
