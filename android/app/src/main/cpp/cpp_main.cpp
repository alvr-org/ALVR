#include "VrApi.h"
#include "VrApi_Helpers.h"
#include "VrApi_Input.h"
#include "alvr_client_core.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <android/log.h>
#include <android/native_window_jni.h>
#include <deque>
#include <map>
#include <thread>
#include <unistd.h>
#include <vector>

void log(AlvrLogLevel level, const char *format, ...) {
    va_list args;
    va_start(args, format);

    char buf[1024];
    int count = vsnprintf(buf, sizeof(buf), format, args);
    if (count > (int) sizeof(buf))
        count = (int) sizeof(buf);
    if (count > 0 && buf[count - 1] == '\n')
        buf[count - 1] = '\0';

    alvr_log(level, buf);

    va_end(args);
}

#define error(...) log(ALVR_LOG_LEVEL_ERROR, __VA_ARGS__)
#define info(...) log(ALVR_LOG_LEVEL_INFO, __VA_ARGS__)

uint64_t HEAD_ID = alvr_path_string_to_hash("/user/head");
uint64_t LEFT_HAND_ID = alvr_path_string_to_hash("/user/hand/left");
uint64_t RIGHT_HAND_ID = alvr_path_string_to_hash("/user/hand/right");
uint64_t LEFT_CONTROLLER_HAPTICS_ID = alvr_path_string_to_hash("/user/hand/left/output/haptic");
uint64_t RIGHT_CONTROLLER_HAPTICS_ID = alvr_path_string_to_hash("/user/hand/right/output/haptic");

uint64_t MENU_CLICK_ID = alvr_path_string_to_hash("/user/hand/left/input/menu/click");
uint64_t A_CLICK_ID = alvr_path_string_to_hash("/user/hand/right/input/a/click");
uint64_t A_TOUCH_ID = alvr_path_string_to_hash("/user/hand/right/input/a/touch");
uint64_t B_CLICK_ID = alvr_path_string_to_hash("/user/hand/right/input/b/click");
uint64_t B_TOUCH_ID = alvr_path_string_to_hash("/user/hand/right/input/b/touch");
uint64_t X_CLICK_ID = alvr_path_string_to_hash("/user/hand/left/input/x/click");
uint64_t X_TOUCH_ID = alvr_path_string_to_hash("/user/hand/left/input/x/touch");
uint64_t Y_CLICK_ID = alvr_path_string_to_hash("/user/hand/left/input/y/click");
uint64_t Y_TOUCH_ID = alvr_path_string_to_hash("/user/hand/left/input/y/touch");
uint64_t LEFT_SQUEEZE_CLICK_ID = alvr_path_string_to_hash("/user/hand/left/input/squeeze/click");
uint64_t LEFT_SQUEEZE_VALUE_ID = alvr_path_string_to_hash("/user/hand/left/input/squeeze/value");
uint64_t LEFT_TRIGGER_CLICK_ID = alvr_path_string_to_hash("/user/hand/left/input/trigger/click");
uint64_t LEFT_TRIGGER_VALUE_ID = alvr_path_string_to_hash("/user/hand/left/input/trigger/value");
uint64_t LEFT_TRIGGER_TOUCH_ID = alvr_path_string_to_hash("/user/hand/left/input/trigger/touch");
uint64_t LEFT_THUMBSTICK_X_ID = alvr_path_string_to_hash("/user/hand/left/input/thumbstick/x");
uint64_t LEFT_THUMBSTICK_Y_ID = alvr_path_string_to_hash("/user/hand/left/input/thumbstick/y");
uint64_t LEFT_THUMBSTICK_CLICK_ID = alvr_path_string_to_hash(
        "/user/hand/left/input/thumbstick/click");
uint64_t LEFT_THUMBSTICK_TOUCH_ID = alvr_path_string_to_hash(
        "/user/hand/left/input/thumbstick/touch");
uint64_t LEFT_THUMBREST_TOUCH_ID = alvr_path_string_to_hash(
        "/user/hand/left/input/thumbrest/touch");
uint64_t RIGHT_SQUEEZE_CLICK_ID = alvr_path_string_to_hash("/user/hand/right/input/squeeze/click");
uint64_t RIGHT_SQUEEZE_VALUE_ID = alvr_path_string_to_hash("/user/hand/right/input/squeeze/value");
uint64_t RIGHT_TRIGGER_CLICK_ID = alvr_path_string_to_hash("/user/hand/right/input/trigger/click");
uint64_t RIGHT_TRIGGER_VALUE_ID = alvr_path_string_to_hash("/user/hand/right/input/trigger/value");
uint64_t RIGHT_TRIGGER_TOUCH_ID = alvr_path_string_to_hash("/user/hand/right/input/trigger/touch");
uint64_t RIGHT_THUMBSTICK_X_ID = alvr_path_string_to_hash("/user/hand/right/input/thumbstick/x");
uint64_t RIGHT_THUMBSTICK_Y_ID = alvr_path_string_to_hash("/user/hand/right/input/thumbstick/y");
uint64_t RIGHT_THUMBSTICK_CLICK_ID = alvr_path_string_to_hash(
        "/user/hand/right/input/thumbstick/click");
uint64_t RIGHT_THUMBSTICK_TOUCH_ID = alvr_path_string_to_hash(
        "/user/hand/right/input/thumbstick/touch");
