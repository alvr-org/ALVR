#include "VrApi.h"
#include "VrApi_Helpers.h"
#include "VrApi_Input.h"
#include "alvr_client_core.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <android/log.h>
#include <android/native_window_jni.h>
#include <map>
#include <thread>
#include <unistd.h>
#include <vector>
#include <deque>

int gGeneralLogLevel = ANDROID_LOG_INFO;
#define LOG(...)                                                                                   \
    do {                                                                                           \
        if (gGeneralLogLevel <= ANDROID_LOG_VERBOSE) {                                             \
            __android_log_print(ANDROID_LOG_VERBOSE, "ALVR Native", __VA_ARGS__);                  \
        }                                                                                          \
    } while (false)
#define LOGI(...)                                                                                  \
    do {                                                                                           \
        if (gGeneralLogLevel <= ANDROID_LOG_INFO) {                                                \
            __android_log_print(ANDROID_LOG_INFO, "ALVR Native", __VA_ARGS__);                     \
        }                                                                                          \
    } while (false)
#define LOGE(...)                                                                                  \
    do {                                                                                           \
        if (gGeneralLogLevel <= ANDROID_LOG_ERROR) {                                               \
            __android_log_print(ANDROID_LOG_ERROR, "ALVR Native", __VA_ARGS__);                    \
        }                                                                                          \
    } while (false)

inline uint64_t getTimestampUs() {
    timeval tv;
    gettimeofday(&tv, nullptr);

    uint64_t Current = (uint64_t) tv.tv_sec * 1000 * 1000 + tv.tv_usec;
    return Current;
}

struct Render_EGL {
    EGLDisplay Display;
    EGLConfig Config;
    EGLSurface TinySurface;
    EGLSurface MainSurface;
    EGLContext Context;
};

