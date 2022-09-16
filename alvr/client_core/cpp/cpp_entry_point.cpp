#include "bindings.h"

#include "ffr.h"
#include "render.h"
#include "utils.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2ext.h>
#include <GLES3/gl3.h>
#include <mutex>

using namespace std;
using namespace gl_render_utils;

void (*reportSubmit)(unsigned long long targetTimestampNs, unsigned long long vsyncQueueNs);
unsigned long long (*pathStringToHash)(const char *path);

const int LOADING_TEXTURE_WIDTH = 1280;
const int LOADING_TEXTURE_HEIGHT = 720;

class GlobalContext {
  public:
    EGLDisplay eglDisplay;

    unique_ptr<Texture> streamTexture;
    unique_ptr<Texture> loadingTexture;
    std::vector<uint8_t> loadingTextureBitmap;
    std::mutex loadingTextureMutex;

    StreamConfigInput streamConfig{};

    std::vector<GLuint> loadingSwapchainTextures[2];
    std::unique_ptr<ovrRenderer> loadingRenderer;
    std::vector<GLuint> streamSwapchainTextures[2];
    std::unique_ptr<ovrRenderer> streamRenderer;
};

namespace {
PFNEGLGETNATIVECLIENTBUFFERANDROIDPROC eglGetNativeClientBufferANDROID;
PFNGLEGLIMAGETARGETTEXTURE2DOESPROC glEGLImageTargetTexture2DOES;
GlobalContext g_ctx;
} // namespace

void initNative() {
    g_ctx.eglDisplay = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    eglGetNativeClientBufferANDROID = (PFNEGLGETNATIVECLIENTBUFFERANDROIDPROC)eglGetProcAddress(
        "eglGetNativeClientBufferANDROID");
    glEGLImageTargetTexture2DOES =
        (PFNGLEGLIMAGETARGETTEXTURE2DOESPROC)eglGetProcAddress("glEGLImageTargetTexture2DOES");

    g_ctx.streamTexture = make_unique<Texture>(true);
    g_ctx.loadingTexture =
        make_unique<Texture>(false, 1280, 720, GL_RGBA, std::vector<uint8_t>(1280 * 720 * 4, 0));
}

void destroyNative() {
    g_ctx.streamTexture.reset();
    g_ctx.loadingTexture.reset();
}

void prepareLoadingRoom(int eyeWidth,
                        int eyeHeight,
                        bool darkMode,
                        const int *swapchainTextures[2],
                        int swapchainLength) {
    for (int eye = 0; eye < 2; eye++) {
        g_ctx.loadingSwapchainTextures[eye].clear();

        for (int i = 0; i < swapchainLength; i++) {
            g_ctx.loadingSwapchainTextures[eye].push_back(swapchainTextures[eye][i]);
        }
    }

    g_ctx.loadingRenderer = std::make_unique<ovrRenderer>();
    ovrRenderer_Create(g_ctx.loadingRenderer.get(),
                       eyeWidth,
                       eyeHeight,
                       nullptr,
                       g_ctx.loadingTexture->GetGLTexture(),
                       g_ctx.loadingSwapchainTextures,
                       darkMode,
                       {false});
}

void setStreamConfig(StreamConfigInput config) { g_ctx.streamConfig = config; }

void streamStartNative(const int *swapchainTextures[2], int swapchainLength) {
    if (g_ctx.streamRenderer) {
        ovrRenderer_Destroy(g_ctx.streamRenderer.get());
        g_ctx.streamRenderer.release();
    }

    for (int eye = 0; eye < 2; eye++) {
        g_ctx.streamSwapchainTextures[eye].clear();

        for (int i = 0; i < swapchainLength; i++) {
            g_ctx.streamSwapchainTextures[eye].push_back(swapchainTextures[eye][i]);
        }
    }

    g_ctx.streamRenderer = std::make_unique<ovrRenderer>();
    ovrRenderer_Create(g_ctx.streamRenderer.get(),
                       g_ctx.streamConfig.eyeWidth,
                       g_ctx.streamConfig.eyeHeight,
                       g_ctx.streamTexture.get(),
                       g_ctx.loadingTexture->GetGLTexture(),
                       g_ctx.streamSwapchainTextures,
                       false,
                       {g_ctx.streamConfig.enableFoveation,
                        g_ctx.streamConfig.eyeWidth,
                        g_ctx.streamConfig.eyeHeight,
                        g_ctx.streamConfig.foveationCenterSizeX,
                        g_ctx.streamConfig.foveationCenterSizeY,
                        g_ctx.streamConfig.foveationCenterShiftX,
                        g_ctx.streamConfig.foveationCenterShiftY,
                        g_ctx.streamConfig.foveationEdgeRatioX,
                        g_ctx.streamConfig.foveationEdgeRatioY});
}

void destroyRenderers() {
    if (g_ctx.streamRenderer) {
        ovrRenderer_Destroy(g_ctx.streamRenderer.get());
        g_ctx.streamRenderer.release();
    }
    if (g_ctx.loadingRenderer) {
        ovrRenderer_Destroy(g_ctx.loadingRenderer.get());
        g_ctx.loadingRenderer.release();
    }
}

void updateLoadingTexuture(const unsigned char *data) {
    std::lock_guard<std::mutex> lock(g_ctx.loadingTextureMutex);

    g_ctx.loadingTextureBitmap.resize(LOADING_TEXTURE_WIDTH * LOADING_TEXTURE_HEIGHT * 4);

    memcpy(
        &g_ctx.loadingTextureBitmap[0], data, LOADING_TEXTURE_WIDTH * LOADING_TEXTURE_HEIGHT * 4);
}

void renderLoadingNative(const EyeInput eyeInputs[2], const int swapchainIndices[2]) {
    // update text image
    {
        std::lock_guard<std::mutex> lock(g_ctx.loadingTextureMutex);

        if (!g_ctx.loadingTextureBitmap.empty()) {
            glBindTexture(GL_TEXTURE_2D, g_ctx.loadingTexture->GetGLTexture());
            glTexSubImage2D(GL_TEXTURE_2D,
                            0,
                            0,
                            0,
                            LOADING_TEXTURE_WIDTH,
                            LOADING_TEXTURE_HEIGHT,
                            GL_RGBA,
                            GL_UNSIGNED_BYTE,
                            &g_ctx.loadingTextureBitmap[0]);
        }
        g_ctx.loadingTextureBitmap.clear();
    }

    ovrRenderer_RenderFrame(g_ctx.loadingRenderer.get(), eyeInputs, swapchainIndices, true);
}

void renderNative(const int swapchainIndices[2], void *streamHardwareBuffer) {
    GL(EGLClientBuffer clientBuffer =
           eglGetNativeClientBufferANDROID((const AHardwareBuffer *)streamHardwareBuffer));
    GL(EGLImage image = eglCreateImage(
           g_ctx.eglDisplay, EGL_NO_CONTEXT, EGL_NATIVE_BUFFER_ANDROID, clientBuffer, nullptr));

    GL(glBindTexture(GL_TEXTURE_EXTERNAL_OES, g_ctx.streamTexture->GetGLTexture()));
    GL(glEGLImageTargetTexture2DOES(GL_TEXTURE_EXTERNAL_OES, (GLeglImageOES)image));

    EyeInput eyeInputs[2] = {};
    ovrRenderer_RenderFrame(g_ctx.streamRenderer.get(), eyeInputs, swapchainIndices, false);

    GL(eglDestroyImage(g_ctx.eglDisplay, image));
}