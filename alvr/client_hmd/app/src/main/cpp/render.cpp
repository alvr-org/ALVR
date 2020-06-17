#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>

#include "render.h"
#include "utils.h"
#include "gltf_model.h"

static const int NUM_MULTI_SAMPLES = 1;
Render_EGL egl;

#if !defined( EGL_OPENGL_ES3_BIT_KHR )
#define EGL_OPENGL_ES3_BIT_KHR		0x0040
#endif

// EXT_texture_border_clamp
#ifndef GL_CLAMP_TO_BORDER
#define GL_CLAMP_TO_BORDER            0x812D
#endif

#ifndef GL_TEXTURE_BORDER_COLOR
#define GL_TEXTURE_BORDER_COLOR        0x1004
#endif

#if !defined( GL_EXT_multisampled_render_to_texture )

typedef void (GL_APIENTRY *PFNGLRENDERBUFFERSTORAGEMULTISAMPLEEXTPROC)(GLenum target,
        GLsizei samples,
GLenum internalformat,
        GLsizei width,
GLsizei height);

typedef void (GL_APIENTRY *PFNGLFRAMEBUFFERTEXTURE2DMULTISAMPLEEXTPROC)(GLenum target,
        GLenum attachment,
GLenum textarget,
        GLuint texture, GLint level,
        GLsizei samples);

#endif

#if !defined( GL_OVR_multiview )
static const int GL_FRAMEBUFFER_ATTACHMENT_TEXTURE_NUM_VIEWS_OVR = 0x9630;
static const int GL_FRAMEBUFFER_ATTACHMENT_TEXTURE_BASE_VIEW_INDEX_OVR = 0x9632;
static const int GL_MAX_VIEWS_OVR = 0x9631;

typedef void (GL_APIENTRY *PFNGLFRAMEBUFFERTEXTUREMULTIVIEWOVRPROC)(GLenum target,
        GLenum attachment,
GLuint texture, GLint level,
GLint baseViewIndex,
        GLsizei numViews);

#endif

#if !defined( GL_OVR_multiview_multisampled_render_to_texture )

typedef void (GL_APIENTRY *PFNGLFRAMEBUFFERTEXTUREMULTISAMPLEMULTIVIEWOVRPROC)(GLenum target,
        GLenum attachment,
GLuint texture,
        GLint level,
GLsizei samples,
        GLint baseViewIndex,
GLsizei numViews);

#endif

/*
================================================================================

OpenGL-ES Utility Functions

================================================================================
*/

typedef struct {
    bool multi_view;                        // GL_OVR_multiview, GL_OVR_multiview2
    bool EXT_texture_border_clamp;            // GL_EXT_texture_border_clamp, GL_OES_texture_border_clamp
} OpenGLExtensions_t;

OpenGLExtensions_t glExtensions;

void EglInitExtensions(bool *multi_view) {
    const char *allExtensions = (const char *) glGetString(GL_EXTENSIONS);
    if (allExtensions != nullptr) {
        glExtensions.multi_view = strstr(allExtensions, "GL_OVR_multiview2") &&
                                  strstr(allExtensions,
                                         "GL_OVR_multiview_multisampled_render_to_texture");

        glExtensions.EXT_texture_border_clamp =
                strstr(allExtensions, "GL_EXT_texture_border_clamp") ||
                strstr(allExtensions, "GL_OES_texture_border_clamp");
    }
    *multi_view = glExtensions.multi_view;
}

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

