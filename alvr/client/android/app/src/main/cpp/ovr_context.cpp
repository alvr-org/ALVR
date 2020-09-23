#include <unistd.h>
#include <jni.h>
#include <VrApi.h>
#include <VrApi_Types.h>
#include <VrApi_Helpers.h>
#include <VrApi_SystemUtils.h>
#include <VrApi_Input.h>
#include <memory>
#include <map>
#include <chrono>
#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <android/input.h>
#include "render.h"
#include "utils.h"
#include "OVR_Platform.h"
#include "ffr.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>
#include <string>
#include <map>
#include <vector>
#include "asset.h"
#include "sound.h"
#include <inttypes.h>
#include <glm/gtx/euler_angles.hpp>
#include <mutex>

using namespace std;
using namespace gl_render_utils;

const int MAXIMUM_TRACKING_FRAMES = 180;
const uint32_t ovrButton_Unknown1 = 0x01000000;
const chrono::duration<float> MENU_BUTTON_LONG_PRESS_DURATION = 1s;


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

    ALVR_INPUT_MAX = ALVR_INPUT_TRACKPAD_TOUCH,
    ALVR_INPUT_COUNT = ALVR_INPUT_MAX + 1
};

const int alvrHandConfidence_High = (1 << 5);

#define ALVR_BUTTON_FLAG(input) (1ULL << input)

struct TrackingFrame {
    ovrTracking2 tracking;
    uint64_t frameIndex;
    uint64_t fetchTime;
    double displayTime;
};

struct HapticsState {
    uint64_t startUs;
    uint64_t endUs;
    float amplitude;
    float frequency;
    bool fresh;
    bool buffered;
};

class OvrContext {
public:
    ANativeWindow *window = nullptr;
    ovrMobile *Ovr = nullptr;
    ovrJava java;

    jmethodID jsetWebViewVisibility;

    int defaultRenderEyeWidth;
    int defaultRenderEyeHeight;
    float defaultRefreshRate;
    vector<float> refreshRates;

    unique_ptr<Texture> streamTexture;
    unique_ptr<Texture> webViewTexture;

    bool streaming = false;
    bool mShowWebView = false;
    std::function<void(InteractionType, glm::vec2)> mWebViewInteractionCallback;
    TrackingQuat webViewInitHeadOrientation = { 0, 0, 0, 1};
    glm::vec3 webViewInitHeadPosition = glm::vec3(0, -WORLD_VERTICAL_OFFSET, 0);

    uint64_t FrameIndex = 0;

    uint32_t m_GuardianPointCount = 0;
    vector<TrackingVector3> m_GuardianPoints;

    typedef std::map<uint64_t, std::shared_ptr<TrackingFrame> > TRACKING_FRAME_MAP;

    TRACKING_FRAME_MAP trackingFrameMap;
    std::mutex trackingFrameMutex;

    unique_ptr<ovrRenderer> renderer = nullptr;

    unique_ptr<SoundPlayer> soundPlayer = nullptr;

    ovrMicrophoneHandle mMicHandle = nullptr;
    std::vector<int16_t> micBuffer;
    size_t mMicMaxElements;

    // mHapticsState[0]: right hand state
    // mHapticsState[1]: left hand state
    HapticsState mHapticsState[2];

    std::chrono::system_clock::time_point mMenuNotPressedLastInstant;
    bool mMenuLongPressActivated = false;
};

namespace {
    OvrContext g_ctx;
}

OnCreateResult onCreate(void *v_env, void *v_activity, void *v_assetManager) {
    auto *env = (JNIEnv *) v_env;
    auto activity = (jobject) v_activity;
    auto assetManager = (jobject) v_assetManager;

    LOG("Initializing EGL.");

    setAssetManager(env, assetManager);

    g_ctx.java.Env = env;
    env->GetJavaVM(&g_ctx.java.Vm);
    g_ctx.java.ActivityObject = env->NewGlobalRef(activity);

    auto clazz = env->GetObjectClass(activity);
    g_ctx.jsetWebViewVisibility = env->GetMethodID(clazz, "setWebViewVisibility", "(Z)V");
    auto jWebViewInteractionCallback = env->GetMethodID(clazz, "applyWebViewInteractionEvent",
                                                        "(IFF)V");
    env->DeleteLocalRef(clazz);

    g_ctx.mWebViewInteractionCallback = [jWebViewInteractionCallback](InteractionType type,
                                                                      glm::vec2 coord) {
        if (g_ctx.mShowWebView) {
            JNIEnv *env;
            jint res = g_ctx.java.Vm->GetEnv((void **) &env, JNI_VERSION_1_6);
            if (res == JNI_OK) {
                env->CallVoidMethod(g_ctx.java.ActivityObject, jWebViewInteractionCallback,
                                    (int) type, coord.x, coord.y);
            } else {
                LOGE("Failed to get JNI environment for dashboard interaction");
            }
        }
    };

    eglInit();

    const ovrInitParms initParms = vrapi_DefaultInitParms(&g_ctx.java);
    int32_t initResult = vrapi_Initialize(&initParms);
    if (initResult != VRAPI_INITIALIZE_SUCCESS) {
        // If initialization failed, vrapi_* function calls will not be available.
        LOGE("vrapi_Initialize failed");
    }

    ovrPlatformInitializeResult res = ovr_PlatformInitializeAndroid("", activity, env);
    LOGI("ovrPlatformInitializeResult %s", ovrPlatformInitializeResult_ToString(res));

    ovrRequest req;
    req = ovr_User_GetLoggedInUser();
    LOGI("Logged in user is %" PRIu64 "\n", req);

    g_ctx.streamTexture = make_unique<Texture>(true);
    g_ctx.webViewTexture = make_unique<Texture>(true);

    return {(int) g_ctx.streamTexture->GetGLTexture(), (int) g_ctx.webViewTexture->GetGLTexture()};
}