uint64_t RIGHT_THUMBREST_TOUCH_ID = alvr_path_string_to_hash(
        "/user/hand/right/input/thumbrest/touch");

const int MAXIMUM_TRACKING_FRAMES = 360;
// minimum change for a scalar button to be registered as a new value
const float BUTTON_EPS = 0.001;
const float IPD_EPS = 0.001; // minimum change of IPD to be registered as a new value

const GLenum SWAPCHAIN_FORMAT = GL_RGBA8;

struct Render_EGL {
    EGLDisplay Display;
    EGLConfig Config;
    EGLSurface TinySurface;
    EGLSurface MainSurface;
    EGLContext Context;
};

struct Swapchain {
    ovrTextureSwapChain *inner;
    int index;
};

class NativeContext {
public:
    JavaVM *vm;
    jobject context;

    Render_EGL egl;

    ANativeWindow *window = nullptr;
    ovrMobile *ovrContext{};

    bool running = false;
    bool streaming = false;
    std::thread eventsThread;

    uint32_t recommendedViewWidth = 1;
    uint32_t recommendedViewHeight = 1;
    float refreshRate = 72.f;
    StreamingStarted_Body streamingConfig = {};

    uint64_t ovrFrameIndex = 0;

    std::deque<std::pair<uint64_t, ovrTracking2>> trackingFrameMap;
    std::mutex trackingFrameMutex;