static const char *GlFrameBufferStatusString(GLenum status) {
    switch (status) {
        case GL_FRAMEBUFFER_UNDEFINED:
            return "GL_FRAMEBUFFER_UNDEFINED";
        case GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT:
            return "GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT";
        case GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT:
            return "GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT";
        case GL_FRAMEBUFFER_UNSUPPORTED:
            return "GL_FRAMEBUFFER_UNSUPPORTED";
        case GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE:
            return "GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE";
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

static const char FRAGMENT_SHADER_AR[] = R"glsl(
#extension GL_OES_EGL_image_external_essl3 : enable
#extension GL_OES_EGL_image_external : enable
in lowp vec2 uv;
in lowp vec4 fragmentColor;
out lowp vec4 outColor;
uniform samplerExternalOES Texture0;
uniform %s Texture1;
uniform lowp float alpha;
void main()
{
    if(alpha > 1.0f){ // Non AR
        outColor = texture(Texture0, uv);
    } else if(alpha < -0.5f){ // Completely AR
        if(uv.x < 0.5f){
            outColor = texture(Texture1, vec2(uv.x * 2.0f, uv.y));
        }else{
            outColor = texture(Texture1, vec2(uv.x * 2.0f - 1.0f, uv.y));
        }
    }else{ // VR+AR
        lowp vec4 arColor;
        if(uv.x < 0.5f){
            arColor = texture(Texture1, vec2(uv.x * 2.0f, uv.y));
        }else{
            arColor = texture(Texture1, vec2(uv.x * 2.0f - 1.0f, uv.y));
        }
        outColor = texture(Texture0, uv) * alpha
                    + arColor * (1.0f - alpha);
    }
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
    if(Mode == 0){
        lowp float distance = length(position.xz);
        // Pick a coordinate to visualize in a grid
        lowp vec2 coord = position.xz / 2.0;
        // Compute anti-aliased world-space grid lines
        lowp vec2 grid = abs(fract(coord - 0.5) - 0.5) / fwidth(coord);
        lowp float line = min(grid.x, grid.y);
        outColor.rgb = vec3(min(line, 1.0) * (1.0 - exp(-distance / 5.0 - 0.01) / 4.0));
        if(distance > 3.0){
            lowp float coef = 1.0 - 3.0 / distance;
            outColor.rgb = (1.0 - coef) * outColor.rgb + coef * vec3(1.0, 1.0, 1.0);
        }
        outColor.a = 1.0;
    } else if(Mode == 1) {
        outColor = texture(sTexture, uv);
    } else {
        lowp float coef = 1.0;
        if(position.y < 50.0){
            coef = position.y / 100.0;
        }else if(position.y < 100.0){
            coef = (position.y - 50.0) / 50.0 * 0.3 + 0.5;
        }else{
            coef = (position.y - 100.0) / 150.0 * 0.2 + 0.8;
        }
        outColor = vec4(0.8, 0.8, 1.0, 1.0) * coef + vec4(1.0, 1.0, 1.0, 1.0) * (1.0 - coef);
    }
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

void eglDestroy()
{
    if ( egl.Display != 0 )
    {
        LOGE( "        eglMakeCurrent( Display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT )" );
        if ( eglMakeCurrent( egl.Display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT ) == EGL_FALSE )
        {
            LOGE( "        eglMakeCurrent() failed: %s", EglErrorString( eglGetError() ) );
        }
    }
    if ( egl.Context != EGL_NO_CONTEXT )
    {
        LOGE( "        eglDestroyContext( Display, Context )" );
        if ( eglDestroyContext( egl.Display, egl.Context ) == EGL_FALSE )
        {
            LOGE( "        eglDestroyContext() failed: %s", EglErrorString( eglGetError() ) );
        }
        egl.Context = EGL_NO_CONTEXT;
    }
    if ( egl.TinySurface != EGL_NO_SURFACE )
    {
        LOGE( "        eglDestroySurface( Display, TinySurface )" );
        if ( eglDestroySurface( egl.Display, egl.TinySurface ) == EGL_FALSE )
        {
            LOGE( "        eglDestroySurface() failed: %s", EglErrorString( eglGetError() ) );
        }
        egl.TinySurface = EGL_NO_SURFACE;
    }
    if ( egl.Display != 0 )
    {
        LOGE( "        eglTerminate( Display )" );
        if ( eglTerminate( egl.Display ) == EGL_FALSE )
        {
            LOGE( "        eglTerminate() failed: %s", EglErrorString( eglGetError() ) );
        }
        egl.Display = 0;
    }
}

#ifdef OVR_SDK

void ovrFramebuffer_Clear(ovrFramebuffer *frameBuffer) {
    frameBuffer->Width = 0;
    frameBuffer->Height = 0;
    frameBuffer->Multisamples = 0;
    frameBuffer->TextureSwapChainLength = 0;
    frameBuffer->TextureSwapChainIndex = 0;
    frameBuffer->UseMultiview = false;
    frameBuffer->ColorTextureSwapChain = NULL;
    frameBuffer->DepthBuffers = NULL;
    frameBuffer->FrameBuffers = NULL;
}

bool ovrFramebuffer_Create(ovrFramebuffer *frameBuffer, const bool useMultiview,
                                  const GLenum colorFormat, const int width,
                                  const int height, const int multisamples) {
    PFNGLRENDERBUFFERSTORAGEMULTISAMPLEEXTPROC glRenderbufferStorageMultisampleEXT =
            (PFNGLRENDERBUFFERSTORAGEMULTISAMPLEEXTPROC) eglGetProcAddress(
                    "glRenderbufferStorageMultisampleEXT");
    PFNGLFRAMEBUFFERTEXTURE2DMULTISAMPLEEXTPROC glFramebufferTexture2DMultisampleEXT =
            (PFNGLFRAMEBUFFERTEXTURE2DMULTISAMPLEEXTPROC) eglGetProcAddress(
                    "glFramebufferTexture2DMultisampleEXT");

    PFNGLFRAMEBUFFERTEXTUREMULTIVIEWOVRPROC glFramebufferTextureMultiviewOVR =
            (PFNGLFRAMEBUFFERTEXTUREMULTIVIEWOVRPROC) eglGetProcAddress(
                    "glFramebufferTextureMultiviewOVR");
    PFNGLFRAMEBUFFERTEXTUREMULTISAMPLEMULTIVIEWOVRPROC glFramebufferTextureMultisampleMultiviewOVR =
            (PFNGLFRAMEBUFFERTEXTUREMULTISAMPLEMULTIVIEWOVRPROC) eglGetProcAddress(
                    "glFramebufferTextureMultisampleMultiviewOVR");

    frameBuffer->Width = width;
    frameBuffer->Height = height;
    frameBuffer->Multisamples = multisamples;
    frameBuffer->UseMultiview = (useMultiview && (glFramebufferTextureMultiviewOVR != NULL)) ? true
                                                                                             : false;

    frameBuffer->ColorTextureSwapChain = vrapi_CreateTextureSwapChain3(
            frameBuffer->UseMultiview ? VRAPI_TEXTURE_TYPE_2D_ARRAY : VRAPI_TEXTURE_TYPE_2D,
            colorFormat, width, height, 1, 3);
    frameBuffer->TextureSwapChainLength = vrapi_GetTextureSwapChainLength(
            frameBuffer->ColorTextureSwapChain);
    frameBuffer->DepthBuffers = (GLuint *) malloc(
            frameBuffer->TextureSwapChainLength * sizeof(GLuint));
    frameBuffer->FrameBuffers = (GLuint *) malloc(
            frameBuffer->TextureSwapChainLength * sizeof(GLuint));

    LOG("        frameBuffer->UseMultiview = %d", frameBuffer->UseMultiview);

    for (int i = 0; i < frameBuffer->TextureSwapChainLength; i++) {
        // Create the color buffer texture.
        const GLuint colorTexture = vrapi_GetTextureSwapChainHandle(
                frameBuffer->ColorTextureSwapChain, i);
        GLenum colorTextureTarget = frameBuffer->UseMultiview ? GL_TEXTURE_2D_ARRAY : GL_TEXTURE_2D;
        GL(glBindTexture(colorTextureTarget, colorTexture));
        if (glExtensions.EXT_texture_border_clamp) {
            GL(glTexParameteri(colorTextureTarget, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_BORDER));
            GL(glTexParameteri(colorTextureTarget, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_BORDER));
            GLfloat borderColor[] = {0.0f, 0.0f, 0.0f, 0.0f};
            GL(glTexParameterfv(colorTextureTarget, GL_TEXTURE_BORDER_COLOR, borderColor));
        } else {
            // Just clamp to edge. However, this requires manually clearing the border
            // around the layer to clear the edge texels.
            GL(glTexParameteri(colorTextureTarget, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE));
            GL(glTexParameteri(colorTextureTarget, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE));
        }
        GL(glTexParameteri(colorTextureTarget, GL_TEXTURE_MIN_FILTER, GL_LINEAR));
        GL(glTexParameteri(colorTextureTarget, GL_TEXTURE_MAG_FILTER, GL_LINEAR));
        GL(glBindTexture(colorTextureTarget, 0));

        if (frameBuffer->UseMultiview) {
            // Create the depth buffer texture.
            GL(glGenTextures(1, &frameBuffer->DepthBuffers[i]));
            GL(glBindTexture(GL_TEXTURE_2D_ARRAY, frameBuffer->DepthBuffers[i]));
            GL(glTexStorage3D(GL_TEXTURE_2D_ARRAY, 1, GL_DEPTH_COMPONENT24, width, height, 2));
            GL(glBindTexture(GL_TEXTURE_2D_ARRAY, 0));

            // Create the frame buffer.
            GL(glGenFramebuffers(1, &frameBuffer->FrameBuffers[i]));
            GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, frameBuffer->FrameBuffers[i]));
            if (multisamples > 1 && (glFramebufferTextureMultisampleMultiviewOVR != NULL)) {
                GL(glFramebufferTextureMultisampleMultiviewOVR(GL_DRAW_FRAMEBUFFER,
                                                               GL_DEPTH_ATTACHMENT,
                                                               frameBuffer->DepthBuffers[i],
                                                               0 /* level */,
                                                               multisamples /* samples */,
                                                               0 /* baseViewIndex */,
                                                               2 /* numViews */ ));
                GL(glFramebufferTextureMultisampleMultiviewOVR(GL_DRAW_FRAMEBUFFER,
                                                               GL_COLOR_ATTACHMENT0, colorTexture,
                                                               0 /* level */,
                                                               multisamples /* samples */,
                                                               0 /* baseViewIndex */,
                                                               2 /* numViews */ ));
            } else {
                GL(glFramebufferTextureMultiviewOVR(GL_DRAW_FRAMEBUFFER, GL_DEPTH_ATTACHMENT,
                                                    frameBuffer->DepthBuffers[i], 0 /* level */,
                                                    0 /* baseViewIndex */, 2 /* numViews */ ));
                GL(glFramebufferTextureMultiviewOVR(GL_DRAW_FRAMEBUFFER, GL_COLOR_ATTACHMENT0,
                                                    colorTexture, 0 /* level */,
                                                    0 /* baseViewIndex */, 2 /* numViews */ ));
            }

            GL(GLenum renderFramebufferStatus = glCheckFramebufferStatus(GL_DRAW_FRAMEBUFFER));
            GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0));
            if (renderFramebufferStatus != GL_FRAMEBUFFER_COMPLETE) {
                LOGE("Incomplete frame buffer object: %s",
                     GlFrameBufferStatusString(renderFramebufferStatus));
                return false;
            }
        } else {
            if (multisamples > 1 && glRenderbufferStorageMultisampleEXT != NULL &&
                glFramebufferTexture2DMultisampleEXT != NULL) {
                // Create multisampled depth buffer.
                GL(glGenRenderbuffers(1, &frameBuffer->DepthBuffers[i]));
                GL(glBindRenderbuffer(GL_RENDERBUFFER, frameBuffer->DepthBuffers[i]));
                GL(glRenderbufferStorageMultisampleEXT(GL_RENDERBUFFER, multisamples,
                                                       GL_DEPTH_COMPONENT24, width, height));
                GL(glBindRenderbuffer(GL_RENDERBUFFER, 0));

                // Create the frame buffer.
                // NOTE: glFramebufferTexture2DMultisampleEXT only works with GL_FRAMEBUFFER.
                GL(glGenFramebuffers(1, &frameBuffer->FrameBuffers[i]));
                GL(glBindFramebuffer(GL_FRAMEBUFFER, frameBuffer->FrameBuffers[i]));
                GL(glFramebufferTexture2DMultisampleEXT(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0,
                                                        GL_TEXTURE_2D, colorTexture, 0,
                                                        multisamples));
                GL(glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER,
                                             frameBuffer->DepthBuffers[i]));
                GL(GLenum renderFramebufferStatus = glCheckFramebufferStatus(GL_FRAMEBUFFER));
                GL(glBindFramebuffer(GL_FRAMEBUFFER, 0));
                if (renderFramebufferStatus != GL_FRAMEBUFFER_COMPLETE) {
                    LOGE("Incomplete frame buffer object: %s",
                         GlFrameBufferStatusString(renderFramebufferStatus));
                    return false;
                }
            } else {
                // Create depth buffer.
                GL(glGenRenderbuffers(1, &frameBuffer->DepthBuffers[i]));
                GL(glBindRenderbuffer(GL_RENDERBUFFER, frameBuffer->DepthBuffers[i]));
                GL(glRenderbufferStorage(GL_RENDERBUFFER, GL_DEPTH_COMPONENT24, width, height));
                GL(glBindRenderbuffer(GL_RENDERBUFFER, 0));

                // Create the frame buffer.
                GL(glGenFramebuffers(1, &frameBuffer->FrameBuffers[i]));
                GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, frameBuffer->FrameBuffers[i]));
                GL(glFramebufferRenderbuffer(GL_DRAW_FRAMEBUFFER, GL_DEPTH_ATTACHMENT,
                                             GL_RENDERBUFFER, frameBuffer->DepthBuffers[i]));
                GL(glFramebufferTexture2D(GL_DRAW_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                                          colorTexture, 0));
                GL(GLenum renderFramebufferStatus = glCheckFramebufferStatus(GL_DRAW_FRAMEBUFFER));
                GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0));
                if (renderFramebufferStatus != GL_FRAMEBUFFER_COMPLETE) {
                    LOGE("Incomplete frame buffer object: %s",
                         GlFrameBufferStatusString(renderFramebufferStatus));
                    return false;
                }
            }
        }
    }

    return true;
}