EyeFov getLeftEyeFov() {
    double displayTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, 0);
    ovrTracking2 tracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, displayTime);

    EyeFov fov;

    auto projection = tracking.Eye[0].ProjectionMatrix;
    double a = projection.M[0][0];
    double b = projection.M[1][1];
    double c = projection.M[0][2];
    double d = projection.M[1][2];
    double n = -projection.M[2][3];
    double w1 = 2.0 * n / a;
    double h1 = 2.0 * n / b;
    double w2 = c * w1;
    double h2 = d * h1;

    double maxX = (w1 + w2) / 2.0;
    double minX = w2 - maxX;
    double maxY = (h1 + h2) / 2.0;
    double minY = h2 - maxY;

    double rr = 180 / M_PI;
    LOGI("getFov maxX=%f minX=%f maxY=%f minY=%f a=%f b=%f c=%f d=%f n=%f", maxX, minX, maxY,
         minY, a, b, c, d, n);

    fov.left = (float) (atan(minX / -n) * rr);
    fov.right = (float) (-atan(maxX / -n) * rr);
    fov.top = (float) (atan(minY / -n) * rr);
    fov.bottom = (float) (-atan(maxY / -n) * rr);

    LOGI("getFov[%d](D) r=%f l=%f t=%f b=%f", 0, fov.left, fov.right, fov.top, fov.bottom);

    return fov;
}

void initRendererForLoadingRoom() {
    if (g_ctx.renderer) {
        ovrRenderer_Destroy(g_ctx.renderer.get());
    }

    g_ctx.renderer = std::make_unique<ovrRenderer>();
    ovrRenderer_Create(g_ctx.renderer.get(), g_ctx.defaultRenderEyeWidth,
                       g_ctx.defaultRenderEyeHeight, g_ctx.streamTexture.get(),
                       g_ctx.webViewTexture.get(), g_ctx.mWebViewInteractionCallback, {false});
    ovrRenderer_CreateScene(g_ctx.renderer.get());
}

void setWebViewVisibility(bool visible) {
    g_ctx.mShowWebView = visible;

    if (g_ctx.mShowWebView) {
        auto q = g_ctx.webViewInitHeadOrientation;
        auto glQuat = glm::quat(q.w, q.x, q.y, q.z);
        auto rotEuler = glm::eulerAngles(glQuat);
        float yaw;
        if (abs(rotEuler.x) < M_PI_2) {
            yaw = rotEuler.y;
        } else {
            yaw = M_PI - rotEuler.y;
        }
        auto rotation = glm::eulerAngleY(yaw);
        auto pos = glm::vec4(0, 0, -1.5, 1);
        glm::vec3 position = glm::vec3(rotation * pos) + g_ctx.webViewInitHeadPosition;
        g_ctx.renderer->webViewPanel->SetPoseTransform(position, yaw, 0);
    }

    JNIEnv *env;
    jint res = g_ctx.java.Vm->GetEnv((void **) &env, JNI_VERSION_1_6);
    if (res == JNI_OK) {
        env->CallVoidMethod(g_ctx.java.ActivityObject, g_ctx.jsetWebViewVisibility, visible);
    } else {
        LOGE("Failed to get JNI environment for setting webview visibility");
    }
}

