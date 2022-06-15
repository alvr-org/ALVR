#include "VrApi.h"
#include "VrApi_Helpers.h"
#include "VrApi_Input.h"
#include "VrApi_SystemUtils.h"
#include "VrApi_Types.h"
#include "alvr_client_core.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <android/input.h>
#include <android/log.h>
#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <chrono>
#include <glm/gtc/quaternion.hpp>
#include <glm/mat4x4.hpp>
#include <inttypes.h>
#include <jni.h>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <unistd.h>
#include <vector>

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

enum ALVR_INPUT {
    ALVR_INPUT_SYSTEM_CLICK,
    ALVR_INPUT_APPLICATION_MENU_CLICK,
    ALVR_INPUT_GRIP_CLICK,
    ALVR_INPUT_GRIP_VALUE,
    ALVR_INPUT_GRIP_TOUCH,
    ALVR_INPUT_DPAD_LEFT_CLICK,
    ALVR_INPUT_DPAD_UP_CLICK,
    ALVR_INPUT_DPAD_RIGHT_CLICK,
    ALVR_INPUT_DPAD_DOWN_CLICK,
    ALVR_INPUT_A_CLICK,
    ALVR_INPUT_A_TOUCH,
    ALVR_INPUT_B_CLICK,
    ALVR_INPUT_B_TOUCH,
    ALVR_INPUT_X_CLICK,
    ALVR_INPUT_X_TOUCH,
    ALVR_INPUT_Y_CLICK,
    ALVR_INPUT_Y_TOUCH,
    ALVR_INPUT_TRIGGER_LEFT_VALUE,
    ALVR_INPUT_TRIGGER_RIGHT_VALUE,
    ALVR_INPUT_SHOULDER_LEFT_CLICK,
    ALVR_INPUT_SHOULDER_RIGHT_CLICK,
    ALVR_INPUT_JOYSTICK_LEFT_CLICK,
    ALVR_INPUT_JOYSTICK_LEFT_X,
    ALVR_INPUT_JOYSTICK_LEFT_Y,
    ALVR_INPUT_JOYSTICK_RIGHT_CLICK,
    ALVR_INPUT_JOYSTICK_RIGHT_X,
    ALVR_INPUT_JOYSTICK_RIGHT_Y,
    ALVR_INPUT_JOYSTICK_CLICK,
    ALVR_INPUT_JOYSTICK_X,
    ALVR_INPUT_JOYSTICK_Y,
    ALVR_INPUT_JOYSTICK_TOUCH,
    ALVR_INPUT_BACK_CLICK,
    ALVR_INPUT_GUIDE_CLICK,
    ALVR_INPUT_START_CLICK,
    ALVR_INPUT_TRIGGER_CLICK,
    ALVR_INPUT_TRIGGER_VALUE,
    ALVR_INPUT_TRIGGER_TOUCH,
    ALVR_INPUT_TRACKPAD_X,
    ALVR_INPUT_TRACKPAD_Y,
    ALVR_INPUT_TRACKPAD_CLICK,
    ALVR_INPUT_TRACKPAD_TOUCH,
    ALVR_INPUT_THUMB_REST_TOUCH,

    ALVR_INPUT_MAX = ALVR_INPUT_THUMB_REST_TOUCH,
    ALVR_INPUT_COUNT = ALVR_INPUT_MAX + 1
};
enum ALVR_HAND_CONFIDENCE {
    alvrThumbConfidence_High = (1 << 0),
    alvrIndexConfidence_High = (1 << 1),
    alvrMiddleConfidence_High = (1 << 2),
    alvrRingConfidence_High = (1 << 3),
    alvrPinkyConfidence_High = (1 << 4),
    alvrHandConfidence_High = (1 << 5),
};
#define ALVR_BUTTON_FLAG(input) (1ULL << (input))

// Must use EGLSyncKHR because the VrApi still supports OpenGL ES 2.0
#define EGL_SYNC

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