void ovrFramebuffer_Destroy(ovrFramebuffer *frameBuffer) {
    GL(glDeleteFramebuffers(frameBuffer->TextureSwapChainLength, frameBuffer->FrameBuffers));
    if (frameBuffer->UseMultiview) {
        GL(glDeleteTextures(frameBuffer->TextureSwapChainLength, frameBuffer->DepthBuffers));
    } else {
        GL(glDeleteRenderbuffers(frameBuffer->TextureSwapChainLength, frameBuffer->DepthBuffers));
    }
    vrapi_DestroyTextureSwapChain(frameBuffer->ColorTextureSwapChain);

    free(frameBuffer->DepthBuffers);
    free(frameBuffer->FrameBuffers);

    ovrFramebuffer_Clear(frameBuffer);
}

void ovrFramebuffer_SetCurrent(ovrFramebuffer *frameBuffer) {
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER,
                         frameBuffer->FrameBuffers[frameBuffer->TextureSwapChainIndex]));
}

void ovrFramebuffer_SetNone() {
    GL(glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0));
}

void ovrFramebuffer_Resolve(ovrFramebuffer *frameBuffer) {
    // Discard the depth buffer, so the tiler won't need to write it back out to memory.
    const GLenum depthAttachment[1] = {GL_DEPTH_ATTACHMENT};
    glInvalidateFramebuffer(GL_DRAW_FRAMEBUFFER, 1, depthAttachment);

    // Flush this frame worth of commands.
    glFlush();
}