OnResumeResult onResume(void *v_env, void *v_surface) {
    auto *env = (JNIEnv *) v_env;
    auto surface = (jobject) v_surface;

    g_ctx.window = ANativeWindow_fromSurface(env, surface);

    LOGI("Entering VR mode.");

    ovrModeParms parms = vrapi_DefaultModeParms(&g_ctx.java);

    parms.Flags |= VRAPI_MODE_FLAG_RESET_WINDOW_FULLSCREEN;

    parms.Flags |= VRAPI_MODE_FLAG_NATIVE_WINDOW;
    parms.Display = (size_t) egl.Display;
    parms.WindowSurface = (size_t) g_ctx.window;
    parms.ShareContext = (size_t) egl.Context;

    g_ctx.Ovr = vrapi_EnterVrMode(&parms);

    if (g_ctx.Ovr == nullptr) {
        LOGE("Invalid ANativeWindow");
    }

    {
        // set Color Space
        ovrHmdColorDesc colorDesc{};
        colorDesc.ColorSpace = VRAPI_COLORSPACE_REC_2020;
        vrapi_SetClientColorDesc(g_ctx.Ovr, &colorDesc);
    }

    ovrResult result = vrapi_SetDisplayRefreshRate(g_ctx.Ovr, g_ctx.defaultRefreshRate);
    LOGI("vrapi_SetDisplayRefreshRate: Result=%d", result);

    int CpuLevel = 3;
    int GpuLevel = 3;
    vrapi_SetClockLevels(g_ctx.Ovr, CpuLevel, GpuLevel);
    vrapi_SetPerfThread(g_ctx.Ovr, VRAPI_PERF_THREAD_TYPE_MAIN, gettid());

    // On Oculus Quest, without ExtraLatencyMode frames passed to vrapi_SubmitFrame2 are sometimes discarded from VrAPI(?).
    // Which introduces stutter animation.
    // I think the number of discarded frames is shown as Stale in Logcat like following:
    //    I/VrApi: FPS=72,Prd=63ms,Tear=0,Early=0,Stale=8,VSnc=1,Lat=0,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/F/N,Mem=1804MHz,Free=989MB,PSM=0,PLS=0,Temp=36.0C/0.0C,TW=1.90ms,App=2.74ms,GD=0.00ms
    // After enabling ExtraLatencyMode:
    //    I/VrApi: FPS=71,Prd=76ms,Tear=0,Early=66,Stale=0,VSnc=1,Lat=1,Fov=0,CPU4/GPU=3/3,1958/515MHz,OC=FF,TA=0/E0/0,SP=N/N/N,Mem=1804MHz,Free=906MB,PSM=0,PLS=0,Temp=38.0C/0.0C,TW=1.93ms,App=1.46ms,GD=0.00ms
    // We need to set ExtraLatencyMode On to workaround for this issue.
    vrapi_SetExtraLatencyMode(g_ctx.Ovr, VRAPI_EXTRA_LATENCY_MODE_OFF);


    OnResumeResult resultData = {};

    auto ovrDeviceType = vrapi_GetSystemPropertyInt(&g_ctx.java, VRAPI_SYS_PROP_DEVICE_TYPE);
    if (VRAPI_DEVICE_TYPE_OCULUSQUEST_START <= ovrDeviceType &&
        ovrDeviceType <= VRAPI_DEVICE_TYPE_OCULUSQUEST_END) {
        resultData.deviceType = DeviceType::OCULUS_QUEST;
    } else if (ovrDeviceType > VRAPI_DEVICE_TYPE_OCULUSQUEST_END) {
        resultData.deviceType = DeviceType::OCULUS_QUEST_2;
    } else {
        resultData.deviceType = DeviceType::UNKNOWN;
    }

    resultData.recommendedEyeWidth = vrapi_GetSystemPropertyInt(&g_ctx.java,
                                                                VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_WIDTH);
    resultData.recommendedEyeHeight = vrapi_GetSystemPropertyInt(&g_ctx.java,
                                                                 VRAPI_SYS_PROP_SUGGESTED_EYE_TEXTURE_HEIGHT);
    g_ctx.defaultRenderEyeWidth = resultData.recommendedEyeWidth;
    g_ctx.defaultRenderEyeHeight = resultData.recommendedEyeHeight;

    resultData.refreshRatesCount = vrapi_GetSystemPropertyInt(&g_ctx.java,
                                                              VRAPI_SYS_PROP_NUM_SUPPORTED_DISPLAY_REFRESH_RATES);

    g_ctx.refreshRates = vector<float>(resultData.refreshRatesCount);
    vrapi_GetSystemPropertyFloatArray(&g_ctx.java, VRAPI_SYS_PROP_SUPPORTED_DISPLAY_REFRESH_RATES,
                                      &g_ctx.refreshRates[0], resultData.refreshRatesCount);
    resultData.refreshRates = &g_ctx.refreshRates[0];

    // choose highest supported refresh rate as default
    g_ctx.defaultRefreshRate = 0;
    for (int i = 0; i < resultData.refreshRatesCount; i++) {
        if (g_ctx.refreshRates[i] > g_ctx.defaultRefreshRate) {
            g_ctx.defaultRefreshRate = g_ctx.refreshRates[i];
        }
    }
    resultData.defaultRefreshRate = g_ctx.defaultRefreshRate;

    resultData.leftEyeFov = getLeftEyeFov();

    // default fov cannot be queried now (?) so Rust chooses some constants based on deviceType.

    initRendererForLoadingRoom();

    setWebViewVisibility(true);

    return resultData;
}