const uint32_t ovrButton_Unknown1 = 0x01000000;
const int MAXIMUM_TRACKING_FRAMES = 360;

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
    bool clientsidePrediction;

    uint64_t ovrFrameIndex = 0;

    std::map<uint64_t, ovrTracking2> trackingFrameMap;
    std::mutex trackingFrameMutex;

    Swapchain loadingSwapchains[2] = {};
    Swapchain streamSwapchains[2] = {};

    uint8_t hmdBattery = 0;
    bool hmdPlugged = false;
    uint8_t lastLeftControllerBattery = 0;
    uint8_t lastRightControllerBattery = 0;

    float lastIpd;
    EyeFov lastFov;

    ovrHandPose lastHandPose[2];
    ovrTracking lastTrackingPos[2];

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

uint64_t mapButtons(ovrInputTrackedRemoteCapabilities *remoteCapabilities,
                    ovrInputStateTrackedRemote *remoteInputState) {
    uint64_t buttons = 0;
    if (remoteCapabilities->ControllerCapabilities & ovrControllerCaps_ModelOculusTouch) {
        // Oculus Quest Touch Cotroller
        if (remoteInputState->Buttons & ovrButton_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_B) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_RThumb) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_X) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_X_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Y) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_Y_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_LThumb) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Enter) {
            // Menu button on left hand
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_SYSTEM_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_GripTrigger) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_GRIP_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Trigger) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Joystick) {
            if (remoteCapabilities->ControllerCapabilities & ovrControllerCaps_LeftHand) {
                buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_LEFT_CLICK);
            } else {
                buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_RIGHT_CLICK);
            }
        }
        if (remoteInputState->Buttons & ovrButton_Unknown1) {
            // Only on right controller. What's button???
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_BACK_CLICK);
        }
        if (remoteInputState->Touches & ovrTouch_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_B) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_X) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_X_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_Y) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_Y_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_IndexTrigger) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_Joystick) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_JOYSTICK_TOUCH);
        }
        if (remoteInputState->Touches & ovrTouch_ThumbRest) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_THUMB_REST_TOUCH);
        }
    } else {
        // GearVR or Oculus Go Controller
        if (remoteInputState->Buttons & ovrButton_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_TOUCH);
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Enter) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Back) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_TOUCH);
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_B_CLICK);
        }
        if (remoteInputState->TrackpadStatus) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRACKPAD_TOUCH);
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_A_TOUCH);
        }
    }
    return buttons;
}