    Swapchain lobbySwapchains[2] = {};
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

NativeContext CTX = {};

static const char *EglErrorString(const EGLint err) {
    switch (err) {
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

    CTX.egl.Display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    eglInitialize(CTX.egl.Display, &major, &minor);

    // Do NOT use eglChooseConfig, because the Android EGL code pushes in multisample
    // flags in eglChooseConfig if the user has selected the "force 4x MSAA" option in
    // settings, and that is completely wasted for our warp target.
    const int MAX_CONFIGS = 1024;
    EGLConfig configs[MAX_CONFIGS];
    EGLint numConfigs = 0;
    if (eglGetConfigs(CTX.egl.Display, configs, MAX_CONFIGS, &numConfigs) == EGL_FALSE) {
        error("        eglGetConfigs() failed: %s", EglErrorString(eglGetError()));
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
    CTX.egl.Config = 0;
    for (int i = 0; i < numConfigs; i++) {
        EGLint value = 0;

        eglGetConfigAttrib(CTX.egl.Display, configs[i], EGL_RENDERABLE_TYPE, &value);
        if ((value & EGL_OPENGL_ES3_BIT_KHR) != EGL_OPENGL_ES3_BIT_KHR) {
            continue;
        }

        // The pbuffer config also needs to be compatible with normal window rendering
        // so it can share textures with the window context.
        eglGetConfigAttrib(CTX.egl.Display, configs[i], EGL_SURFACE_TYPE, &value);
        if ((value & (EGL_WINDOW_BIT | EGL_PBUFFER_BIT)) != (EGL_WINDOW_BIT | EGL_PBUFFER_BIT)) {
            continue;
        }

        int j = 0;
        for (; configAttribs[j] != EGL_NONE; j += 2) {
            eglGetConfigAttrib(CTX.egl.Display, configs[i], configAttribs[j], &value);
            if (value != configAttribs[j + 1]) {
                break;
            }
        }
        if (configAttribs[j] == EGL_NONE) {
            CTX.egl.Config = configs[i];
            break;
        }
    }
    if (CTX.egl.Config == 0) {
        error("        eglChooseConfig() failed: %s", EglErrorString(eglGetError()));
        return;
    }
    EGLint contextAttribs[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
    CTX.egl.Context = eglCreateContext(CTX.egl.Display, CTX.egl.Config, EGL_NO_CONTEXT,
                                       contextAttribs);
    if (CTX.egl.Context == EGL_NO_CONTEXT) {
        error("        eglCreateContext() failed: %s", EglErrorString(eglGetError()));
        return;
    }
    const EGLint surfaceAttribs[] = {EGL_WIDTH, 16, EGL_HEIGHT, 16, EGL_NONE};
    CTX.egl.TinySurface = eglCreatePbufferSurface(CTX.egl.Display, CTX.egl.Config, surfaceAttribs);
    if (CTX.egl.TinySurface == EGL_NO_SURFACE) {
        error("        eglCreatePbufferSurface() failed: %s", EglErrorString(eglGetError()));
        eglDestroyContext(CTX.egl.Display, CTX.egl.Context);
        CTX.egl.Context = EGL_NO_CONTEXT;
        return;
    }
    if (eglMakeCurrent(CTX.egl.Display, CTX.egl.TinySurface, CTX.egl.TinySurface,
                       CTX.egl.Context) == EGL_FALSE) {
        error("        eglMakeCurrent() failed: %s", EglErrorString(eglGetError()));
        eglDestroySurface(CTX.egl.Display, CTX.egl.TinySurface);
        eglDestroyContext(CTX.egl.Display, CTX.egl.Context);
        CTX.egl.Context = EGL_NO_CONTEXT;
        return;
    }
}

void eglDestroy() {
    if (CTX.egl.Display != 0) {
        error("        eglMakeCurrent( Display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT )");
        if (eglMakeCurrent(CTX.egl.Display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT) ==
            EGL_FALSE) {
            error("        eglMakeCurrent() failed: %s", EglErrorString(eglGetError()));
        }
    }
    if (CTX.egl.Context != EGL_NO_CONTEXT) {
        error("        eglDestroyContext( Display, Context )");
        if (eglDestroyContext(CTX.egl.Display, CTX.egl.Context) == EGL_FALSE) {
            error("        eglDestroyContext() failed: %s", EglErrorString(eglGetError()));
        }
        CTX.egl.Context = EGL_NO_CONTEXT;
    }
    if (CTX.egl.TinySurface != EGL_NO_SURFACE) {
        error("        eglDestroySurface( Display, TinySurface )");
        if (eglDestroySurface(CTX.egl.Display, CTX.egl.TinySurface) == EGL_FALSE) {
            error("        eglDestroySurface() failed: %s", EglErrorString(eglGetError()));
        }
        CTX.egl.TinySurface = EGL_NO_SURFACE;
    }
    if (CTX.egl.Display != 0) {
        error("        eglTerminate( Display )");
        if (eglTerminate(CTX.egl.Display) == EGL_FALSE) {
            error("        eglTerminate() failed: %s", EglErrorString(eglGetError()));
        }
        CTX.egl.Display = 0;
    }
}

inline uint64_t getTimestampUs() {
    timeval tv;
    gettimeofday(&tv, nullptr);

    uint64_t Current = (uint64_t) tv.tv_sec * 1000 * 1000 + tv.tv_usec;
    return Current;
}

ovrJava getOvrJava(bool initThread = false) {
    JNIEnv *env;
    if (initThread) {
        JavaVMAttachArgs args = {JNI_VERSION_1_6};
        CTX.vm->AttachCurrentThread(&env, &args);
    } else {
        CTX.vm->GetEnv((void **) &env, JNI_VERSION_1_6);
    }

    ovrJava java{};
    java.Vm = CTX.vm;
    java.Env = env;
    java.ActivityObject = CTX.context;

    return java;
}

void updateBinary(uint64_t path, uint32_t flag) {
    auto value = flag != 0;
    auto *stateRef = &CTX.previousButtonsState[path];
    if (stateRef->binary != value) {
        stateRef->tag = ALVR_BUTTON_VALUE_BINARY;
        stateRef->binary = value;

        alvr_send_button(path, *stateRef);
    }
}

void updateScalar(uint64_t path, float value) {
    auto *stateRef = &CTX.previousButtonsState[path];
    if (abs(stateRef->scalar - value) > BUTTON_EPS) {
        stateRef->tag = ALVR_BUTTON_VALUE_SCALAR;
        stateRef->scalar = value;

        alvr_send_button(path, *stateRef);
    }
}

void updateButtons() {
    ovrInputCapabilityHeader capabilitiesHeader;
    uint32_t deviceIndex = 0;
    while (vrapi_EnumerateInputDevices(CTX.ovrContext, deviceIndex, &capabilitiesHeader) >= 0) {
        if (capabilitiesHeader.Type == ovrControllerType_TrackedRemote) {
            ovrInputTrackedRemoteCapabilities capabilities = {};
            capabilities.Header = capabilitiesHeader;
            if (vrapi_GetInputDeviceCapabilities(CTX.ovrContext, &capabilities.Header) !=
                ovrSuccess) {
                continue;
            }

            ovrInputStateTrackedRemote inputState = {};
            inputState.Header.ControllerType = capabilities.Header.Type;
            if (vrapi_GetCurrentInputState(CTX.ovrContext,
                                           capabilities.Header.DeviceID,
                                           &inputState.Header) != ovrSuccess) {
                continue;
            }

            if (capabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) {
                updateBinary(MENU_CLICK_ID, inputState.Buttons & ovrButton_Enter);
                updateBinary(X_CLICK_ID, inputState.Buttons & ovrButton_X);
                updateBinary(X_TOUCH_ID, inputState.Touches & ovrTouch_X);
                updateBinary(Y_CLICK_ID, inputState.Buttons & ovrButton_Y);
                updateBinary(Y_TOUCH_ID, inputState.Touches & ovrTouch_Y);
                updateBinary(LEFT_SQUEEZE_CLICK_ID, inputState.Buttons & ovrButton_GripTrigger);
                updateScalar(LEFT_SQUEEZE_VALUE_ID, inputState.GripTrigger);
                updateBinary(LEFT_TRIGGER_CLICK_ID, inputState.Buttons & ovrButton_Trigger);
                updateScalar(LEFT_TRIGGER_VALUE_ID, inputState.IndexTrigger);
                updateBinary(LEFT_TRIGGER_TOUCH_ID, inputState.Touches & ovrTouch_IndexTrigger);
                updateScalar(LEFT_THUMBSTICK_X_ID, inputState.Joystick.x);
                updateScalar(LEFT_THUMBSTICK_Y_ID, inputState.Joystick.y);
                updateBinary(LEFT_THUMBSTICK_CLICK_ID, inputState.Buttons & ovrButton_Joystick);
                updateBinary(LEFT_THUMBSTICK_TOUCH_ID, inputState.Touches & ovrTouch_LThumb);
                updateBinary(LEFT_THUMBREST_TOUCH_ID, inputState.Touches & ovrTouch_ThumbRest);
            } else {
                updateBinary(A_CLICK_ID, inputState.Buttons & ovrButton_A);
                updateBinary(A_TOUCH_ID, inputState.Touches & ovrTouch_A);
                updateBinary(B_CLICK_ID, inputState.Buttons & ovrButton_B);
                updateBinary(B_TOUCH_ID, inputState.Touches & ovrTouch_B);
                updateBinary(RIGHT_SQUEEZE_CLICK_ID, inputState.Buttons & ovrButton_GripTrigger);
                updateScalar(RIGHT_SQUEEZE_VALUE_ID, inputState.GripTrigger);
                updateBinary(RIGHT_TRIGGER_CLICK_ID, inputState.Buttons & ovrButton_Trigger);
                updateScalar(RIGHT_TRIGGER_VALUE_ID, inputState.IndexTrigger);
                updateBinary(RIGHT_TRIGGER_TOUCH_ID, inputState.Touches & ovrTouch_IndexTrigger);
                updateScalar(RIGHT_THUMBSTICK_X_ID, inputState.Joystick.x);
                updateScalar(RIGHT_THUMBSTICK_Y_ID, inputState.Joystick.y);
                updateBinary(RIGHT_THUMBSTICK_CLICK_ID, inputState.Buttons & ovrButton_Joystick);
                updateBinary(RIGHT_THUMBSTICK_TOUCH_ID, inputState.Touches & ovrTouch_RThumb);
                updateBinary(RIGHT_THUMBREST_TOUCH_ID, inputState.Touches & ovrTouch_ThumbRest);
            }
        }

        deviceIndex++;
    }
}

// return fov in OpenXR convention
EyeFov getFov(ovrTracking2 tracking, int eye) {
    // ovrTracking2 tracking = vrapi_GetPredictedTracking2(CTX.ovrContext, 0.0);

    EyeFov fov;
    auto projection = tracking.Eye[eye].ProjectionMatrix;
    double a = projection.M[0][0];
    double b = projection.M[1][1];
    double c = projection.M[0][2];
    double d = projection.M[1][2];

    fov.left = (float) atan((c - 1) / a);
    fov.right = (float) atan((c + 1) / a);
    fov.top = -(float) atan((d - 1) / b);
    fov.bottom = -(float) atan((d + 1) / b);

    return fov;
}

void getPlayspaceArea(float *width, float *height) {
    ovrPosef spacePose;
    ovrVector3f bboxScale;
    // Theoretically pose (the 2nd parameter) could be nullptr, since we already have that, but
    // then this function gives us 0-size bounding box, so it has to be provided.
    vrapi_GetBoundaryOrientedBoundingBox(CTX.ovrContext, &spacePose, &bboxScale);
    *width = 2.0f * bboxScale.x;
    *height = 2.0f * bboxScale.z;
}

uint8_t getControllerBattery(int index) {
    ovrInputCapabilityHeader curCaps;
    auto result = vrapi_EnumerateInputDevices(CTX.ovrContext, index, &curCaps);
    if (result < 0 || curCaps.Type != ovrControllerType_TrackedRemote) {
        return 0;
    }

    ovrInputTrackedRemoteCapabilities remoteCapabilities;
    remoteCapabilities.Header = curCaps;
    result = vrapi_GetInputDeviceCapabilities(CTX.ovrContext, &remoteCapabilities.Header);
    if (result != ovrSuccess) {
        return 0;
    }

    ovrInputStateTrackedRemote remoteInputState;
    remoteInputState.Header.ControllerType = remoteCapabilities.Header.Type;
    result = vrapi_GetCurrentInputState(
            CTX.ovrContext, remoteCapabilities.Header.DeviceID, &remoteInputState.Header);
    if (result != ovrSuccess) {
        return 0;
    }

    return remoteInputState.BatteryPercentRemaining;
}

void finishHapticsBuffer(ovrDeviceID DeviceID) {
    uint8_t hapticBuffer[1] = {0};
    ovrHapticBuffer buffer;
    buffer.BufferTime = vrapi_GetPredictedDisplayTime(CTX.ovrContext, CTX.ovrFrameIndex);
    buffer.HapticBuffer = &hapticBuffer[0];
    buffer.NumSamples = 1;
    buffer.Terminated = true;

    auto result = vrapi_SetHapticVibrationBuffer(CTX.ovrContext, DeviceID, &buffer);
    if (result != ovrSuccess) {
        info("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
    }
}

void updateHapticsState() {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(CTX.ovrContext, deviceIndex, &curCaps) >= 0;
         deviceIndex++) {

        if (curCaps.Type != ovrControllerType_TrackedRemote)
            continue;

        ovrInputTrackedRemoteCapabilities remoteCapabilities;

        remoteCapabilities.Header = curCaps;
        result = vrapi_GetInputDeviceCapabilities(CTX.ovrContext, &remoteCapabilities.Header);
        if (result != ovrSuccess) {
            continue;
        }

        int curHandIndex =
                (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) ? 1 : 0;
        auto &s = CTX.hapticsState[curHandIndex];

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

            auto requiredHapticsBuffer = static_cast<uint32_t>(
                    (s.endUs - currentUs) / (remoteCapabilities.HapticSampleDurationMS * 1000));

            std::vector<uint8_t> hapticBuffer(remoteCapabilities.HapticSamplesMax);
            ovrHapticBuffer buffer;
            buffer.BufferTime =
                    vrapi_GetPredictedDisplayTime(CTX.ovrContext, CTX.ovrFrameIndex);
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

            result = vrapi_SetHapticVibrationBuffer(CTX.ovrContext, curCaps.DeviceID, &buffer);
            if (result != ovrSuccess) {
                info("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
            }
            s.buffered = true;
        } else if (remoteCapabilities.ControllerCapabilities &
                   ovrControllerCaps_HasSimpleHapticVibration) {
            vrapi_SetHapticVibrationSimple(CTX.ovrContext, curCaps.DeviceID, s.amplitude);
        }
    }
}

// low frequency events.
// This thread gets created after the creation of ovrContext and before its destruction
void eventsThread() {
    auto java = getOvrJava(true);

    jclass cls = java.Env->GetObjectClass(java.ActivityObject);
    jmethodID onStreamStartMethod = java.Env->GetMethodID(cls, "onStreamStart", "()V");
    jmethodID onStreamStopMethod = java.Env->GetMethodID(cls, "onStreamStop", "()V");

    auto deadline = std::chrono::steady_clock::now();
    auto motionVec = std::vector<AlvrDeviceMotion>();

    int recenterCount = 0;

    while (CTX.running) {
        if (CTX.streaming) {
            motionVec.clear();
            OculusHand leftHand = {false};
            OculusHand rightHand = {false};

            AlvrDeviceMotion headMotion = {};
            uint64_t targetTimestampNs =
                    vrapi_GetTimeInSeconds() * 1e9 + alvr_get_prediction_offset_ns();
            auto headTracking =
                    vrapi_GetPredictedTracking2(CTX.ovrContext, (double) targetTimestampNs / 1e9);
            headMotion.device_id = HEAD_ID;
            memcpy(&headMotion.orientation, &headTracking.HeadPose.Pose.Orientation, 4 * 4);
            memcpy(headMotion.position, &headTracking.HeadPose.Pose.Position, 4 * 3);
            // Note: do not copy velocities. Avoid reprojection in SteamVR
            motionVec.push_back(headMotion);

            {
                std::lock_guard<std::mutex> lock(CTX.trackingFrameMutex);
                // Insert from the front: it will be searched first
                CTX.trackingFrameMap.push_front({targetTimestampNs, headTracking});
                if (CTX.trackingFrameMap.size() > MAXIMUM_TRACKING_FRAMES) {
                    CTX.trackingFrameMap.pop_back();
                }
            }

            updateButtons();

            double controllerDisplayTimeS =
                    vrapi_GetTimeInSeconds() + (double) alvr_get_prediction_offset_ns() / 1e9 *
                                               CTX.streamingConfig.controller_prediction_multiplier;

            ovrInputCapabilityHeader capabilitiesHeader;
            uint32_t deviceIndex = 0;
            while (vrapi_EnumerateInputDevices(CTX.ovrContext, deviceIndex, &capabilitiesHeader) >=
                   0) {
                if (capabilitiesHeader.Type == ovrControllerType_TrackedRemote) {
                    ovrInputTrackedRemoteCapabilities capabilities = {};
                    capabilities.Header = capabilitiesHeader;
                    if (vrapi_GetInputDeviceCapabilities(CTX.ovrContext, &capabilities.Header) !=
                        ovrSuccess) {
                        continue;
                    }

                    uint64_t handID;
                    if (capabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) {
                        handID = LEFT_HAND_ID;
                    } else {
                        handID = RIGHT_HAND_ID;
                    }

                    ovrTracking tracking = {};
                    if (vrapi_GetInputTrackingState(CTX.ovrContext,
                                                    capabilities.Header.DeviceID,
                                                    controllerDisplayTimeS,
                                                    &tracking) == ovrSuccess) {
                        if(((tracking.Status & VRAPI_TRACKING_STATUS_POSITION_VALID) && (tracking.Status & VRAPI_TRACKING_STATUS_ORIENTATION_VALID)) ||
                            (capabilities.ControllerCapabilities & ovrControllerCaps_ModelOculusGo)) {
                            AlvrDeviceMotion motion = {};
                            motion.device_id = handID;
                            memcpy(&motion.orientation, &tracking.HeadPose.Pose.Orientation, 4 * 4);
                            memcpy(motion.position, &tracking.HeadPose.Pose.Position, 4 * 3);
                            memcpy(motion.linear_velocity, &tracking.HeadPose.LinearVelocity, 4 * 3);
                            memcpy(motion.angular_velocity, &tracking.HeadPose.AngularVelocity, 4 * 3);

                            motionVec.push_back(motion);
                        }
                    }
                } else if (capabilitiesHeader.Type == ovrControllerType_Hand) {
                    ovrInputHandCapabilities capabilities;
                    capabilities.Header = capabilitiesHeader;
                    if (vrapi_GetInputDeviceCapabilities(CTX.ovrContext, &capabilities.Header) !=
                        ovrSuccess) {
                        continue;
                    }

                    uint64_t handID;
                    OculusHand *handRef = nullptr;
                    if (capabilities.HandCapabilities & ovrHandCaps_LeftHand) {
                        handID = LEFT_HAND_ID;
                        handRef = &leftHand;
                    } else {
                        handID = RIGHT_HAND_ID;
                        handRef = &rightHand;
                    }

                    ovrHandPose handPose;
                    handPose.Header.Version = ovrHandVersion_1;
                    if (vrapi_GetHandPose(CTX.ovrContext,
                                          capabilities.Header.DeviceID,
                                          controllerDisplayTimeS,
                                          &handPose.Header) == ovrSuccess &&
                        (handPose.Status & ovrHandTrackingStatus_Tracked)) {
                        AlvrDeviceMotion motion = {};
                        motion.device_id = handID;
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

            alvr_send_tracking(targetTimestampNs, &motionVec[0], motionVec.size(), leftHand,
                               rightHand);

        }


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

        ovrTracking2 tracking = vrapi_GetPredictedTracking2(CTX.ovrContext, 0.0);
        auto newLeftFov = getFov(tracking, 0);
        auto newRightFov = getFov(tracking, 1);
        float newIpd = vrapi_GetInterpupillaryDistance(&tracking);

        if (abs(newIpd - CTX.lastIpd) > IPD_EPS ||
            abs(newLeftFov.left - CTX.lastFov.left) > IPD_EPS) {
            EyeFov fov[2] = {newLeftFov, newRightFov};
            alvr_send_views_config(fov, newIpd);
            CTX.lastIpd = newIpd;
            CTX.lastFov = newLeftFov;
        }

        uint8_t leftBattery = getControllerBattery(0);
        if (leftBattery != CTX.lastLeftControllerBattery) {
            alvr_send_battery(LEFT_HAND_ID, (float) leftBattery / 100.f, false);
            CTX.lastLeftControllerBattery = leftBattery;
        }
        uint8_t rightBattery = getControllerBattery(1);
        if (rightBattery != CTX.lastRightControllerBattery) {
            alvr_send_battery(RIGHT_HAND_ID, (float) rightBattery / 100.f, false);
            CTX.lastRightControllerBattery = rightBattery;
        }

        AlvrEvent event;
        while (alvr_poll_event(&event)) {
            if (event.tag == ALVR_EVENT_HAPTICS) {
                auto haptics = event.HAPTICS;
                int curHandIndex = (haptics.device_id == RIGHT_CONTROLLER_HAPTICS_ID ? 0 : 1);
                auto &s = CTX.hapticsState[curHandIndex];
                s.startUs = 0;
                s.endUs = (uint64_t) (haptics.duration_s * 1000'000);
                s.amplitude = haptics.amplitude;
                s.frequency = haptics.frequency;
                s.fresh = true;
                s.buffered = false;
            } else if (event.tag == ALVR_EVENT_STREAMING_STARTED) {
                CTX.streamingConfig = event.STREAMING_STARTED;
                java.Env->CallVoidMethod(java.ActivityObject, onStreamStartMethod);
            } else if (event.tag == ALVR_EVENT_STREAMING_STOPPED) {
                java.Env->CallVoidMethod(java.ActivityObject, onStreamStopMethod);
            } else if (event.tag == ALVR_EVENT_NAL_READY) {
                // unused and unreachable
            }
        }

        deadline += std::chrono::nanoseconds((uint64_t) (1e9 / CTX.refreshRate / 3));
        std::this_thread::sleep_until(deadline);
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_initializeNative(JNIEnv *env, jobject context) {
    env->GetJavaVM(&CTX.vm);
    CTX.context = env->NewGlobalRef(context);

    auto java = getOvrJava(true);

    eglInit();

    memset(CTX.hapticsState, 0, sizeof(CTX.hapticsState));
    const ovrInitParms initParms = vrapi_DefaultInitParms(&java);
    vrapi_Initialize(&initParms);

    CTX.recommendedViewWidth =
            vrapi_GetSystemPropertyInt(&java, VRAPI_SYS_PROP_DISPLAY_PIXELS_WIDE) / 2;
    CTX.recommendedViewHeight =
            vrapi_GetSystemPropertyInt(&java, VRAPI_SYS_PROP_DISPLAY_PIXELS_HIGH);

    auto refreshRatesCount =
            vrapi_GetSystemPropertyInt(&java, VRAPI_SYS_PROP_NUM_SUPPORTED_DISPLAY_REFRESH_RATES);
    auto refreshRatesBuffer = std::vector<float>(refreshRatesCount);
    vrapi_GetSystemPropertyFloatArray(&java,
                                      VRAPI_SYS_PROP_SUPPORTED_DISPLAY_REFRESH_RATES,
                                      &refreshRatesBuffer[0],
                                      refreshRatesCount);

    alvr_initialize((void *) CTX.vm,
                    (void *) CTX.context,
                    CTX.recommendedViewWidth,
                    CTX.recommendedViewHeight,
                    &refreshRatesBuffer[0],
                    refreshRatesCount,
                    false);
    alvr_initialize_opengl();
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_destroyNative(JNIEnv *_env, jobject _context) {
    vrapi_Shutdown();

    alvr_destroy();
    alvr_destroy_opengl();

    eglDestroy();

    auto java = getOvrJava();
    java.Env->DeleteGlobalRef(CTX.context);
}

extern "C" JNIEXPORT void JNICALL Java_com_polygraphene_alvr_OvrActivity_onResumeNative(
        JNIEnv *_env, jobject _context, jobject surface) {
    auto java = getOvrJava();

    CTX.window = ANativeWindow_fromSurface(java.Env, surface);

    info("Entering VR mode.");

    ovrModeParms parms = vrapi_DefaultModeParms(&java);

    parms.Flags |= VRAPI_MODE_FLAG_RESET_WINDOW_FULLSCREEN;

    parms.Flags |= VRAPI_MODE_FLAG_NATIVE_WINDOW;
    parms.Display = (size_t) CTX.egl.Display;
    parms.WindowSurface = (size_t) CTX.window;
    parms.ShareContext = (size_t) CTX.egl.Context;

    CTX.ovrContext = vrapi_EnterVrMode(&parms);

    if (CTX.ovrContext == nullptr) {
        error("Invalid ANativeWindow");
    }

    // set Color Space
    ovrHmdColorDesc colorDesc{};
    colorDesc.ColorSpace = VRAPI_COLORSPACE_RIFT_S;
    vrapi_SetClientColorDesc(CTX.ovrContext, &colorDesc);

    vrapi_SetPerfThread(CTX.ovrContext, VRAPI_PERF_THREAD_TYPE_MAIN, gettid());

    vrapi_SetTrackingSpace(CTX.ovrContext, VRAPI_TRACKING_SPACE_STAGE);

    std::vector<int32_t> textureHandlesBuffer[2];
    for (int eye = 0; eye < 2; eye++) {
        CTX.lobbySwapchains[eye].inner =
                vrapi_CreateTextureSwapChain3(VRAPI_TEXTURE_TYPE_2D,
                                              SWAPCHAIN_FORMAT,
                                              CTX.recommendedViewWidth,
                                              CTX.recommendedViewHeight,
                                              1,
                                              3);
        int size = vrapi_GetTextureSwapChainLength(CTX.lobbySwapchains[eye].inner);

        for (int index = 0; index < size; index++) {
            auto handle =
                    vrapi_GetTextureSwapChainHandle(CTX.lobbySwapchains[eye].inner, index);
            textureHandlesBuffer[eye].push_back(handle);
        }

        CTX.lobbySwapchains[eye].index = 0;
    }
    const int32_t *textureHandles[2] = {&textureHandlesBuffer[0][0], &textureHandlesBuffer[1][0]};

    CTX.running = true;
    CTX.eventsThread = std::thread(eventsThread);

    alvr_resume_opengl(CTX.recommendedViewWidth, CTX.recommendedViewHeight, textureHandles,
                       textureHandlesBuffer[0].size());
    alvr_resume();

    vrapi_SetDisplayRefreshRate(CTX.ovrContext, CTX.refreshRate);
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_onStreamStartNative(JNIEnv *_env, jobject _context) {
    auto java = getOvrJava();

    CTX.refreshRate = CTX.streamingConfig.fps;

    std::vector<int32_t> textureHandlesBuffer[2];
    for (int eye = 0; eye < 2; eye++) {
        CTX.streamSwapchains[eye].inner =
                vrapi_CreateTextureSwapChain3(VRAPI_TEXTURE_TYPE_2D,
                                              SWAPCHAIN_FORMAT,
                                              CTX.streamingConfig.view_width,
                                              CTX.streamingConfig.view_height,
                                              1,
                                              3);
        auto size = vrapi_GetTextureSwapChainLength(CTX.streamSwapchains[eye].inner);

        for (int index = 0; index < size; index++) {
            auto handle = vrapi_GetTextureSwapChainHandle(CTX.streamSwapchains[eye].inner, index);
            textureHandlesBuffer[eye].push_back(handle);
        }

        CTX.streamSwapchains[eye].index = 0;
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
    vrapi_SetExtraLatencyMode(CTX.ovrContext,
                              (ovrExtraLatencyMode) CTX.streamingConfig.extra_latency);

    ovrResult result = vrapi_SetDisplayRefreshRate(CTX.ovrContext, CTX.refreshRate);
    if (result != ovrSuccess) {
        error("Failed to set refresh rate requested by the server: %d", result);
    }

    vrapi_SetPropertyInt(
            &java, VRAPI_FOVEATION_LEVEL, CTX.streamingConfig.oculus_foveation_level);
    vrapi_SetPropertyInt(
            &java, VRAPI_DYNAMIC_FOVEATION_ENABLED, CTX.streamingConfig.dynamic_oculus_foveation);

    ovrTracking2 tracking = vrapi_GetPredictedTracking2(CTX.ovrContext, 0.0);
    EyeFov fovArr[2] = {getFov(tracking, 0), getFov(tracking, 1)};
    float ipd = vrapi_GetInterpupillaryDistance(&tracking);
    alvr_send_views_config(fovArr, ipd);

    alvr_send_battery(HEAD_ID, CTX.hmdBattery, CTX.hmdPlugged);
    alvr_send_battery(LEFT_HAND_ID, getControllerBattery(0) / 100.f, false);
    alvr_send_battery(RIGHT_HAND_ID, getControllerBattery(1) / 100.f, false);

    float areaWidth, areaHeight;
    getPlayspaceArea(&areaWidth, &areaHeight);
    alvr_send_playspace(areaWidth, areaHeight);

    alvr_start_stream_opengl(textureHandles, textureHandlesBuffer[0].size());

    CTX.streaming = true;
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_onStreamStopNative(JNIEnv *_env, jobject _context) {
    CTX.streaming = false;

    if (CTX.streamSwapchains[0].inner != nullptr) {
        vrapi_DestroyTextureSwapChain(CTX.streamSwapchains[0].inner);
        vrapi_DestroyTextureSwapChain(CTX.streamSwapchains[1].inner);
        CTX.streamSwapchains[0].inner = nullptr;
        CTX.streamSwapchains[1].inner = nullptr;
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_onPauseNative(JNIEnv *_env, jobject _context) {
    Java_com_polygraphene_alvr_OvrActivity_onStreamStopNative(_env, _context);

    alvr_pause();
    alvr_pause_opengl();

    if (CTX.running) {
        CTX.running = false;
        CTX.eventsThread.join();
    }
    if (CTX.lobbySwapchains[0].inner != nullptr) {
        vrapi_DestroyTextureSwapChain(CTX.lobbySwapchains[0].inner);
        vrapi_DestroyTextureSwapChain(CTX.lobbySwapchains[1].inner);
        CTX.lobbySwapchains[0].inner = nullptr;
        CTX.lobbySwapchains[1].inner = nullptr;
    }

    vrapi_LeaveVrMode(CTX.ovrContext);

    CTX.ovrContext = nullptr;

    if (CTX.window != nullptr) {
        ANativeWindow_release(CTX.window);
    }
    CTX.window = nullptr;
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_renderNative(JNIEnv *_env, jobject _context) {
    ovrLayerProjection2 worldLayer = vrapi_DefaultLayerProjection2();

    double displayTime;
    ovrTracking2 tracking;

    if (CTX.streaming) {
        void *streamHardwareBuffer = nullptr;
        auto timestampNs = alvr_get_frame(&streamHardwareBuffer);
        displayTime = (double) timestampNs / 1e9;

        if (timestampNs == -1) {
            return;
        }

        updateHapticsState();

        {
            std::lock_guard<std::mutex> lock(CTX.trackingFrameMutex);

            // Take the frame with equal timestamp, or the next closest one.
            for (auto &pair: CTX.trackingFrameMap) {
                if (pair.first <= timestampNs) {
                    tracking = pair.second;
                    break;
                }
            }
        }

        int swapchainIndices[2] = {CTX.streamSwapchains[0].index,
                                   CTX.streamSwapchains[1].index};
        alvr_render_stream_opengl(streamHardwareBuffer, swapchainIndices);

        double vsyncQueueS = vrapi_GetPredictedDisplayTime(CTX.ovrContext, CTX.ovrFrameIndex) -
                             vrapi_GetTimeInSeconds();
        alvr_report_submit(timestampNs, vsyncQueueS * 1e9);

        worldLayer.HeadPose = tracking.HeadPose;
        for (int eye = 0; eye < 2; eye++) {
            worldLayer.Textures[eye].ColorSwapChain = CTX.streamSwapchains[eye].inner;
            worldLayer.Textures[eye].SwapChainIndex = CTX.streamSwapchains[eye].index;
            CTX.streamSwapchains[eye].index = (CTX.streamSwapchains[eye].index + 1) % 3;
        }
    } else {
        displayTime = vrapi_GetPredictedDisplayTime(CTX.ovrContext, CTX.ovrFrameIndex);
        tracking = vrapi_GetPredictedTracking2(CTX.ovrContext, displayTime);

        AlvrEyeInput eyeInputs[2] = {};
        int swapchainIndices[2] = {};
        for (int eye = 0; eye < 2; eye++) {
            auto q = tracking.HeadPose.Pose.Orientation;
            auto v = ovrMatrix4f_Inverse(&tracking.Eye[eye].ViewMatrix);

            eyeInputs[eye].orientation = AlvrQuat{q.x, q.y, q.z, q.w};
            eyeInputs[eye].position[0] = v.M[0][3];
            eyeInputs[eye].position[1] = v.M[1][3];
            eyeInputs[eye].position[2] = v.M[2][3];
            eyeInputs[eye].fov = getFov(tracking, eye);

            swapchainIndices[eye] = CTX.lobbySwapchains[eye].index;
        }
        alvr_render_lobby_opengl(eyeInputs, swapchainIndices);

        for (int eye = 0; eye < 2; eye++) {
            worldLayer.Textures[eye].ColorSwapChain = CTX.lobbySwapchains[eye].inner;
            worldLayer.Textures[eye].SwapChainIndex = CTX.lobbySwapchains[eye].index;
            CTX.lobbySwapchains[eye].index = (CTX.lobbySwapchains[eye].index + 1) % 3;
        }
    }

    for (int eye = 0; eye < 2; eye++) {
        worldLayer.Textures[eye].TexCoordsFromTanAngles =
                ovrMatrix4f_TanAngleMatrixFromProjection(&tracking.Eye[eye].ProjectionMatrix);
    }

    worldLayer.HeadPose = tracking.HeadPose;

    const ovrLayerHeader2 *layers[] = {&worldLayer.Header};

    ovrSubmitFrameDescription2 frameDesc = {};
    frameDesc.Flags = 0;
    frameDesc.SwapInterval = 1;
    frameDesc.FrameIndex = CTX.ovrFrameIndex;
    frameDesc.DisplayTime = displayTime;
    frameDesc.LayerCount = 1;
    frameDesc.Layers = layers;

    vrapi_SubmitFrame2(CTX.ovrContext, &frameDesc);

    CTX.ovrFrameIndex++;
}

extern "C" JNIEXPORT void JNICALL Java_com_polygraphene_alvr_OvrActivity_onBatteryChangedNative(
        JNIEnv *_env, jobject _context, jint battery, jboolean plugged) {
    alvr_send_battery(HEAD_ID, (float) battery / 100.f, (bool) plugged);
    CTX.hmdBattery = battery;
    CTX.hmdPlugged = plugged;
}