void onStreamStart(OnStreamStartParams params) {
    vrapi_SetExtraLatencyMode(g_ctx.Ovr, VRAPI_EXTRA_LATENCY_MODE_ON);

    if (g_ctx.renderer) {
        ovrRenderer_Destroy(g_ctx.renderer.get());
    }

    g_ctx.renderer.reset(new ovrRenderer());
    ovrRenderer_Create(g_ctx.renderer.get(), params.eyeWidth, params.eyeHeight,
                       g_ctx.streamTexture.get(), g_ctx.webViewTexture.get(),
                       g_ctx.mWebViewInteractionCallback,
                       {params.foveationEnabled, (uint32_t) params.eyeWidth,
                        (uint32_t) params.eyeHeight,
                        params.leftEyeFov, params.foveationStrength, params.foveationShape,
                        params.foveationVerticalOffset});
    ovrRenderer_CreateScene(g_ctx.renderer.get());

    if (params.enableGameAudio) {
        g_ctx.soundPlayer = std::make_unique<SoundPlayer>();
        if (g_ctx.soundPlayer->initialize() != 0) {
            LOGE("Failed on SoundPlayer initialize.");
            g_ctx.soundPlayer.reset();
        }
    }

    if (params.enableMicrophone) {
        g_ctx.mMicHandle = ovr_Microphone_Create();
        g_ctx.mMicMaxElements = ovr_Microphone_GetOutputBufferMaxSize(g_ctx.mMicHandle);
        LOGI("Mic_maxElements %zu", g_ctx.mMicMaxElements);
        if (g_ctx.micBuffer.size() != g_ctx.mMicMaxElements) {
            g_ctx.micBuffer.assign(g_ctx.mMicMaxElements, 0);
        }
        ovr_Microphone_Start(g_ctx.mMicHandle);
    }

    ovrResult result = vrapi_SetDisplayRefreshRate(g_ctx.Ovr, params.refreshRate);
    LOGI("vrapi_SetDisplayRefreshRate: Result=%d", result);

    memset(g_ctx.mHapticsState, 0, sizeof(g_ctx.mHapticsState));

    setWebViewVisibility(false);
}

void finishHapticsBuffer(ovrDeviceID DeviceID) {
    uint8_t hapticBuffer[1] = {0};
    ovrHapticBuffer buffer;
    buffer.BufferTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
    buffer.HapticBuffer = &hapticBuffer[0];
    buffer.NumSamples = 1;
    buffer.Terminated = true;

    auto result = vrapi_SetHapticVibrationBuffer(g_ctx.Ovr, DeviceID, &buffer);
    if (result != ovrSuccess) {
        LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
    }
}

void updateHapticsState() {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(g_ctx.Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        ovrInputTrackedRemoteCapabilities remoteCapabilities;

        remoteCapabilities.Header = curCaps;
        result = vrapi_GetInputDeviceCapabilities(g_ctx.Ovr, &remoteCapabilities.Header);
        if (result != ovrSuccess) {
            continue;
        }

        int curHandIndex = (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand)
                           ? 1 : 0;
        auto &s = g_ctx.mHapticsState[curHandIndex];

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

            // First, call with buffer.Terminated = false and when haptics is no more needed call with buffer.Terminated = true (to stop haptics?).
            LOG("Send haptic buffer. HapticSamplesMax=%d HapticSampleDurationMS=%d",
                remoteCapabilities.HapticSamplesMax, remoteCapabilities.HapticSampleDurationMS);

            uint32_t requiredHapticsBuffer = static_cast<uint32_t >((s.endUs - currentUs) /
                                                                    remoteCapabilities.HapticSampleDurationMS *
                                                                    1000);

            std::vector<uint8_t> hapticBuffer(remoteCapabilities.HapticSamplesMax);
            ovrHapticBuffer buffer;
            buffer.BufferTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
            buffer.HapticBuffer = &hapticBuffer[0];
            buffer.NumSamples = std::min(remoteCapabilities.HapticSamplesMax,
                                         requiredHapticsBuffer);
            buffer.Terminated = false;

            for (int i = 0; i < remoteCapabilities.HapticSamplesMax; i++) {
                float current = ((currentUs - s.startUs) / 1000000.0f) +
                                (remoteCapabilities.HapticSampleDurationMS * i) / 1000.0f;
                float intensity =
                        (sinf(static_cast<float>(current * M_PI * 2 * s.frequency)) + 1.0f) * 0.5f *
                        s.amplitude;
                if (intensity < 0) {
                    intensity = 0;
                } else if (intensity > 1.0) {
                    intensity = 1.0;
                }
                hapticBuffer[i] = static_cast<uint8_t>(255 * intensity);
            }

            result = vrapi_SetHapticVibrationBuffer(g_ctx.Ovr, curCaps.DeviceID, &buffer);
            if (result != ovrSuccess) {
                LOGI("vrapi_SetHapticVibrationBuffer: Failed. result=%d", result);
            }
            s.buffered = true;
        } else if (remoteCapabilities.ControllerCapabilities &
                   ovrControllerCaps_HasSimpleHapticVibration) {
            LOG("Send simple haptic. amplitude=%f", s.amplitude);
            vrapi_SetHapticVibrationSimple(g_ctx.Ovr, curCaps.DeviceID, s.amplitude);
        }
    }
}