Render_EGL egl;

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
    const EGLint configAttribs[] = {EGL_RED_SIZE,
                                    8,
                                    EGL_GREEN_SIZE,
                                    8,
                                    EGL_BLUE_SIZE,
                                    8,
                                    EGL_ALPHA_SIZE,
                                    8, // need alpha for the multi-pass timewarp compositor
                                    EGL_DEPTH_SIZE,
                                    0,
                                    EGL_STENCIL_SIZE,
                                    0,
                                    EGL_SAMPLES,
                                    0,
                                    EGL_NONE};
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
    EGLint contextAttribs[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
    LOG("        Context = eglCreateContext( Display, Config, EGL_NO_CONTEXT, contextAttribs )");
    egl.Context = eglCreateContext(egl.Display, egl.Config, EGL_NO_CONTEXT, contextAttribs);
    if (egl.Context == EGL_NO_CONTEXT) {
        LOGE("        eglCreateContext() failed: %s", EglErrorString(eglGetError()));
        return;
    }
    const EGLint surfaceAttribs[] = {EGL_WIDTH, 16, EGL_HEIGHT, 16, EGL_NONE};
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

using namespace std;

uint64_t HEAD_PATH;
uint64_t LEFT_HAND_PATH;
uint64_t RIGHT_HAND_PATH;
uint64_t LEFT_CONTROLLER_HAPTICS_PATH;
uint64_t RIGHT_CONTROLLER_HAPTICS_PATH;

// oculus touch
uint64_t MENU_CLICK;
uint64_t A_CLICK;
uint64_t A_TOUCH;
uint64_t B_CLICK;
uint64_t B_TOUCH;
uint64_t X_CLICK;
uint64_t X_TOUCH;
uint64_t Y_CLICK;
uint64_t Y_TOUCH;
uint64_t LEFT_SQUEEZE_CLICK;
uint64_t LEFT_SQUEEZE_VALUE;
uint64_t LEFT_TRIGGER_CLICK;
uint64_t LEFT_TRIGGER_VALUE;
uint64_t LEFT_TRIGGER_TOUCH;
uint64_t LEFT_THUMBSTICK_X;
uint64_t LEFT_THUMBSTICK_Y;
uint64_t LEFT_THUMBSTICK_CLICK;
uint64_t LEFT_THUMBSTICK_TOUCH;
uint64_t LEFT_THUMBREST_TOUCH;
uint64_t RIGHT_SQUEEZE_CLICK;
uint64_t RIGHT_SQUEEZE_VALUE;
uint64_t RIGHT_TRIGGER_CLICK;
uint64_t RIGHT_TRIGGER_VALUE;
uint64_t RIGHT_TRIGGER_TOUCH;
uint64_t RIGHT_THUMBSTICK_X;
uint64_t RIGHT_THUMBSTICK_Y;
uint64_t RIGHT_THUMBSTICK_CLICK;
uint64_t RIGHT_THUMBSTICK_TOUCH;
uint64_t RIGHT_THUMBREST_TOUCH;

const int MAXIMUM_TRACKING_FRAMES = 360;
const float BUTTON_EPS = 0.001; // minimum change for a scalar button to be registered as a new value
const float IPD_EPS = 0.001; // minimum change of IPD to be registered as a new value

const GLenum SWAPCHAIN_FORMAT = GL_RGBA8;

struct Swapchain {
    ovrTextureSwapChain *inner;
    int index;
};

class GlobalContext {
public:
    JavaVM *vm;
    jobject context;
    ANativeWindow *window = nullptr;
    ovrMobile *ovrContext{};
    bool running = false;
    bool streaming = false;

    std::thread eventsThread;
    std::thread trackingThread;

    float refreshRate = 60.f;
    bool controllerPredictionMultiplier;

    uint64_t ovrFrameIndex = 0;

    std::deque<std::pair<uint64_t, ovrTracking2>> trackingFrameMap;
    std::mutex trackingFrameMutex;

    Swapchain loadingSwapchains[2] = {};
    Swapchain streamSwapchains[2] = {};

    uint8_t hmdBattery = 0;
    bool hmdPlugged = false;
    uint8_t lastLeftControllerBattery = 0;
    uint8_t lastRightControllerBattery = 0;

    float lastIpd;
    EyeFov lastFov;

    std::map<uint64_t, AlvrButtonValue> previousButtonsState;

    struct HapticsState {
        uint64_t startUs;
        uint64_t endUs;
        float amplitude;
        float frequency;
        bool fresh;
        bool buffered;
    };
    HapticsState hapticsState[2]{};
};

namespace {
    GlobalContext g_ctx;
}

ovrJava getOvrJava(bool initThread = false) {
    JNIEnv *env;
    if (initThread) {
        JavaVMAttachArgs args = {JNI_VERSION_1_6};
        g_ctx.vm->AttachCurrentThread(&env, &args);
    } else {
        g_ctx.vm->GetEnv((void **) &env, JNI_VERSION_1_6);
    }

    ovrJava java{};
    java.Vm = g_ctx.vm;
    java.Env = env;
    java.ActivityObject = g_ctx.context;

    return java;
}

void updateBinary(uint64_t path, uint32_t flag) {
    auto value = flag != 0;
    auto *stateRef = &g_ctx.previousButtonsState[path];
    if (stateRef->binary != value) {
        stateRef->tag = ALVR_BUTTON_VALUE_BINARY;
        stateRef->binary = value;

        alvr_send_button(path, *stateRef);
    }
}

void updateScalar(uint64_t path, float value) {
    auto *stateRef = &g_ctx.previousButtonsState[path];
    if (abs(stateRef->scalar - value) > BUTTON_EPS) {
        stateRef->tag = ALVR_BUTTON_VALUE_SCALAR;
        stateRef->scalar = value;

        alvr_send_button(path, *stateRef);
    }
}

void updateButtons() {
    ovrInputCapabilityHeader capabilitiesHeader;
    uint32_t deviceIndex = 0;
    while (vrapi_EnumerateInputDevices(g_ctx.ovrContext, deviceIndex, &capabilitiesHeader) >= 0) {
        if (capabilitiesHeader.Type == ovrControllerType_TrackedRemote) {
            ovrInputTrackedRemoteCapabilities capabilities = {};
            capabilities.Header = capabilitiesHeader;
            if (vrapi_GetInputDeviceCapabilities(g_ctx.ovrContext, &capabilities.Header) !=
                ovrSuccess) {
                continue;
            }

            ovrInputStateTrackedRemote inputState = {};
            inputState.Header.ControllerType = capabilities.Header.Type;
            if (vrapi_GetCurrentInputState(g_ctx.ovrContext,
                                           capabilities.Header.DeviceID,
                                           &inputState.Header) != ovrSuccess) {
                continue;
            }

            if (capabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) {
                updateBinary(MENU_CLICK, inputState.Buttons & ovrButton_Enter);
                updateBinary(X_CLICK, inputState.Buttons & ovrButton_X);
                updateBinary(X_TOUCH, inputState.Touches & ovrTouch_X);
                updateBinary(Y_CLICK, inputState.Buttons & ovrButton_Y);
                updateBinary(Y_TOUCH, inputState.Touches & ovrTouch_Y);
                updateBinary(LEFT_SQUEEZE_CLICK, inputState.Buttons & ovrButton_GripTrigger);
                updateScalar(LEFT_SQUEEZE_VALUE, inputState.GripTrigger);
                updateBinary(LEFT_TRIGGER_CLICK, inputState.Buttons & ovrButton_Trigger);
                updateScalar(LEFT_TRIGGER_VALUE, inputState.IndexTrigger);
                updateBinary(LEFT_TRIGGER_TOUCH, inputState.Touches & ovrTouch_IndexTrigger);
                updateScalar(LEFT_THUMBSTICK_X, inputState.Joystick.x);
                updateScalar(LEFT_THUMBSTICK_Y, inputState.Joystick.y);
                updateBinary(LEFT_THUMBSTICK_CLICK, inputState.Buttons & ovrButton_Joystick);
                updateBinary(LEFT_THUMBSTICK_TOUCH, inputState.Touches & ovrTouch_LThumb);
                updateBinary(LEFT_THUMBREST_TOUCH, inputState.Touches & ovrTouch_ThumbRest);
            } else {
                updateBinary(A_CLICK, inputState.Buttons & ovrButton_A);
                updateBinary(A_TOUCH, inputState.Touches & ovrTouch_A);
                updateBinary(B_CLICK, inputState.Buttons & ovrButton_B);
                updateBinary(B_TOUCH, inputState.Touches & ovrTouch_B);
                updateBinary(RIGHT_SQUEEZE_CLICK, inputState.Buttons & ovrButton_GripTrigger);
                updateScalar(RIGHT_SQUEEZE_VALUE, inputState.GripTrigger);
                updateBinary(RIGHT_TRIGGER_CLICK, inputState.Buttons & ovrButton_Trigger);
                updateScalar(RIGHT_TRIGGER_VALUE, inputState.IndexTrigger);
                updateBinary(RIGHT_TRIGGER_TOUCH, inputState.Touches & ovrTouch_IndexTrigger);
                updateScalar(RIGHT_THUMBSTICK_X, inputState.Joystick.x);
                updateScalar(RIGHT_THUMBSTICK_Y, inputState.Joystick.y);
                updateBinary(RIGHT_THUMBSTICK_CLICK, inputState.Buttons & ovrButton_Joystick);
                updateBinary(RIGHT_THUMBSTICK_TOUCH, inputState.Touches & ovrTouch_RThumb);
                updateBinary(RIGHT_THUMBREST_TOUCH, inputState.Touches & ovrTouch_ThumbRest);
            }
        }

        deviceIndex++;
    }
}

float getIPD() {
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(g_ctx.ovrContext, 0.0);
    float ipd = vrapi_GetInterpupillaryDistance(&tracking);
    return ipd;
}

// return fov in OpenXR convention
std::pair<EyeFov, EyeFov> getFov() {
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(g_ctx.ovrContext, 0.0);

    EyeFov fov[2];

    for (int eye = 0; eye < 2; eye++) {
        auto projection = tracking.Eye[eye].ProjectionMatrix;
        double a = projection.M[0][0];
        double b = projection.M[1][1];
        double c = projection.M[0][2];
        double d = projection.M[1][2];

        fov[eye].left = (float) atan((c - 1) / a);
        fov[eye].right = (float) atan((c + 1) / a);
        fov[eye].top = -(float) atan((d - 1) / b);
        fov[eye].bottom = -(float) atan((d + 1) / b);
    }
    return {fov[0], fov[1]};
}

void getPlayspaceArea(float *width, float *height) {
    ovrPosef spacePose;
    ovrVector3f bboxScale;
    // Theoretically pose (the 2nd parameter) could be nullptr, since we already have that, but
    // then this function gives us 0-size bounding box, so it has to be provided.
    vrapi_GetBoundaryOrientedBoundingBox(g_ctx.ovrContext, &spacePose, &bboxScale);
    *width = 2.0f * bboxScale.x;
    *height = 2.0f * bboxScale.z;
}

uint8_t getControllerBattery(int index) {
    ovrInputCapabilityHeader curCaps;
    auto result = vrapi_EnumerateInputDevices(g_ctx.ovrContext, index, &curCaps);
    if (result < 0 || curCaps.Type != ovrControllerType_TrackedRemote) {
        return 0;
    }

    ovrInputTrackedRemoteCapabilities remoteCapabilities;
    remoteCapabilities.Header = curCaps;
    result = vrapi_GetInputDeviceCapabilities(g_ctx.ovrContext, &remoteCapabilities.Header);
    if (result != ovrSuccess) {
        return 0;
    }

    ovrInputStateTrackedRemote remoteInputState;
    remoteInputState.Header.ControllerType = remoteCapabilities.Header.Type;
    result = vrapi_GetCurrentInputState(
            g_ctx.ovrContext, remoteCapabilities.Header.DeviceID, &remoteInputState.Header);
    if (result != ovrSuccess) {
        return 0;
    }

    return remoteInputState.BatteryPercentRemaining;
}

void finishHapticsBuffer(ovrDeviceID DeviceID) {
    uint8_t hapticBuffer[1] = {0};
    ovrHapticBuffer buffer;
    buffer.BufferTime = vrapi_GetPredictedDisplayTime(g_ctx.ovrContext, g_ctx.ovrFrameIndex);
    buffer.HapticBuffer = &hapticBuffer[0];
    buffer.NumSamples = 1;
    buffer.Terminated = true;

    auto result = vrapi_SetHapticVibrationBuffer(g_ctx.ovrContext, DeviceID, &buffer);
    if (result != ovrSuccess) {
        LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
    }
}

void updateHapticsState() {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(g_ctx.ovrContext, deviceIndex, &curCaps) >= 0;
         deviceIndex++) {
        if (curCaps.Type == ovrControllerType_Gamepad)
            continue;
        ovrInputTrackedRemoteCapabilities remoteCapabilities;

        remoteCapabilities.Header = curCaps;
        result = vrapi_GetInputDeviceCapabilities(g_ctx.ovrContext, &remoteCapabilities.Header);
        if (result != ovrSuccess) {
            continue;
        }

        int curHandIndex =
                (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) ? 1 : 0;
        auto &s = g_ctx.hapticsState[curHandIndex];

        uint64_t currentUs = getTimestampUs();

        if (s.fresh) {
            s.startUs = s.startUs + currentUs;
            s.endUs = s.startUs + s.endUs;
            s.fresh = false;
        }

        if (s.startUs <= 0) {
            // No requested haptics for this hand.
            if (s.buffered) {
                finishHapticsBuffer(curCaps.DeviceID);
                s.buffered = false;
            }
            continue;
        }

        if (currentUs >= s.endUs) {
            // No more haptics is needed.
            s.startUs = 0;
            if (s.buffered) {
                finishHapticsBuffer(curCaps.DeviceID);
                s.buffered = false;
            }
            continue;
        }

        if (remoteCapabilities.ControllerCapabilities &
            ovrControllerCaps_HasBufferedHapticVibration) {
            // Note: HapticSamplesMax=25 HapticSampleDurationMS=2 on Quest

            // First, call with buffer.Terminated = false and when haptics is no more needed call
            // with buffer.Terminated = true (to stop haptics?).
            LOG("Send haptic buffer. HapticSamplesMax=%d HapticSampleDurationMS=%d",
                remoteCapabilities.HapticSamplesMax,
                remoteCapabilities.HapticSampleDurationMS);

            auto requiredHapticsBuffer = static_cast<uint32_t>(
                    (s.endUs - currentUs) / (remoteCapabilities.HapticSampleDurationMS * 1000));

            std::vector<uint8_t> hapticBuffer(remoteCapabilities.HapticSamplesMax);
            ovrHapticBuffer buffer;
            buffer.BufferTime =
                    vrapi_GetPredictedDisplayTime(g_ctx.ovrContext, g_ctx.ovrFrameIndex);
            buffer.HapticBuffer = &hapticBuffer[0];
            buffer.NumSamples =
                    std::min(remoteCapabilities.HapticSamplesMax, requiredHapticsBuffer);
            buffer.Terminated = false;

            for (uint32_t i = 0; i < buffer.NumSamples; i++) {
                if (s.amplitude > 1.0f)
                    hapticBuffer[i] = 255;
                else
                    hapticBuffer[i] = static_cast<uint8_t>(255 * s.amplitude);
            }

            result = vrapi_SetHapticVibrationBuffer(g_ctx.ovrContext, curCaps.DeviceID, &buffer);
            if (result != ovrSuccess) {
                LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
            }
            s.buffered = true;
        } else if (remoteCapabilities.ControllerCapabilities &
                   ovrControllerCaps_HasSimpleHapticVibration) {
            LOG("Send simple haptic. amplitude=%f", s.amplitude);
            vrapi_SetHapticVibrationSimple(g_ctx.ovrContext, curCaps.DeviceID, s.amplitude);
        }
    }
}