void ovrFramebuffer_Advance(ovrFramebuffer *frameBuffer) {
    // Advance to the next texture from the set.
    frameBuffer->TextureSwapChainIndex =
            (frameBuffer->TextureSwapChainIndex + 1) % frameBuffer->TextureSwapChainLength;
}

#endif

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
        if (geometry->VertexAttribs[i].Index != -1) {
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
                {UNIFORM_VIEW_ID,          UNIFORM_TYPE_INT,       "ViewID"},
                {UNIFORM_MVP_MATRIX,       UNIFORM_TYPE_MATRIX4X4, "mvpMatrix"},
                {UNIFORM_ALPHA, UNIFORM_TYPE_FLOAT,       "alpha"},
                {UNIFORM_COLOR, UNIFORM_TYPE_VECTOR4,       "Color"},
                {UNIFORM_M_MATRIX, UNIFORM_TYPE_MATRIX4X4,       "mMatrix"},
                {UNIFORM_MODE, UNIFORM_TYPE_INT,       "Mode"},
        };

static const char *programVersion = "#version 300 es\n";

bool
ovrProgram_Create(ovrProgram *program, const char *vertexSource, const char *fragmentSource,
                  const bool useMultiview) {
    GLint r;

    LOGI("Compiling shaders.");
    GL(program->VertexShader = glCreateShader(GL_VERTEX_SHADER));
    if (program->VertexShader == 0) {
        LOGE("glCreateShader error: %d", glGetError());
        return false;
    }

    const char *vertexSources[3] = {programVersion,
                                    (useMultiview) ? "#define DISABLE_MULTIVIEW 0\n"
                                                   : "#define DISABLE_MULTIVIEW 1\n",
                                    vertexSource
    };
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
    for (int i = 0; i < sizeof(ProgramVertexAttributes) / sizeof(ProgramVertexAttributes[0]); i++) {
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
    for (int i = 0; i < sizeof(ProgramUniforms) / sizeof(ProgramUniforms[0]); i++) {
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

void
ovrRenderer_Create(ovrRenderer *renderer, const bool useMultiview, int width, int height
        , int SurfaceTextureID, int LoadingTexture, int CameraTexture, bool ARMode, FFRData ffrData) {
    renderer->NumBuffers = useMultiview ? 1 : VRAPI_FRAME_LAYER_EYE_MAX;
    renderer->UseMultiview = useMultiview;

    renderer->enableFFR = ffrData.enabled;
    if (renderer->enableFFR) {
        renderer->ffrSourceTexture = std::make_unique<gl_render_utils::Texture>(SurfaceTextureID, true);
        renderer->ffr = std::make_unique<FFR>(renderer->ffrSourceTexture.get());
        renderer->ffr->Initialize(ffrData);
    }

#ifdef OVR_SDK
    // Create the frame buffers.
    for (int eye = 0; eye < renderer->NumBuffers; eye++) {
        ovrFramebuffer_Create(&renderer->FrameBuffer[eye], useMultiview,
                              GL_RGBA8,
                              width, height,
                              NUM_MULTI_SAMPLES);

    }
#endif

    renderer->SurfaceTextureID = SurfaceTextureID;
    renderer->LoadingTexture = LoadingTexture;
    renderer->CameraTexture = CameraTexture;
    renderer->SceneCreated = false;
    renderer->ARMode = ARMode;
    renderer->loadingScene = new GltfModel();
    renderer->loadingScene->load();
}


void ovrRenderer_CreateScene(ovrRenderer *renderer) {
    if(renderer->SceneCreated) {
        return;
    }
    const char *fragment_shader_fmt = FRAGMENT_SHADER;
    if(renderer->ARMode) {
        fragment_shader_fmt = FRAGMENT_SHADER_AR;
    }

    std::string fragment_shader;
    fragment_shader = string_format(fragment_shader_fmt,
            renderer->enableFFR ? "sampler2D" : "samplerExternalOES");

    ovrProgram_Create(&renderer->Program, VERTEX_SHADER, fragment_shader.c_str(), renderer->UseMultiview);
    ovrProgram_Create(&renderer->ProgramLoading, VERTEX_SHADER_LOADING, FRAGMENT_SHADER_LOADING,
                      renderer->UseMultiview);
    ovrGeometry_CreatePanel(&renderer->Panel);
    ovrGeometry_CreateVAO(&renderer->Panel);
    renderer->SceneCreated = true;
}

void ovrRenderer_Destroy(ovrRenderer *renderer) {
    // On Gvr, ovrFence_Destroy produces error because we cannot call it on GL render thread.
#if !defined(GVR_SDK)
    if(renderer->SceneCreated) {
        ovrProgram_Destroy(&renderer->Program);
        ovrProgram_Destroy(&renderer->ProgramLoading);
        ovrGeometry_DestroyVAO(&renderer->Panel);
        ovrGeometry_Destroy(&renderer->Panel);
    }
#endif

#ifdef OVR_SDK
    for (int eye = 0; eye < renderer->NumBuffers; eye++) {
        ovrFramebuffer_Destroy(&renderer->FrameBuffer[eye]);
    }
#endif
}

#ifdef OVR_SDK

ovrLayerProjection2 ovrRenderer_RenderFrame(ovrRenderer *renderer, const ovrTracking2 *tracking,
                                                   bool loading, int AROverlayMode) {
    if (renderer->enableFFR) {
        renderer->ffr->Render();
    }

    const ovrTracking2& updatedTracking = *tracking;

    ovrLayerProjection2 layer = vrapi_DefaultLayerProjection2();
    layer.HeadPose = updatedTracking.HeadPose;
    for (int eye = 0; eye < VRAPI_FRAME_LAYER_EYE_MAX; eye++) {
        ovrFramebuffer *frameBuffer = &renderer->FrameBuffer[renderer->NumBuffers == 1 ? 0 : eye];
        layer.Textures[eye].ColorSwapChain = frameBuffer->ColorTextureSwapChain;
        layer.Textures[eye].SwapChainIndex = frameBuffer->TextureSwapChainIndex;
        layer.Textures[eye].TexCoordsFromTanAngles = ovrMatrix4f_TanAngleMatrixFromProjection(
                &updatedTracking.Eye[eye].ProjectionMatrix);
    }
    layer.Header.Flags |= VRAPI_FRAME_LAYER_FLAG_CHROMATIC_ABERRATION_CORRECTION;


    ovrFramebuffer *frameBuffer = &renderer->FrameBuffer[0];
    ovrFramebuffer_SetCurrent(frameBuffer);

    // Render the eye images.
    for (int eye = 0; eye < renderer->NumBuffers; eye++) {
        // NOTE: In the non-mv case, latency can be further reduced by updating the sensor prediction
        // for each eye (updates orientation, not position)
        ovrFramebuffer *frameBuffer = &renderer->FrameBuffer[eye];
        ovrFramebuffer_SetCurrent(frameBuffer);

        ovrMatrix4f mvpMatrix[2];
        mvpMatrix[1] = mvpMatrix[0] = ovrMatrix4f_CreateTranslation(0, -1.5f, 0);

        mvpMatrix[0] = ovrMatrix4f_Multiply(&tracking->Eye[0].ViewMatrix,
                                            &mvpMatrix[0]);
        mvpMatrix[1] = ovrMatrix4f_Multiply(&tracking->Eye[1].ViewMatrix,
                                            &mvpMatrix[1]);
        mvpMatrix[0] = ovrMatrix4f_Multiply(&tracking->Eye[0].ProjectionMatrix,
                                            &mvpMatrix[0]);
        mvpMatrix[1] = ovrMatrix4f_Multiply(&tracking->Eye[1].ProjectionMatrix,
                                            &mvpMatrix[1]);

        Recti viewport = {0, 0, frameBuffer->Width, frameBuffer->Height};
        renderEye(eye, mvpMatrix, &viewport, renderer, loading, AROverlayMode);

        ovrFramebuffer_Resolve(frameBuffer);
        ovrFramebuffer_Advance(frameBuffer);
    }

    ovrFramebuffer_SetNone();

    return layer;
}

#endif

void renderEye(int eye, ovrMatrix4f mvpMatrix[2], Recti *viewport, ovrRenderer *renderer,
               bool loading, int AROverlayMode) {
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

        ovrMatrix4f mvpMatrix[2];
        mvpMatrix[0] = ovrMatrix4f_CreateIdentity();
        mvpMatrix[1] = ovrMatrix4f_CreateIdentity();

        GL(glBindVertexArray(renderer->Panel.VertexArrayObject));

        GL(glUniformMatrix4fv(renderer->Program.UniformLocation[UNIFORM_MVP_MATRIX], 2, true,
                              (float *) mvpMatrix));

        if(AROverlayMode == 0) {
            // VR 100%
            GL(glUniform1f(renderer->Program.UniformLocation[UNIFORM_ALPHA], 2.0f));
            GL(glActiveTexture(GL_TEXTURE0));
            if (renderer->enableFFR) {
                GL(glBindTexture(GL_TEXTURE_2D,
                                 renderer->ffr->GetOutputTexture()->GetGLTexture()));
            } else {
                GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, renderer->SurfaceTextureID));
            }

            GL(glDrawElements(GL_TRIANGLES, renderer->Panel.IndexCount, GL_UNSIGNED_SHORT, NULL));
        }else {
            if(AROverlayMode == 1) {
                // AR 30% VR 70%
                GL(glUniform1f(renderer->Program.UniformLocation[UNIFORM_ALPHA], 0.7f));
            }else if(AROverlayMode == 2) {
                // AR 70% VR 30%
                GL(glUniform1f(renderer->Program.UniformLocation[UNIFORM_ALPHA], 0.3f));
            }else if(AROverlayMode == 3) {
                // AR 100%
                GL(glUniform1f(renderer->Program.UniformLocation[UNIFORM_ALPHA], -2.0f));
            }
            GL(glActiveTexture(GL_TEXTURE0));
            if (renderer->enableFFR) {
                GL(glBindTexture(GL_TEXTURE_2D,
                                 renderer->ffr->GetOutputTexture()->GetGLTexture()));
            } else {
                GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, renderer->SurfaceTextureID));
            }
            GL(glActiveTexture(GL_TEXTURE1));
            GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, renderer->CameraTexture));
            GL(glDrawElements(GL_TRIANGLES, renderer->Panel.IndexCount, GL_UNSIGNED_SHORT, NULL));
        }
        GL(glBindVertexArray(0));

        GL(glActiveTexture(GL_TEXTURE0));
        GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0));
        GL(glActiveTexture(GL_TEXTURE1));
        GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0));
    }

    GL(glUseProgram(0));

    // Explicitly clear the border texels to black when GL_CLAMP_TO_BORDER is not available.
    if (!glExtensions.EXT_texture_border_clamp) {
        // Clear to fully opaque black.
        GL(glClearColor(0.0f, 0.0f, 0.0f, 1.0f));
        // bottom
        GL(glScissor(viewport->x, viewport->y, viewport->width, 1));
        GL(glClear(GL_COLOR_BUFFER_BIT));
        // top
        GL(glScissor(viewport->x, viewport->height - 1, viewport->width, 1));
        GL(glClear(GL_COLOR_BUFFER_BIT));
        // left
        GL(glScissor(viewport->x, viewport->y, 1, viewport->height));
        GL(glClear(GL_COLOR_BUFFER_BIT));
        // right
        GL(glScissor(viewport->x + viewport->width - 1, viewport->y, 1, viewport->height));
        GL(glClear(GL_COLOR_BUFFER_BIT));
    }
}