void render(bool streaming, long long renderedFrameIndex) {
    g_ctx.streaming = streaming;
    if (streaming) {
        FrameLog(renderedFrameIndex, "Got frame for render.");

        updateHapticsState();

        uint64_t oldestFrame = 0;
        uint64_t mostRecentFrame = 0;
        std::shared_ptr<TrackingFrame> frame;
        {
            std::lock_guard<decltype(OvrContext::trackingFrameMutex)> lock(
                    g_ctx.trackingFrameMutex);

            if (!g_ctx.trackingFrameMap.empty()) {
                oldestFrame = g_ctx.trackingFrameMap.cbegin()->second->frameIndex;
                mostRecentFrame = g_ctx.trackingFrameMap.crbegin()->second->frameIndex;
            }

            const auto it = g_ctx.trackingFrameMap.find(renderedFrameIndex);
            if (it != g_ctx.trackingFrameMap.end()) {
                frame = it->second;
            } else {
                // No matching tracking info. Too old frame.
                LOG("Too old frame has arrived. Instead, we use most old tracking data in trackingFrameMap."
                    "FrameIndex=%lld trackingFrameMap=(%lu - %lu)",
                    renderedFrameIndex, oldestFrame, mostRecentFrame);
                if (!g_ctx.trackingFrameMap.empty())
                    frame = g_ctx.trackingFrameMap.cbegin()->second;
                else
                    return;
            }
        }

        FrameLog(renderedFrameIndex, "Frame latency is %lu us.",
                 getTimestampUs() - frame->fetchTime);

        // Render eye images and setup the primary layer using ovrTracking2.
        const ovrLayerProjection2 worldLayer =
                ovrRenderer_RenderFrame(g_ctx.renderer.get(), &frame->tracking, true,
                                        g_ctx.mShowWebView);

        const ovrLayerHeader2 *layers2[] =
                {
                        &worldLayer.Header
                };

        ovrSubmitFrameDescription2 frameDesc = {};
        frameDesc.Flags = 0;
        frameDesc.SwapInterval = 1;
        frameDesc.FrameIndex = renderedFrameIndex;
        frameDesc.DisplayTime = 0.0;
        frameDesc.LayerCount = 1;
        frameDesc.Layers = layers2;

        ovrResult res = vrapi_SubmitFrame2(g_ctx.Ovr, &frameDesc);

        FrameLog(renderedFrameIndex, "vrapi_SubmitFrame2 Orientation=(%f, %f, %f, %f)",
                 frame->tracking.HeadPose.Pose.Orientation.x,
                 frame->tracking.HeadPose.Pose.Orientation.y,
                 frame->tracking.HeadPose.Pose.Orientation.z,
                 frame->tracking.HeadPose.Pose.Orientation.w
        );
    } else {

        double DisplayTime = GetTimeInSeconds();

        // Show a loading icon.
        g_ctx.FrameIndex++;

        double displayTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
        ovrTracking2 headTracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, displayTime);

        const ovrLayerProjection2 worldLayer = ovrRenderer_RenderFrame(g_ctx.renderer.get(),
                                                                       &headTracking, false,
                                                                       g_ctx.mShowWebView);

        const ovrLayerHeader2 *layers[] =
                {
                        &worldLayer.Header
                };


        ovrSubmitFrameDescription2 frameDesc = {};
        frameDesc.Flags = 0;
        frameDesc.SwapInterval = 1;
        frameDesc.FrameIndex = g_ctx.FrameIndex;
        frameDesc.DisplayTime = DisplayTime;
        frameDesc.LayerCount = 1;
        frameDesc.Layers = layers;

        vrapi_SubmitFrame2(g_ctx.Ovr, &frameDesc);
    }
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
    } else {
        // GearVR or Oculus Go Controller
        if (remoteInputState->Buttons & ovrButton_A) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRIGGER_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Enter) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRACKPAD_CLICK);
        }
        if (remoteInputState->Buttons & ovrButton_Back) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_BACK_CLICK);
        }
        if (remoteInputState->TrackpadStatus) {
            buttons |= ALVR_BUTTON_FLAG(ALVR_INPUT_TRACKPAD_TOUCH);
        }
    }
    return buttons;
}