AlvrEyeInput trackingToEyeInput(ovrTracking2 *tracking, int eye) {
    auto q = tracking->HeadPose.Pose.Orientation;

    auto v = ovrMatrix4f_Inverse(&tracking->Eye[eye].ViewMatrix);

    EyeFov fov;
    if (eye == 0) {
        fov = getFov().first;
    } else {
        fov = getFov().second;
    }

    auto input = AlvrEyeInput{};
    input.orientation = AlvrQuat{q.x, q.y, q.z, q.w};
    input.position[0] = v.M[0][3];
    input.position[1] = v.M[1][3];
    input.position[2] = v.M[2][3];
    input.fov = fov;

    return input;
}

// low frequency events.
// This thread gets created after the creation of ovrContext and before its destruction
void eventsThread() {
    auto java = getOvrJava(true);

    int recenterCount = 0;

    while (g_ctx.running) {
        // there is no useful event in the oculus API, ignore
        ovrEventHeader _eventHeader;
        auto _res = vrapi_PollEvent(&_eventHeader);

        int newRecenterCount = vrapi_GetSystemStatusInt(&java, VRAPI_SYS_STATUS_RECENTER_COUNT);
        if (recenterCount != newRecenterCount) {
            float width, height;
            getPlayspaceArea(&width, &height);
            alvr_send_playspace(width, height);

            recenterCount = newRecenterCount;
        }

        float new_ipd = getIPD();
        auto new_fov = getFov();
        if (abs(new_ipd - g_ctx.lastIpd) > IPD_EPS ||
            abs(new_fov.first.left - g_ctx.lastFov.left) > IPD_EPS) {
            EyeFov fov[2] = {new_fov.first, new_fov.second};
            alvr_send_views_config(fov, new_ipd);
            g_ctx.lastIpd = new_ipd;
            g_ctx.lastFov = new_fov.first;
        }

        uint8_t leftBattery = getControllerBattery(0);
        if (leftBattery != g_ctx.lastLeftControllerBattery) {
            alvr_send_battery(LEFT_HAND_PATH, (float) leftBattery / 100.f, false);
            g_ctx.lastLeftControllerBattery = leftBattery;
        }
        uint8_t rightBattery = getControllerBattery(1);
        if (rightBattery != g_ctx.lastRightControllerBattery) {
            alvr_send_battery(RIGHT_HAND_PATH, (float) rightBattery / 100.f, false);
            g_ctx.lastRightControllerBattery = rightBattery;
        }

        AlvrEvent event;
        while (alvr_poll_event(&event)) {
            if (event.tag == ALVR_EVENT_HAPTICS) {
                auto haptics = event.HAPTICS;
                int curHandIndex = (haptics.device_id == RIGHT_CONTROLLER_HAPTICS_PATH ? 0 : 1);
                auto &s = g_ctx.hapticsState[curHandIndex];
                s.startUs = 0;
                s.endUs = (uint64_t) (haptics.duration_s * 1000'000);
                s.amplitude = haptics.amplitude;
                s.frequency = haptics.frequency;
                s.fresh = true;
                s.buffered = false;
            }
        }

        usleep(1e6 / g_ctx.refreshRate);
    }
}

// note: until some timing optimization algorithms are in place, we poll sensor data 3 times per
// frame to minimize latency
void trackingThread() {
    auto deadline = std::chrono::steady_clock::now();
    auto interval = std::chrono::nanoseconds((uint64_t) (1e9 / g_ctx.refreshRate / 3));

    auto motionVec = std::vector<AlvrDeviceMotion>();

    while (g_ctx.streaming) {
        motionVec.clear();
        OculusHand leftHand = {false};
        OculusHand rightHand = {false};

        AlvrDeviceMotion headMotion = {};
        uint64_t targetTimestampNs =
                vrapi_GetTimeInSeconds() * 1e9 + alvr_get_prediction_offset_ns();
        auto headTracking =
                vrapi_GetPredictedTracking2(g_ctx.ovrContext, (double) targetTimestampNs / 1e9);
        headMotion.device_id = HEAD_PATH;
        memcpy(&headMotion.orientation, &headTracking.HeadPose.Pose.Orientation, 4 * 4);
        memcpy(headMotion.position, &headTracking.HeadPose.Pose.Position, 4 * 3);
        // Note: do not copy velocities. Avoid reprojection in SteamVR
        motionVec.push_back(headMotion);

        {
            std::lock_guard<std::mutex> lock(g_ctx.trackingFrameMutex);
            // Insert from the front: it will be searched first
            g_ctx.trackingFrameMap.push_front({targetTimestampNs, headTracking});
            if (g_ctx.trackingFrameMap.size() > MAXIMUM_TRACKING_FRAMES) {
                g_ctx.trackingFrameMap.pop_back();
            }
        }

        updateButtons();

        double controllerDisplayTimeS =
                vrapi_GetTimeInSeconds() +
                (double) alvr_get_prediction_offset_ns() / 1e9 *
                g_ctx.controllerPredictionMultiplier;

        ovrInputCapabilityHeader capabilitiesHeader;
        uint32_t deviceIndex = 0;
        while (vrapi_EnumerateInputDevices(g_ctx.ovrContext, deviceIndex, &capabilitiesHeader) >=
               0) {
            if (capabilitiesHeader.Type == ovrControllerType_TrackedRemote) {
                ovrInputTrackedRemoteCapabilities capabilities = {};
                capabilities.Header = capabilitiesHeader;
                if (vrapi_GetInputDeviceCapabilities(g_ctx.ovrContext, &capabilities.Header) !=
                    ovrSuccess) {
                    continue;
                }

                uint64_t handPath;
                if (capabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) {
                    handPath = LEFT_HAND_PATH;
                } else {
                    handPath = RIGHT_HAND_PATH;
                }

                ovrTracking tracking = {};
                if (vrapi_GetInputTrackingState(g_ctx.ovrContext,
                                                capabilities.Header.DeviceID,
                                                controllerDisplayTimeS,
                                                &tracking) == ovrSuccess) {
                    AlvrDeviceMotion motion = {};
                    motion.device_id = handPath;
                    memcpy(&motion.orientation, &tracking.HeadPose.Pose.Orientation, 4 * 4);
                    memcpy(motion.position, &tracking.HeadPose.Pose.Position, 4 * 3);
                    memcpy(motion.linear_velocity, &tracking.HeadPose.LinearVelocity, 4 * 3);
                    memcpy(motion.angular_velocity, &tracking.HeadPose.AngularVelocity, 4 * 3);

                    motionVec.push_back(motion);
                }
            } else if (capabilitiesHeader.Type == ovrControllerType_Hand) {
                ovrInputHandCapabilities capabilities;
                capabilities.Header = capabilitiesHeader;
                if (vrapi_GetInputDeviceCapabilities(g_ctx.ovrContext, &capabilities.Header) !=
                    ovrSuccess) {
                    continue;
                }

                uint64_t handPath;
                OculusHand *handRef = nullptr;
                if (capabilities.HandCapabilities & ovrHandCaps_LeftHand) {
                    handPath = LEFT_HAND_PATH;
                    handRef = &leftHand;
                } else {
                    handPath = RIGHT_HAND_PATH;
                    handRef = &rightHand;
                }

                ovrHandPose handPose;
                handPose.Header.Version = ovrHandVersion_1;
                if (vrapi_GetHandPose(g_ctx.ovrContext,
                                      capabilities.Header.DeviceID,
                                      controllerDisplayTimeS,
                                      &handPose.Header) == ovrSuccess &&
                    (handPose.Status & ovrHandTrackingStatus_Tracked)) {
                    AlvrDeviceMotion motion = {};
                    motion.device_id = handPath;
                    memcpy(&motion.orientation, &handPose.RootPose.Orientation, 4 * 4);
                    memcpy(motion.position, &handPose.RootPose.Position, 4 * 3);
                    // Note: ovrHandPose does not have velocities
                    for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                        memcpy(&handRef->bone_rotations[i], &handPose.BoneRotations[i], 4 * 4);
                    }
                    motionVec.push_back(motion);
                    handRef->enabled = true;
                }
            }

            deviceIndex++;
        }

        alvr_send_tracking(targetTimestampNs, &motionVec[0], motionVec.size(), leftHand, rightHand);

        deadline += interval;
        std::this_thread::sleep_until(deadline);
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_initializeNative(JNIEnv *env, jobject context) {
    env->GetJavaVM(&g_ctx.vm);
    g_ctx.context = env->NewGlobalRef(context);

    HEAD_PATH = alvr_path_string_to_hash("/user/head");
    LEFT_HAND_PATH = alvr_path_string_to_hash("/user/hand/left");
    RIGHT_HAND_PATH = alvr_path_string_to_hash("/user/hand/right");
    LEFT_CONTROLLER_HAPTICS_PATH = alvr_path_string_to_hash("/user/hand/left/output/haptic");
    RIGHT_CONTROLLER_HAPTICS_PATH = alvr_path_string_to_hash("/user/hand/right/output/haptic");

    MENU_CLICK = alvr_path_string_to_hash("/user/hand/left/input/menu/click");
    A_CLICK = alvr_path_string_to_hash("/user/hand/right/input/a/click");
    A_TOUCH = alvr_path_string_to_hash("/user/hand/right/input/a/touch");
    B_CLICK = alvr_path_string_to_hash("/user/hand/right/input/b/click");
    B_TOUCH = alvr_path_string_to_hash("/user/hand/right/input/b/touch");
    X_CLICK = alvr_path_string_to_hash("/user/hand/left/input/x/click");
    X_TOUCH = alvr_path_string_to_hash("/user/hand/left/input/x/touch");
    Y_CLICK = alvr_path_string_to_hash("/user/hand/left/input/y/click");
    Y_TOUCH = alvr_path_string_to_hash("/user/hand/left/input/y/touch");
    LEFT_SQUEEZE_CLICK = alvr_path_string_to_hash("/user/hand/left/input/squeeze/click");
    LEFT_SQUEEZE_VALUE = alvr_path_string_to_hash("/user/hand/left/input/squeeze/value");
    LEFT_TRIGGER_CLICK = alvr_path_string_to_hash("/user/hand/left/input/trigger/click");
    LEFT_TRIGGER_VALUE = alvr_path_string_to_hash("/user/hand/left/input/trigger/value");
    LEFT_TRIGGER_TOUCH = alvr_path_string_to_hash("/user/hand/left/input/trigger/touch");
    LEFT_THUMBSTICK_X = alvr_path_string_to_hash("/user/hand/left/input/thumbstick/x");
    LEFT_THUMBSTICK_Y = alvr_path_string_to_hash("/user/hand/left/input/thumbstick/y");
    LEFT_THUMBSTICK_CLICK = alvr_path_string_to_hash("/user/hand/left/input/thumbstick/click");
    LEFT_THUMBSTICK_TOUCH = alvr_path_string_to_hash("/user/hand/left/input/thumbstick/touch");
    LEFT_THUMBREST_TOUCH = alvr_path_string_to_hash("/user/hand/left/input/thumbrest/touch");
    RIGHT_SQUEEZE_CLICK = alvr_path_string_to_hash("/user/hand/right/input/squeeze/click");
    RIGHT_SQUEEZE_VALUE = alvr_path_string_to_hash("/user/hand/right/input/squeeze/value");
    RIGHT_TRIGGER_CLICK = alvr_path_string_to_hash("/user/hand/right/input/trigger/click");
    RIGHT_TRIGGER_VALUE = alvr_path_string_to_hash("/user/hand/right/input/trigger/value");
    RIGHT_TRIGGER_TOUCH = alvr_path_string_to_hash("/user/hand/right/input/trigger/touch");
    RIGHT_THUMBSTICK_X = alvr_path_string_to_hash("/user/hand/right/input/thumbstick/x");
    RIGHT_THUMBSTICK_Y = alvr_path_string_to_hash("/user/hand/right/input/thumbstick/y");
    RIGHT_THUMBSTICK_CLICK = alvr_path_string_to_hash("/user/hand/right/input/thumbstick/click");
    RIGHT_THUMBSTICK_TOUCH = alvr_path_string_to_hash("/user/hand/right/input/thumbstick/touch");
    RIGHT_THUMBREST_TOUCH = alvr_path_string_to_hash("/user/hand/right/input/thumbrest/touch");

    auto java = getOvrJava(true);

    eglInit();

    alvr_initialize((void *) g_ctx.vm, (void *) g_ctx.context);

    memset(g_ctx.hapticsState, 0, sizeof(g_ctx.hapticsState));
    const ovrInitParms initParms = vrapi_DefaultInitParms(&java);
    int32_t initResult = vrapi_Initialize(&initParms);
    if (initResult != VRAPI_INITIALIZE_SUCCESS) {
        // If initialization failed, vrapi_* function calls will not be available.
        LOGE("vrapi_Initialize failed");
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_destroyNative(JNIEnv *_env, jobject _context) {
    vrapi_Shutdown();

    alvr_destroy();

    eglDestroy();

    auto java = getOvrJava();
    java.Env->DeleteGlobalRef(g_ctx.context);
}

extern "C" JNIEXPORT void JNICALL Java_com_polygraphene_alvr_OvrActivity_onResumeNative(
        JNIEnv *_env, jobject _context, jobject surface) {
    auto java = getOvrJava();

    g_ctx.window = ANativeWindow_fromSurface(java.Env, surface);

    LOGI("Entering VR mode.");

    ovrModeParms parms = vrapi_DefaultModeParms(&java);

    parms.Flags |= VRAPI_MODE_FLAG_RESET_WINDOW_FULLSCREEN;

    parms.Flags |= VRAPI_MODE_FLAG_NATIVE_WINDOW;
    parms.Display = (size_t) egl.Display;
    parms.WindowSurface = (size_t) g_ctx.window;
    parms.ShareContext = (size_t) egl.Context;

    g_ctx.ovrContext = vrapi_EnterVrMode(&parms);

    if (g_ctx.ovrContext == nullptr) {
        LOGE("Invalid ANativeWindow");
    }

    // set Color Space
    ovrHmdColorDesc colorDesc{};
    colorDesc.ColorSpace = VRAPI_COLORSPACE_RIFT_S;
    vrapi_SetClientColorDesc(g_ctx.ovrContext, &colorDesc);

    vrapi_SetPerfThread(g_ctx.ovrContext, VRAPI_PERF_THREAD_TYPE_MAIN, gettid());

    vrapi_SetTrackingSpace(g_ctx.ovrContext, VRAPI_TRACKING_SPACE_STAGE);

    auto width = vrapi_GetSystemPropertyInt(&java, VRAPI_SYS_PROP_DISPLAY_PIXELS_WIDE) / 2;
    auto height = vrapi_GetSystemPropertyInt(&java, VRAPI_SYS_PROP_DISPLAY_PIXELS_HIGH);

    std::vector<int32_t> textureHandlesBuffer[2];
    for (int eye = 0; eye < 2; eye++) {
        g_ctx.loadingSwapchains[eye].inner = vrapi_CreateTextureSwapChain3(
                VRAPI_TEXTURE_TYPE_2D, SWAPCHAIN_FORMAT, width, height, 1, 3);
        int size = vrapi_GetTextureSwapChainLength(g_ctx.loadingSwapchains[eye].inner);

        for (int index = 0; index < size; index++) {
            auto handle =
                    vrapi_GetTextureSwapChainHandle(g_ctx.loadingSwapchains[eye].inner, index);
            textureHandlesBuffer[eye].push_back(handle);
        }

        g_ctx.loadingSwapchains[eye].index = 0;
    }
    const int32_t *textureHandles[2] = {&textureHandlesBuffer[0][0], &textureHandlesBuffer[1][0]};

    g_ctx.running = true;
    g_ctx.eventsThread = std::thread(eventsThread);

    auto refreshRatesCount =
            vrapi_GetSystemPropertyInt(&java, VRAPI_SYS_PROP_NUM_SUPPORTED_DISPLAY_REFRESH_RATES);
    auto refreshRatesBuffer = vector<float>(refreshRatesCount);
    vrapi_GetSystemPropertyFloatArray(&java,
                                      VRAPI_SYS_PROP_SUPPORTED_DISPLAY_REFRESH_RATES,
                                      &refreshRatesBuffer[0],
                                      refreshRatesCount);

    alvr_resume(width,
                height,
                &refreshRatesBuffer[0],
                refreshRatesCount,
                textureHandles,
                textureHandlesBuffer[0].size());
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_onPauseNative(JNIEnv *_env, jobject _context) {
    alvr_pause();

    LOGI("Leaving VR mode.");

    if (g_ctx.streaming) {
        g_ctx.streaming = false;
        g_ctx.trackingThread.join();
    }
    if (g_ctx.running) {
        g_ctx.running = false;
        g_ctx.eventsThread.join();
    }

    if (g_ctx.streamSwapchains[0].inner != nullptr) {
        vrapi_DestroyTextureSwapChain(g_ctx.streamSwapchains[0].inner);
        vrapi_DestroyTextureSwapChain(g_ctx.streamSwapchains[1].inner);
        g_ctx.streamSwapchains[0].inner = nullptr;
        g_ctx.streamSwapchains[1].inner = nullptr;
    }
    if (g_ctx.loadingSwapchains[0].inner != nullptr) {
        vrapi_DestroyTextureSwapChain(g_ctx.loadingSwapchains[0].inner);
        vrapi_DestroyTextureSwapChain(g_ctx.loadingSwapchains[1].inner);
        g_ctx.loadingSwapchains[0].inner = nullptr;
        g_ctx.loadingSwapchains[1].inner = nullptr;
    }

    vrapi_LeaveVrMode(g_ctx.ovrContext);

    g_ctx.ovrContext = nullptr;

    if (g_ctx.window != nullptr) {
        ANativeWindow_release(g_ctx.window);
    }
    g_ctx.window = nullptr;
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_onStreamStartNative(JNIEnv *_env,
                                                           jobject _context,
                                                           jint eyeWidth,
                                                           jint eyeHeight,
                                                           jfloat fps,
                                                           jobject decoder,
                                                           jint codec,
                                                           jboolean realTimeDecoder,
                                                           jint oculusFoveationLevel,
                                                           jboolean dynamicOculusFoveation,
                                                           jboolean extraLatency,
                                                           jfloat controllerPredictionMultiplier) {
    auto java = getOvrJava();

    g_ctx.refreshRate = fps;
    g_ctx.controllerPredictionMultiplier = controllerPredictionMultiplier;

    if (g_ctx.streamSwapchains[0].inner != nullptr) {
        vrapi_DestroyTextureSwapChain(g_ctx.streamSwapchains[0].inner);
        vrapi_DestroyTextureSwapChain(g_ctx.streamSwapchains[1].inner);
        g_ctx.streamSwapchains[0].inner = nullptr;
        g_ctx.streamSwapchains[1].inner = nullptr;
    }

    std::vector<int32_t> textureHandlesBuffer[2];
    for (int eye = 0; eye < 2; eye++) {
        g_ctx.streamSwapchains[eye].inner = vrapi_CreateTextureSwapChain3(
                VRAPI_TEXTURE_TYPE_2D, SWAPCHAIN_FORMAT, eyeWidth, eyeHeight, 1, 3);
        auto size = vrapi_GetTextureSwapChainLength(g_ctx.streamSwapchains[eye].inner);

        for (int index = 0; index < size; index++) {
            auto handle = vrapi_GetTextureSwapChainHandle(g_ctx.streamSwapchains[eye].inner, index);
            textureHandlesBuffer[eye].push_back(handle);
        }

        g_ctx.streamSwapchains[eye].index = 0;
    }
    const int32_t *textureHandles[2] = {&textureHandlesBuffer[0][0], &textureHandlesBuffer[1][0]};

    // On Oculus Quest, without ExtraLatencyMode frames passed to vrapi_SubmitFrame2 are sometimes
    // discarded from VrAPI(?). Which introduces stutter animation. I think the number of discarded
    // frames is shown as Stale in Logcat like following:
    //    I/VrApi:
    //    FPS=72,Prd=63ms,Tear=0,Early=0,Stale=8,VSnc=1,Lat=0,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/F/N,Mem=1804MHz,Free=989MB,PSM=0,PLS=0,Temp=36.0C/0.0C,TW=1.90ms,App=2.74ms,GD=0.00ms
    // After enabling ExtraLatencyMode:
    //    I/VrApi:
    //    FPS=71,Prd=76ms,Tear=0,Early=66,Stale=0,VSnc=1,Lat=1,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/N/N,Mem=1804MHz,Free=906MB,PSM=0,PLS=0,Temp=38.0C/0.0C,TW=1.93ms,App=1.46ms,GD=0.00ms
    // We need to set ExtraLatencyMode On to workaround for this issue.
    vrapi_SetExtraLatencyMode(g_ctx.ovrContext, (ovrExtraLatencyMode) extraLatency);

    ovrResult result = vrapi_SetDisplayRefreshRate(g_ctx.ovrContext, fps);
    if (result != ovrSuccess) {
        LOGE("Failed to set refresh rate requested by the server: %d", result);
    }

    vrapi_SetPropertyInt(&java, VRAPI_FOVEATION_LEVEL, oculusFoveationLevel);
    vrapi_SetPropertyInt(&java, VRAPI_DYNAMIC_FOVEATION_ENABLED, dynamicOculusFoveation);

    if (g_ctx.streaming) {
        g_ctx.streaming = false;
        g_ctx.trackingThread.join();
    }
    g_ctx.streaming = true;
    g_ctx.trackingThread = std::thread(trackingThread);

    auto fov = getFov();

    EyeFov fovArr[2] = {fov.first, fov.second};
    auto ipd = getIPD();
    alvr_send_views_config(fovArr, ipd);

    alvr_send_battery(HEAD_PATH, g_ctx.hmdBattery, g_ctx.hmdPlugged);
    alvr_send_battery(LEFT_HAND_PATH, getControllerBattery(0) / 100.f, false);
    alvr_send_battery(LEFT_HAND_PATH, getControllerBattery(1) / 100.f, false);

    float areaWidth, areaHeight;
    getPlayspaceArea(&areaWidth, &areaHeight);
    alvr_send_playspace(areaWidth, areaHeight);

    alvr_start_stream(
            decoder, codec, realTimeDecoder, textureHandles, textureHandlesBuffer[0].size());
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_polygraphene_alvr_OvrActivity_isConnectedNative(JNIEnv *_env, jobject _context) {
    return alvr_is_streaming();
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_renderLoadingNative(JNIEnv *_env, jobject _context) {
    double displayTime = vrapi_GetPredictedDisplayTime(g_ctx.ovrContext, g_ctx.ovrFrameIndex);
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(g_ctx.ovrContext, displayTime);

    AlvrEyeInput eyeInputs[2] = {trackingToEyeInput(&tracking, 0),
                                 trackingToEyeInput(&tracking, 1)};
    int swapchainIndices[2] = {g_ctx.loadingSwapchains[0].index, g_ctx.loadingSwapchains[1].index};
    alvr_render_lobby(eyeInputs, swapchainIndices);

    ovrLayerProjection2 worldLayer = vrapi_DefaultLayerProjection2();
    worldLayer.HeadPose = tracking.HeadPose;
    for (int eye = 0; eye < VRAPI_FRAME_LAYER_EYE_MAX; eye++) {
        worldLayer.Textures[eye].ColorSwapChain = g_ctx.loadingSwapchains[eye].inner;
        worldLayer.Textures[eye].SwapChainIndex = g_ctx.loadingSwapchains[eye].index;
        worldLayer.Textures[eye].TexCoordsFromTanAngles =
                ovrMatrix4f_TanAngleMatrixFromProjection(&tracking.Eye[eye].ProjectionMatrix);
    }
    worldLayer.Header.Flags |= VRAPI_FRAME_LAYER_FLAG_CHROMATIC_ABERRATION_CORRECTION;

    const ovrLayerHeader2 *layers[] = {&worldLayer.Header};

    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = g_ctx.ovrFrameIndex;
    frameDesc.DisplayTime = displayTime;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers;

    vrapi_SubmitFrame2(g_ctx.ovrContext, &frameDesc);

    g_ctx.loadingSwapchains[0].index = (g_ctx.loadingSwapchains[0].index + 1) % 3;
    g_ctx.loadingSwapchains[1].index = (g_ctx.loadingSwapchains[1].index + 1) % 3;
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_renderNative(JNIEnv *_env, jobject _context) {
    auto timestampNs = alvr_wait_for_frame();

    if (timestampNs == -1) {
        return;
    }

    g_ctx.ovrFrameIndex++;

    updateHapticsState();

    ovrTracking2 tracking;
    {
        std::lock_guard<std::mutex> lock(g_ctx.trackingFrameMutex);

        // Take the frame with equal timestamp, or the next closest one.
        for (auto &pair: g_ctx.trackingFrameMap) {
            if (pair.first <= timestampNs) {
                tracking = pair.second;
                break;
            }
        }
    }

    int swapchainIndices[2] = {g_ctx.streamSwapchains[0].index, g_ctx.streamSwapchains[1].index};
    alvr_render_stream(timestampNs, swapchainIndices);

    double vsyncQueueS = vrapi_GetPredictedDisplayTime(g_ctx.ovrContext, g_ctx.ovrFrameIndex) -
                         vrapi_GetTimeInSeconds();
    alvr_report_submit(timestampNs, vsyncQueueS * 1e9);

    ovrLayerProjection2 worldLayer = vrapi_DefaultLayerProjection2();
    worldLayer.HeadPose = tracking.HeadPose;
    for (int eye = 0; eye < VRAPI_FRAME_LAYER_EYE_MAX; eye++) {
        worldLayer.Textures[eye].ColorSwapChain = g_ctx.streamSwapchains[eye].inner;
        worldLayer.Textures[eye].SwapChainIndex = g_ctx.streamSwapchains[eye].index;
        worldLayer.Textures[eye].TexCoordsFromTanAngles =
                ovrMatrix4f_TanAngleMatrixFromProjection(&tracking.Eye[eye].ProjectionMatrix);
    }
    worldLayer.Header.Flags |= VRAPI_FRAME_LAYER_FLAG_CHROMATIC_ABERRATION_CORRECTION;

    const ovrLayerHeader2 *layers2[] = {&worldLayer.Header};

    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = g_ctx.ovrFrameIndex;
    frameDesc.DisplayTime = (double) timestampNs / 1e9;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers2;

    vrapi_SubmitFrame2(g_ctx.ovrContext, &frameDesc);

    g_ctx.streamSwapchains[0].index = (g_ctx.streamSwapchains[0].index + 1) % 3;
    g_ctx.streamSwapchains[1].index = (g_ctx.streamSwapchains[1].index + 1) % 3;
}

extern "C" JNIEXPORT void JNICALL Java_com_polygraphene_alvr_OvrActivity_onBatteryChangedNative(
        JNIEnv *_env, jobject _context, jint battery, jboolean plugged) {
    alvr_send_battery(HEAD_PATH, (float) battery / 100.f, (bool) plugged);
    g_ctx.hmdBattery = battery;
    g_ctx.hmdPlugged = plugged;
}

extern "C" JNIEXPORT void JNICALL Java_com_polygraphene_alvr_DecoderThread_setWaitingNextIDR(
        JNIEnv *_env, jclass _class, jboolean waiting) {
    alvr_set_waiting_next_idr(waiting);
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_DecoderThread_requestIDR(JNIEnv *_env, jclass _class) {
    alvr_request_idr();
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_DecoderThread_restartRenderCycle(JNIEnv *_env, jclass _class) {
    alvr_restart_rendering_cycle();
}

extern "C" JNIEXPORT jint JNICALL
Java_com_polygraphene_alvr_DecoderThread_getStreamTextureHandle(JNIEnv *_env, jclass _class) {
    return alvr_get_stream_texture_handle();
}