void setControllerInfo(TrackingInfo *packet, double displayTime) {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;
    int controller = 0;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(g_ctx.ovrContext, deviceIndex, &curCaps) >= 0;
         deviceIndex++) {
        LOG("Device %d: Type=%d ID=%d", deviceIndex, curCaps.Type, curCaps.DeviceID);
        if (curCaps.Type == ovrControllerType_Hand) { // A3
            ovrInputHandCapabilities handCapabilities;
            ovrInputStateHand inputStateHand;
            handCapabilities.Header = curCaps;

            result = vrapi_GetInputDeviceCapabilities(g_ctx.ovrContext, &handCapabilities.Header);

            if (result != ovrSuccess) {
                continue;
            }

            if ((handCapabilities.HandCapabilities & ovrHandCaps_LeftHand) != 0) {
                controller = 0;
            } else {
                controller = 1;
            }
            inputStateHand.Header.ControllerType = handCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(
                    g_ctx.ovrContext, handCapabilities.Header.DeviceID, &inputStateHand.Header);
            if (result != ovrSuccess) {
                continue;
            }

            auto &c = packet->controller[controller];

            c.enabled = true;
            c.isHand = true;

            memcpy(&c.orientation,
                   &inputStateHand.PointerPose.Orientation,
                   sizeof(inputStateHand.PointerPose.Orientation));
            memcpy(&c.position,
                   &inputStateHand.PointerPose.Position,
                   sizeof(inputStateHand.PointerPose.Position));

            ovrHandedness handedness = handCapabilities.HandCapabilities & ovrHandCaps_LeftHand
                                       ? VRAPI_HAND_LEFT
                                       : VRAPI_HAND_RIGHT;
            ovrHandSkeleton handSkeleton;
            handSkeleton.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandSkeleton(g_ctx.ovrContext, handedness, &handSkeleton.Header) !=
                ovrSuccess) {
                LOG("VrHands - failed to get hand skeleton");
            } else {
                for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                    memcpy(&c.bonePositionsBase[i],
                           &handSkeleton.BonePoses[i].Position,
                           sizeof(handSkeleton.BonePoses[i].Position));
                }
            }

            ovrHandPose handPose;
            handPose.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandPose(g_ctx.ovrContext,
                                  handCapabilities.Header.DeviceID,
                                  displayTime,
                                  &handPose.Header) != ovrSuccess) {
                LOG("VrHands - failed to get hand pose");
            } else {
                if (handPose.HandConfidence == ovrConfidence_HIGH) {
                    c.handFingerConfidences |= alvrHandConfidence_High;
                }
                for (int i = 0; i < ovrHandFinger_Max; i++) {
                    c.handFingerConfidences |=
                            handPose.FingerConfidences[i] == ovrConfidence_HIGH ? (1 << i) : 0;
                }
                if (handPose.Status & ovrHandTrackingStatus_Tracked) {
                    memcpy(&c.boneRootOrientation,
                           &handPose.RootPose.Orientation,
                           sizeof(handPose.RootPose.Orientation));
                    memcpy(&c.boneRootPosition,
                           &handPose.RootPose.Position,
                           sizeof(handPose.RootPose.Position));
                    for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                        memcpy(&c.boneRotations[i],
                               &handPose.BoneRotations[i],
                               sizeof(handPose.BoneRotations[i]));
                    }
                    memcpy(&g_ctx.lastHandPose[controller], &handPose, sizeof(handPose));
                } else if (g_ctx.lastHandPose[controller].Status & ovrHandTrackingStatus_Tracked) {
                    memcpy(&c.boneRootOrientation,
                           &g_ctx.lastHandPose[controller].RootPose.Orientation,
                           sizeof(g_ctx.lastHandPose[controller].RootPose.Orientation));
                    memcpy(&c.boneRootPosition,
                           &g_ctx.lastHandPose[controller].RootPose.Position,
                           sizeof(g_ctx.lastHandPose[controller].RootPose.Position));
                    for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                        memcpy(&c.boneRotations[i],
                               &g_ctx.lastHandPose[controller].BoneRotations[i],
                               sizeof(g_ctx.lastHandPose[controller].BoneRotations[i]));
                    }
                }
            }
        }
        if (curCaps.Type == ovrControllerType_TrackedRemote) {
            ovrInputTrackedRemoteCapabilities remoteCapabilities;
            ovrInputStateTrackedRemote remoteInputState;

            remoteCapabilities.Header = curCaps;
            result = vrapi_GetInputDeviceCapabilities(g_ctx.ovrContext, &remoteCapabilities.Header);
            if (result != ovrSuccess) {
                continue;
            }
            remoteInputState.Header.ControllerType = remoteCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(
                    g_ctx.ovrContext, remoteCapabilities.Header.DeviceID, &remoteInputState.Header);
            if (result != ovrSuccess) {
                continue;
            }

            LOG("ID=%d Cap Controller=%08X Button=%08X Touch=%08X",
                curCaps.DeviceID,
                remoteCapabilities.ControllerCapabilities,
                remoteCapabilities.ButtonCapabilities,
                remoteCapabilities.TouchCapabilities);
            LOG("ID=%d Sta Button=%08X Touch=%08X Joystick=(%f,%f) IndexValue=%f GripValue=%f",
                curCaps.DeviceID,
                remoteInputState.Buttons,
                remoteInputState.Touches,
                remoteInputState.JoystickNoDeadZone.x,
                remoteInputState.JoystickNoDeadZone.y,
                remoteInputState.IndexTrigger,
                remoteInputState.GripTrigger);

            uint64_t hand_path;
            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) != 0) {
                hand_path = LEFT_HAND_PATH;
                controller = 0;
            } else {
                hand_path = RIGHT_HAND_PATH;
                controller = 1;
            }

            auto &c = packet->controller[controller];

            c.enabled = true;

            c.buttons = mapButtons(&remoteCapabilities, &remoteInputState);

            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_HasJoystick) != 0) {
                c.trackpadPosition[0] = remoteInputState.JoystickNoDeadZone.x;
                c.trackpadPosition[1] = remoteInputState.JoystickNoDeadZone.y;
            } else {
                // Normalize to -1.0 - +1.0 for OpenVR Input. y-asix should be reversed.
                c.trackpadPosition[0] =
                        remoteInputState.TrackpadPosition.x / remoteCapabilities.TrackpadMaxX *
                        2.0f -
                        1.0f;
                c.trackpadPosition[1] =
                        remoteInputState.TrackpadPosition.y / remoteCapabilities.TrackpadMaxY *
                        2.0f -
                        1.0f;
                c.trackpadPosition[1] = -c.trackpadPosition[1];
            }
            c.triggerValue = remoteInputState.IndexTrigger;
            c.gripValue = remoteInputState.GripTrigger;

            if (hand_path == LEFT_HAND_PATH) {
                if (remoteInputState.BatteryPercentRemaining != g_ctx.lastLeftControllerBattery) {
                    alvr_send_battery(
                            hand_path, (float) remoteInputState.BatteryPercentRemaining / 100.f,
                            false);
                    g_ctx.lastLeftControllerBattery = remoteInputState.BatteryPercentRemaining;
                }
            } else {
                if (remoteInputState.BatteryPercentRemaining != g_ctx.lastRightControllerBattery) {
                    alvr_send_battery(
                            hand_path, (float) remoteInputState.BatteryPercentRemaining / 100.f,
                            false);
                    g_ctx.lastRightControllerBattery = remoteInputState.BatteryPercentRemaining;
                }
            }

            ovrTracking tracking;
            if (vrapi_GetInputTrackingState(
                    g_ctx.ovrContext, remoteCapabilities.Header.DeviceID, displayTime, &tracking) !=
                ovrSuccess) {
                LOG("vrapi_GetInputTrackingState failed. Device was disconnected?");
            } else {

                memcpy(&c.orientation,
                       &tracking.HeadPose.Pose.Orientation,
                       sizeof(tracking.HeadPose.Pose.Orientation));

                if ((tracking.Status & VRAPI_TRACKING_STATUS_POSITION_TRACKED) ||
                    (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_ModelOculusGo)) {
                    memcpy(&c.position,
                           &tracking.HeadPose.Pose.Position,
                           sizeof(tracking.HeadPose.Pose.Position));
                    memcpy(&g_ctx.lastTrackingPos[controller], &tracking, sizeof(tracking));
                } else {
                    memcpy(&c.position,
                           &g_ctx.lastTrackingPos[controller].HeadPose.Pose.Position,
                           sizeof(g_ctx.lastTrackingPos[controller].HeadPose.Pose.Position));
                }

                memcpy(&c.angularVelocity,
                       &tracking.HeadPose.AngularVelocity,
                       sizeof(tracking.HeadPose.AngularVelocity));

                memcpy(&c.linearVelocity,
                       &tracking.HeadPose.LinearVelocity,
                       sizeof(tracking.HeadPose.LinearVelocity));
            }
        }
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