void setControllerInfo(TrackingInfo *packet, double displayTime, GUIInput *guiInput) {
    ovrInputCapabilityHeader curCaps;
    ovrResult result;
    int controller = 0;

    for (uint32_t deviceIndex = 0;
         vrapi_EnumerateInputDevices(g_ctx.Ovr, deviceIndex, &curCaps) >= 0; deviceIndex++) {
        LOG("Device %d: Type=%d ID=%d", deviceIndex, curCaps.Type, curCaps.DeviceID);
        if (curCaps.Type == ovrControllerType_Hand) {  //A3
            if (g_ctx.streaming) {
                setWebViewVisibility(false);
            }

            // Oculus Quest Hand Tracking
            if (controller >= 2) {
                LOG("Device %d: Ignore.", deviceIndex);
                continue;
            }

            auto &c = packet->controller[controller];

            ovrInputHandCapabilities handCapabilities;
            ovrInputStateHand inputStateHand;
            handCapabilities.Header = curCaps;

            result = vrapi_GetInputDeviceCapabilities(g_ctx.Ovr, &handCapabilities.Header);

            if (result != ovrSuccess) {
                continue;
            }

            if ((handCapabilities.HandCapabilities & ovrHandCaps_LeftHand) != 0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND;
            }
            inputStateHand.Header.ControllerType = handCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(g_ctx.Ovr, handCapabilities.Header.DeviceID,
                                                &inputStateHand.Header);
            if (result != ovrSuccess) {
                continue;
            }

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_ENABLE;

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_HAND;

            c.inputStateStatus = inputStateHand.InputStateStatus;
            memcpy(&c.fingerPinchStrengths, &inputStateHand.PinchStrength,
                   sizeof(c.fingerPinchStrengths));

            memcpy(&c.orientation, &inputStateHand.PointerPose.Orientation,
                   sizeof(inputStateHand.PointerPose.Orientation));
            memcpy(&c.position, &inputStateHand.PointerPose.Position,
                   sizeof(inputStateHand.PointerPose.Position));

            ovrHandedness handedness =
                    handCapabilities.HandCapabilities & ovrHandCaps_LeftHand ? VRAPI_HAND_LEFT
                                                                             : VRAPI_HAND_RIGHT;
            ovrHandSkeleton handSkeleton;
            handSkeleton.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandSkeleton(g_ctx.Ovr, handedness, &handSkeleton.Header) != ovrSuccess) {
                LOG("VrHands - failed to get hand skeleton");
            } else {
                for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                    memcpy(&c.bonePositionsBase[i], &handSkeleton.BonePoses[i].Position,
                           sizeof(handSkeleton.BonePoses[i].Position));
                }
                //for(int i=0;i<ovrHandBone_MaxSkinnable;i++) {
                //    memcpy(&c.boneRotationsBase[i], &handSkeleton.BonePoses[i].Orientation, sizeof(handSkeleton.BonePoses[i].Orientation));
                //}
            }

            ovrHandPose handPose;
            handPose.Header.Version = ovrHandVersion_1;
            if (vrapi_GetHandPose(g_ctx.Ovr, handCapabilities.Header.DeviceID, 0,
                                  &handPose.Header) !=
                ovrSuccess) {
                LOG("VrHands - failed to get hand pose");
            } else {
                if (handPose.HandConfidence == ovrConfidence_HIGH) {
                    c.handFingerConfidences |= alvrHandConfidence_High;
                }
                for (int i = 0; i < ovrHandFinger_Max; i++) {
                    c.handFingerConfidences |=
                            handPose.FingerConfidences[i] == ovrConfidence_HIGH ? (1 << i) : 0;
                }

                memcpy(&c.boneRootOrientation, &handPose.RootPose.Orientation,
                       sizeof(handPose.RootPose.Orientation));
                memcpy(&c.boneRootPosition, &handPose.RootPose.Position,
                       sizeof(handPose.RootPose.Position));
                for (int i = 0; i < ovrHandBone_MaxSkinnable; i++) {
                    memcpy(&c.boneRotations[i], &handPose.BoneRotations[i],
                           sizeof(handPose.BoneRotations[i]));
                }
            }
            controller++;
        }
        if (curCaps.Type == ovrControllerType_TrackedRemote) {
            // Gear VR / Oculus Go 3DoF Controller / Oculus Quest Touch Controller
            if (controller >= 2) {
                LOG("Device %d: Ignore.", deviceIndex);
                continue;
            }

            auto &c = packet->controller[controller];

            ovrInputTrackedRemoteCapabilities remoteCapabilities;
            ovrInputStateTrackedRemote remoteInputState;

            remoteCapabilities.Header = curCaps;
            result = vrapi_GetInputDeviceCapabilities(g_ctx.Ovr, &remoteCapabilities.Header);
            if (result != ovrSuccess) {
                continue;
            }
            remoteInputState.Header.ControllerType = remoteCapabilities.Header.Type;

            result = vrapi_GetCurrentInputState(g_ctx.Ovr, remoteCapabilities.Header.DeviceID,
                                                &remoteInputState.Header);
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
                remoteInputState.Buttons, remoteInputState.Touches,
                remoteInputState.JoystickNoDeadZone.x, remoteInputState.JoystickNoDeadZone.y,
                remoteInputState.IndexTrigger, remoteInputState.GripTrigger);

            c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_ENABLE;

            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_LeftHand) != 0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND;

                if (remoteInputState.Buttons & ovrButton_Enter) {
                    if (!g_ctx.mMenuLongPressActivated && std::chrono::system_clock::now()
                                                          - g_ctx.mMenuNotPressedLastInstant >
                                                          MENU_BUTTON_LONG_PRESS_DURATION) {
                        g_ctx.webViewInitHeadOrientation = packet->HeadPose_Pose_Orientation;
                        g_ctx.webViewInitHeadPosition = guiInput->headPosition;
                        setWebViewVisibility(!g_ctx.mShowWebView);
                        g_ctx.mMenuLongPressActivated = true;
                    }
                } else {
                    g_ctx.mMenuNotPressedLastInstant = std::chrono::system_clock::now();
                    g_ctx.mMenuLongPressActivated = false;
                }
            }

            if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_ModelGearVR) !=
                0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_GEARVR;
            } else if (
                    (remoteCapabilities.ControllerCapabilities & ovrControllerCaps_ModelOculusGo) !=
                    0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_GO;
            } else if ((remoteCapabilities.ControllerCapabilities &
                        ovrControllerCaps_ModelOculusTouch) !=
                       0) {
                c.flags |= TrackingInfo::Controller::FLAG_CONTROLLER_OCULUS_QUEST;
            }

            if (g_ctx.mShowWebView) {
                guiInput->actionButtonsDown[controller] =
                        remoteInputState.Buttons & (ovrButton_A | ovrButton_X | ovrButton_Trigger);
            } else {
                c.buttons = mapButtons(&remoteCapabilities, &remoteInputState);

                if ((remoteCapabilities.ControllerCapabilities & ovrControllerCaps_HasJoystick) !=
                    0) {
                    c.trackpadPosition.x = remoteInputState.JoystickNoDeadZone.x;
                    c.trackpadPosition.y = remoteInputState.JoystickNoDeadZone.y;
                } else {
                    // Normalize to -1.0 - +1.0 for OpenVR Input. y-asix should be reversed.
                    c.trackpadPosition.x =
                            remoteInputState.TrackpadPosition.x / remoteCapabilities.TrackpadMaxX *
                            2.0f - 1.0f;
                    c.trackpadPosition.y =
                            remoteInputState.TrackpadPosition.y / remoteCapabilities.TrackpadMaxY *
                            2.0f - 1.0f;
                    c.trackpadPosition.y = -c.trackpadPosition.y;
                }
                c.triggerValue = remoteInputState.IndexTrigger;
                c.gripValue = remoteInputState.GripTrigger;

                c.batteryPercentRemaining = remoteInputState.BatteryPercentRemaining;
            }

            ovrTracking tracking;
            if (vrapi_GetInputTrackingState(g_ctx.Ovr, remoteCapabilities.Header.DeviceID,
                                            0, &tracking) != ovrSuccess) {
                LOG("vrapi_GetInputTrackingState failed. Device was disconnected?");
            } else {

                memcpy(&c.orientation,
                       &tracking.HeadPose.Pose.Orientation,
                       sizeof(tracking.HeadPose.Pose.Orientation));

                memcpy(&c.position,
                       &tracking.HeadPose.Pose.Position,
                       sizeof(tracking.HeadPose.Pose.Position));

                memcpy(&c.angularVelocity,
                       &tracking.HeadPose.AngularVelocity,
                       sizeof(tracking.HeadPose.AngularVelocity));

                memcpy(&c.linearVelocity,
                       &tracking.HeadPose.LinearVelocity,
                       sizeof(tracking.HeadPose.LinearVelocity));

                memcpy(&c.angularAcceleration,
                       &tracking.HeadPose.AngularAcceleration,
                       sizeof(tracking.HeadPose.AngularAcceleration));

                memcpy(&c.linearAcceleration,
                       &tracking.HeadPose.LinearAcceleration,
                       sizeof(tracking.HeadPose.LinearAcceleration));

                auto pos = tracking.HeadPose.Pose.Position;
                guiInput->controllersPosition[controller] = glm::vec3(pos.x,
                                                                      pos.y - WORLD_VERTICAL_OFFSET,
                                                                      pos.z);
                auto rot = tracking.HeadPose.Pose.Orientation;
                guiInput->controllersRotation[controller] = glm::quat(rot.w, rot.x, rot.y, rot.z);
            }
            controller++;
        }
    }
}

// Called TrackingThread. So, we can't use this->env.
TrackingInfo getTrackingInfo() {
    std::shared_ptr<TrackingFrame> frame(new TrackingFrame());

    g_ctx.FrameIndex++;

    frame->frameIndex = g_ctx.FrameIndex;
    frame->fetchTime = getTimestampUs();

    frame->displayTime = vrapi_GetPredictedDisplayTime(g_ctx.Ovr, g_ctx.FrameIndex);
    frame->tracking = vrapi_GetPredictedTracking2(g_ctx.Ovr, frame->displayTime);

    /*LOGI("MVP %llu: \nL-V:\n%s\nL-P:\n%s\nR-V:\n%s\nR-P:\n%s", FrameIndex,
         DumpMatrix(&frame->tracking.Eye[0].ViewMatrix).c_str(),
         DumpMatrix(&frame->tracking.Eye[0].ProjectionMatrix).c_str(),
         DumpMatrix(&frame->tracking.Eye[1].ViewMatrix).c_str(),
         DumpMatrix(&frame->tracking.Eye[1].ProjectionMatrix).c_str()
         );*/

    {
        std::lock_guard<decltype(OvrContext::trackingFrameMutex)> lock(g_ctx.trackingFrameMutex);
        g_ctx.trackingFrameMap.insert(
                std::pair<uint64_t, std::shared_ptr<TrackingFrame> >(g_ctx.FrameIndex, frame));
        if (g_ctx.trackingFrameMap.size() > MAXIMUM_TRACKING_FRAMES) {
            g_ctx.trackingFrameMap.erase(g_ctx.trackingFrameMap.cbegin());
        }
    }

    TrackingInfo info = {};

    info.clientTime = getTimestampUs();
    info.FrameIndex = g_ctx.FrameIndex;
    info.predictedDisplayTime = frame->displayTime;

    memcpy(&info.HeadPose_Pose_Orientation, &frame->tracking.HeadPose.Pose.Orientation,
           sizeof(ovrQuatf));
    memcpy(&info.HeadPose_Pose_Position, &frame->tracking.HeadPose.Pose.Position,
           sizeof(ovrVector3f));

    GUIInput guiInput = {};
    auto pos = frame->tracking.HeadPose.Pose.Position;
    guiInput.headPosition = glm::vec3(pos.x, pos.y - WORLD_VERTICAL_OFFSET, pos.z);

    setControllerInfo(&info, frame->displayTime, &guiInput);

    g_ctx.renderer->gui->Update(guiInput);

    FrameLog(g_ctx.FrameIndex, "Sending tracking info.");

    return info;
}