TrackingInfo getInput() {
    auto java = getOvrJava();

    // vrapi_GetTimeInSeconds doesn't match getTimestampUs
    uint64_t targetTimestampNs = vrapi_GetTimeInSeconds() * 1e9 + alvr_get_prediction_offset_ns();
    auto tracking = vrapi_GetPredictedTracking2(g_ctx.ovrContext, (double) targetTimestampNs / 1e9);

    {
        std::lock_guard<std::mutex> lock(g_ctx.trackingFrameMutex);
        g_ctx.trackingFrameMap.insert({targetTimestampNs, tracking});
        if (g_ctx.trackingFrameMap.size() > MAXIMUM_TRACKING_FRAMES) {
            g_ctx.trackingFrameMap.erase(g_ctx.trackingFrameMap.cbegin());
        }
    }

    TrackingInfo info = {};
    info.targetTimestampNs = targetTimestampNs;

    info.mounted = vrapi_GetSystemStatusInt(&java, VRAPI_SYS_STATUS_MOUNTED);

    memcpy(&info.HeadPose_Pose_Orientation, &tracking.HeadPose.Pose.Orientation, sizeof(ovrQuatf));
    memcpy(&info.HeadPose_Pose_Position, &tracking.HeadPose.Pose.Position, sizeof(ovrVector3f));

    setControllerInfo(&info, g_ctx.clientsidePrediction ? (double) targetTimestampNs / 1e9 : 0.);

    return info;
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

    auto v = glm::mat4();
    for (int x = 0; x < 4; x++) {
        for (int y = 0; y < 4; y++) {
            v[x][y] = tracking->Eye[eye].ViewMatrix.M[y][x];
        }
    }
    v = glm::inverse(v);

    EyeFov fov;
    if (eye == 0) {
        fov = getFov().first;
    } else {
        fov = getFov().second;
    }

    auto input = AlvrEyeInput{};
    input.orientation = TrackingQuat{q.x, q.y, q.z, q.w};
    input.position[0] = v[3][0];
    input.position[1] = v[3][1];
    input.position[2] = v[3][2];
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
        if (abs(new_ipd - g_ctx.lastIpd) > 0.001 ||
            abs(new_fov.first.left - g_ctx.lastFov.left) > 0.001) {
            EyeFov fov[2] = {new_fov.first, new_fov.second};
            alvr_send_views_config(fov, new_ipd);
            g_ctx.lastIpd = new_ipd;
            g_ctx.lastFov = new_fov.first;
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
    getOvrJava(true);

    auto deadline = std::chrono::steady_clock::now();
    auto interval = std::chrono::nanoseconds((uint64_t) (1e9 / g_ctx.refreshRate / 3));

    while (g_ctx.streaming) {
        auto input = getInput();
        alvr_send_input(input);

        deadline += interval;
        std::this_thread::sleep_until(deadline);
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_polygraphene_alvr_OvrActivity_initializeNative(JNIEnv *env, jobject context) {
    env->GetJavaVM(&g_ctx.vm);
    g_ctx.context = env->NewGlobalRef(context);

    alvr_initialize((void *) g_ctx.vm, (void *) g_ctx.context);

    memset(g_ctx.hapticsState, 0, sizeof(g_ctx.hapticsState));

    auto java = getOvrJava(true);
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

    auto java = getOvrJava();
    java.Env->DeleteGlobalRef(g_ctx.context);
}

extern "C" JNIEXPORT void JNICALL Java_com_polygraphene_alvr_OvrActivity_onResumeNative(
        JNIEnv *_env, jobject _context, jobject surface, jobject decoder) {
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

    alvr_resume((void *) decoder,
                width,
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
                                                           jint codec,
                                                           jboolean realTimeDecoder,
                                                           jint oculusFoveationLevel,
                                                           jboolean dynamicOculusFoveation,
                                                           jboolean extraLatency,
                                                           jboolean clientPrediction) {
    auto java = getOvrJava();

    g_ctx.refreshRate = fps;
    g_ctx.clientsidePrediction = clientPrediction;

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

    alvr_start_stream(codec, realTimeDecoder, textureHandles, textureHandlesBuffer[0].size());
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

        const auto it = g_ctx.trackingFrameMap.find(timestampNs);
        if (it != g_ctx.trackingFrameMap.end()) {
            tracking = it->second;
        } else {
            if (!g_ctx.trackingFrameMap.empty())
                tracking = g_ctx.trackingFrameMap.cbegin()->second;
            else
                return;
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