void enqueueAudio(unsigned char *buf, int len) {
    if (g_ctx.soundPlayer) {
        g_ctx.soundPlayer->putData(buf, len);
    }
}

// Called TrackingThread. So, we can't use this->env.
MicAudioFrame getMicData() {
    size_t outputBufferNumElements = ovr_Microphone_GetPCM(g_ctx.mMicHandle, &g_ctx.micBuffer[0],
                                                           g_ctx.mMicMaxElements);

    return {&g_ctx.micBuffer[0], (long long) outputBufferNumElements};
}

void
onHapticsFeedback(unsigned long long startTime, float amplitude, float duration, float frequency,
                  int hand) {
    LOGI("OvrContext::onHapticsFeedback: processing haptics. %llu %f %f %f, %d", startTime,
         amplitude, duration, frequency, hand);

    int curHandIndex = (hand == 0) ? 0 : 1;
    auto &s = g_ctx.mHapticsState[curHandIndex];
    s.startUs = startTime;
    s.endUs = static_cast<uint64_t>(duration * 1000000);
    s.amplitude = amplitude;
    s.frequency = frequency;
    s.fresh = true;
    s.buffered = false;
}

// Called TrackingThread. So, we can't use this->env.
GuardianData getGuardianInfo() {
    vrapi_GetBoundaryGeometry(g_ctx.Ovr, 0, &g_ctx.m_GuardianPointCount, nullptr);

    if (g_ctx.m_GuardianPointCount > 0) {
        g_ctx.m_GuardianPoints = vector<TrackingVector3>(g_ctx.m_GuardianPointCount);
        vector<ovrVector3f> temp_points(g_ctx.m_GuardianPointCount);

        vrapi_GetBoundaryGeometry(g_ctx.Ovr, g_ctx.m_GuardianPointCount,
                                  &g_ctx.m_GuardianPointCount, &temp_points[0]);

        memcpy(&g_ctx.m_GuardianPoints[0], &temp_points[0],
               sizeof(TrackingVector3) * g_ctx.m_GuardianPointCount);
    }

    GuardianData packet = {};
    packet.totalPointCount = g_ctx.m_GuardianPointCount;
    packet.points = &g_ctx.m_GuardianPoints[0];

    ovrPosef spacePose = vrapi_LocateTrackingSpace(g_ctx.Ovr, VRAPI_TRACKING_SPACE_LOCAL_FLOOR);
    memcpy(&packet.standingPosRotation, &spacePose.Orientation, sizeof(TrackingQuat));
    memcpy(&packet.standingPosPosition, &spacePose.Position, sizeof(TrackingVector3));

    ovrVector3f bboxScale;
    vrapi_GetBoundaryOrientedBoundingBox(g_ctx.Ovr, &spacePose /* dummy variable */, &bboxScale);
    packet.playAreaSize.x = 2.0f * bboxScale.x;
    packet.playAreaSize.y = 2.0f * bboxScale.z;

    return packet;
}

void onStreamStop() {
    setWebViewVisibility(true);

    ovrResult result = vrapi_SetDisplayRefreshRate(g_ctx.Ovr, g_ctx.defaultRefreshRate);
    LOGI("vrapi_SetDisplayRefreshRate: Result=%d", result);

    vrapi_SetExtraLatencyMode(g_ctx.Ovr, VRAPI_EXTRA_LATENCY_MODE_OFF);

    if (g_ctx.mMicHandle) {
        ovr_Microphone_Stop(g_ctx.mMicHandle);
        ovr_Microphone_Destroy(g_ctx.mMicHandle);
        g_ctx.mMicHandle = nullptr;
    }

    g_ctx.soundPlayer.reset();

    initRendererForLoadingRoom();
}

void onPause() {
    ovrRenderer_Destroy(g_ctx.renderer.get());
    g_ctx.renderer.reset();

    LOGI("Leaving VR mode.");
    vrapi_LeaveVrMode(g_ctx.Ovr);

    ANativeWindow_release(g_ctx.window);
}

void onDestroy(void *v_env) {
    auto *env = (JNIEnv *) v_env;

    LOG("Destroying EGL.");

    vrapi_Shutdown();

    eglDestroy();

    env->DeleteGlobalRef(g_ctx.java.ActivityObject);
}